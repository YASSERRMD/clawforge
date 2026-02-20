/// Google Chat adapter — receives webhook events from Google Chat Spaces
/// and sends via the Google Chat Incoming Webhook API.
use anyhow::Result;
use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid;

use clawforge_core::{AuditEventPayload, Event, EventKind, Message};

use crate::ChannelAdapter;

pub struct GoogleChatConfig {
    pub incoming_webhook_url: Option<String>,
    pub webhook_path: String,
}

pub struct GoogleChatAdapter {
    config: GoogleChatConfig,
    supervisor_tx: mpsc::Sender<Message>,
    http: Client,
}

impl GoogleChatAdapter {
    pub fn new(config: GoogleChatConfig, supervisor_tx: mpsc::Sender<Message>) -> Self {
        Self { config, supervisor_tx, http: Client::new() }
    }

    pub async fn send_message(&self, text: &str) -> Result<()> {
        if let Some(url) = &self.config.incoming_webhook_url {
            self.http.post(url)
                .json(&serde_json::json!({ "text": text }))
                .send()
                .await?
                .error_for_status()?;
        }
        Ok(())
    }
}

#[derive(Clone)]
struct AppState { supervisor_tx: mpsc::Sender<Message> }

#[derive(Deserialize)]
struct ChatEvent {
    #[serde(rename = "type")]
    event_type: Option<String>,
    message: Option<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatMessage {
    text: Option<String>,
    sender: Option<ChatSender>,
    space: Option<ChatSpace>,
}

#[derive(Deserialize)]
struct ChatSender { #[serde(rename = "displayName")] display_name: Option<String> }

#[derive(Deserialize)]
struct ChatSpace { #[serde(rename = "displayName")] display_name: Option<String> }

async fn event_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChatEvent>,
) -> impl IntoResponse {
    if payload.event_type.as_deref() == Some("REMOVED_FROM_SPACE") {
        return StatusCode::OK.into_response();
    }
    if let Some(msg) = payload.message {
        let text = msg.text.unwrap_or_default();
        let sender = msg.sender.and_then(|s| s.display_name).unwrap_or_default();
        let space = msg.space.and_then(|s| s.display_name).unwrap_or_default();
        info!("[GoogleChat] {} in {}: {}", sender, space, text);
        let event = Event::new(
            Uuid::new_v4(), Uuid::new_v4(), EventKind::RunStarted,
            serde_json::json!({ "source": "googlechat", "sender": sender, "space": space, "text": text }),
        );
        let _ = state.supervisor_tx.send(Message::AuditEvent(AuditEventPayload { event })).await;
    }
    (StatusCode::OK, serde_json::json!({ "text": "✅" }).to_string()).into_response()
}

#[async_trait]
impl ChannelAdapter for GoogleChatAdapter {
    fn name(&self) -> &str { "googlechat" }
    fn build_router(&self) -> Router {
        let state = AppState { supervisor_tx: self.supervisor_tx.clone() };
        Router::new().route(&self.config.webhook_path, post(event_handler)).with_state(state)
    }
    async fn start(&self, _supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        info!("[GoogleChat] Adapter ready at {}", self.config.webhook_path);
        Ok(())
    }
}
