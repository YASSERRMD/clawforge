use anyhow::{Context, Result};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use crate::ChannelAdapter;
use clawforge_core::{
    Message, EventKind, Event, AuditEventPayload
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Clone)]
pub struct BlueBubblesConfig {
    pub server_url: String,
    pub password: String,
    pub webhook_path: String,
}

#[derive(Clone)]
struct AppState {
    config: BlueBubblesConfig,
    supervisor_tx: mpsc::Sender<Message>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BlueBubblesWebhookPayload {
    #[serde(rename = "type")]
    event_type: String,
    data: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BlueBubblesMessageData {
    guid: String,
    text: Option<String>,
    handle: Option<BlueBubblesHandle>,
    chats: Option<Vec<BlueBubblesChat>>,
    is_from_me: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BlueBubblesHandle {
    address: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct BlueBubblesChat {
    guid: String,
}

#[derive(Deserialize)]
struct WebhookQuery {
    password: Option<String>,
}

pub struct BlueBubblesAdapter {
    config: BlueBubblesConfig,
    supervisor_tx: mpsc::Sender<Message>,
    http_client: Client,
}

impl BlueBubblesAdapter {
    pub fn new(config: BlueBubblesConfig, supervisor_tx: mpsc::Sender<Message>) -> Self {
        Self {
            config,
            supervisor_tx,
            http_client: Client::new(),
        }
    }

    pub fn build_router(&self) -> Router {
        let state = AppState {
            config: self.config.clone(),
            supervisor_tx: self.supervisor_tx.clone(),
        };

        Router::new()
            .route(&self.config.webhook_path, post(handle_webhook))
            .with_state(state)
    }

    async fn resolve_chat_guid(&self, target: &str) -> Result<String> {
        if target.contains(";") {
            Ok(target.to_string())
        } else {
            Ok(format!("iMessage;-;{}", target))
        }
    }
}

async fn handle_webhook(
    State(state): State<AppState>,
    Query(query): Query<WebhookQuery>,
    Json(payload): Json<BlueBubblesWebhookPayload>,
) -> impl IntoResponse {
    let passes_auth = query
        .password
        .map(|pwd| pwd == state.config.password)
        .unwrap_or(false);

    if !passes_auth {
        error!("[BlueBubbles] Unauthorized webhook request");
        return (StatusCode::UNAUTHORIZED, "Unauthorized");
    }

    if payload.event_type != "new-message" {
        return (StatusCode::OK, "Ignored event type");
    }

    let Some(data) = payload.data else {
        return (StatusCode::OK, "No data in payload");
    };

    let Ok(msg_data) = serde_json::from_value::<BlueBubblesMessageData>(data) else {
        error!("[BlueBubbles] Failed to parse message data from webhook");
        return (StatusCode::BAD_REQUEST, "Invalid format");
    };

    if msg_data.is_from_me.unwrap_or(false) {
        return (StatusCode::OK, "Ignored self-send");
    }

    let Some(text) = msg_data.text else {
        return (StatusCode::OK, "Ignored empty text");
    };

    let sender_address = msg_data
        .handle
        .map(|h| h.address)
        .unwrap_or_else(|| "unknown".to_string());
    
    let chat_id = msg_data
        .chats
        .and_then(|chats| chats.first().map(|c| c.guid.clone()))
        .unwrap_or_else(|| "unknown_chat".to_string());

    info!(
        "[BlueBubbles] Received message from {} in chat {}: {}",
        sender_address, chat_id, text
    );

    let event = Event::new(
        Uuid::new_v4(), // Dummy Run ID
        Uuid::new_v4(), // Dummy Agent ID
        EventKind::RunStarted,
        serde_json::json!({
            "source": "bluebubbles",
            "chat_id": chat_id,
            "message_id": msg_data.guid,
            "text": text
        })
    );

    let _ = state.supervisor_tx.send(Message::AuditEvent(AuditEventPayload { event })).await;

    (StatusCode::OK, "OK")
}

#[async_trait::async_trait]
impl ChannelAdapter for BlueBubblesAdapter {
    async fn start(&self, supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        info!("[BlueBubbles] Adapter started natively via webhook. outbound listener not strictly using this `start` method right now.");
        // We actually want to listen on the webhook for inbound. Outbound isn't implemented as a dedicated background loop in channels unless doing websocket polling.
        Ok(())
    }

    async fn send_message(&self, chat_id: &str, text: &str) -> anyhow::Result<()> {
        let chat_guid = self.resolve_chat_guid(chat_id).await?;
        let url = format!(
            "{}/api/v1/message/text?password={}",
            self.config.server_url.trim_end_matches('/'),
            self.config.password
        );

        let payload = serde_json::json!({
            "chatGuid": chat_guid,
            "tempGuid": Uuid::new_v4().to_string(),
            "message": text,
        });

        match self.http_client.post(&url).json(&payload).send().await {
            Ok(res) if res.status().is_success() => {
                info!("[BlueBubbles] Sent message to {}", chat_guid);
            }
            Ok(res) => {
                error!(
                    "[BlueBubbles] Failed to send to {}: {}",
                    chat_guid,
                    res.text().await.unwrap_or_default()
                );
            }
            Err(e) => {
                error!("[BlueBubbles] HTTP error sending to {}: {}", chat_guid, e);
            }
        }
        Ok(())
    }
}
