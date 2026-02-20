/// LINE adapter — receives webhook events from LINE Messaging API.
/// Sends via the LINE Reply API.
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

pub struct LineConfig {
    pub channel_secret: String,
    pub channel_access_token: String,
    pub webhook_path: String,
}

pub struct LineAdapter {
    config: LineConfig,
    supervisor_tx: mpsc::Sender<Message>,
    http: Client,
}

impl LineAdapter {
    pub fn new(config: LineConfig, supervisor_tx: mpsc::Sender<Message>) -> Self {
        Self { config, supervisor_tx, http: Client::new() }
    }

    pub async fn reply(&self, reply_token: &str, text: &str) -> Result<()> {
        self.http.post("https://api.line.me/v2/bot/message/reply")
            .bearer_auth(&self.config.channel_access_token)
            .json(&serde_json::json!({
                "replyToken": reply_token,
                "messages": [{ "type": "text", "text": text }]
            }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[derive(Clone)]
struct AppState { supervisor_tx: mpsc::Sender<Message> }

#[derive(Deserialize)]
struct LineWebhook {
    events: Vec<LineEvent>,
}

#[derive(Deserialize)]
struct LineEvent {
    #[serde(rename = "type")]
    event_type: String,
    message: Option<LineMessage>,
    source: Option<LineSource>,
    #[serde(rename = "replyToken")]
    reply_token: Option<String>,
}

#[derive(Deserialize)]
struct LineMessage { #[serde(rename = "type")] kind: String, text: Option<String> }

#[derive(Deserialize)]
struct LineSource { #[serde(rename = "userId")] user_id: Option<String> }

async fn webhook_handler(
    State(state): State<AppState>,
    Json(payload): Json<LineWebhook>,
) -> impl IntoResponse {
    for ev in payload.events {
        if ev.event_type != "message" { continue; }
        if let Some(msg) = &ev.message {
            if msg.kind != "text" { continue; }
            let text = msg.text.clone().unwrap_or_default();
            let user = ev.source.as_ref().and_then(|s| s.user_id.clone()).unwrap_or_default();
            info!("[LINE] {} said: {}", user, text);
            let event = Event::new(
                Uuid::new_v4(), Uuid::new_v4(), EventKind::RunStarted,
                serde_json::json!({ "source": "line", "user_id": user, "text": text }),
            );
            let _ = state.supervisor_tx.send(Message::AuditEvent(AuditEventPayload { event })).await;
        }
    }
    StatusCode::OK
}

#[async_trait]
impl ChannelAdapter for LineAdapter {
    fn name(&self) -> &str { "line" }
    fn build_router(&self) -> Router {
        let state = AppState { supervisor_tx: self.supervisor_tx.clone() };
        Router::new().route(&self.config.webhook_path, post(webhook_handler)).with_state(state)
    }
    async fn start(&self, _supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        info!("[LINE] Adapter ready at {}", self.config.webhook_path);
        Ok(())
    }
}
