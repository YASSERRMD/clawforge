/// Microsoft Teams adapter — receives webhooks and sends via the Incoming Webhook
/// or Bot Framework.
///
/// Inbound: HTTP POST on the configured webhook path (Teams Outgoing Webhook format)
/// Outbound: POST to the Incoming Webhook URL or Bot Framework REST API
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
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

use clawforge_core::{AuditEventPayload, Event, EventKind, Message};

use crate::ChannelAdapter;

pub struct MSTeamsConfig {
    pub incoming_webhook_url: Option<String>,
    pub webhook_path: String,
}

pub struct MSTeamsAdapter {
    config: MSTeamsConfig,
    supervisor_tx: mpsc::Sender<Message>,
    http: Client,
}

impl MSTeamsAdapter {
    pub fn new(config: MSTeamsConfig, supervisor_tx: mpsc::Sender<Message>) -> Self {
        Self { config, supervisor_tx, http: Client::new() }
    }

    pub async fn send_message(&self, text: &str) -> Result<()> {
        if let Some(webhook_url) = &self.config.incoming_webhook_url {
            self.http.post(webhook_url)
                .json(&serde_json::json!({ "text": text }))
                .send()
                .await?
                .error_for_status()?;
        }
        Ok(())
    }
}

#[derive(Clone)]
struct AppState {
    supervisor_tx: mpsc::Sender<Message>,
}

#[derive(Deserialize)]
struct TeamsPayload {
    text: Option<String>,
    #[serde(rename = "from")]
    from: Option<TeamsFrom>,
}

#[derive(Deserialize)]
struct TeamsFrom {
    name: Option<String>,
}

async fn webhook_handler(
    State(state): State<AppState>,
    Json(payload): Json<TeamsPayload>,
) -> impl IntoResponse {
    let text = payload.text.unwrap_or_default();
    let sender = payload.from.and_then(|f| f.name).unwrap_or_else(|| "unknown".into());
    info!("[MSTeams] {} said: {}", sender, text);

    let event = Event::new(
        Uuid::new_v4(), Uuid::new_v4(), EventKind::RunStarted,
        serde_json::json!({ "source": "msteams", "sender": sender, "text": text }),
    );
    let _ = state.supervisor_tx.send(Message::AuditEvent(AuditEventPayload { event })).await;
    StatusCode::OK
}

#[async_trait]
impl ChannelAdapter for MSTeamsAdapter {
    fn name(&self) -> &str { "msteams" }

    fn build_router(&self) -> Router {
        let state = AppState { supervisor_tx: self.supervisor_tx.clone() };
        Router::new()
            .route(&self.config.webhook_path, post(webhook_handler))
            .with_state(state)
    }

    async fn start(&self, _supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        info!("[MSTeams] Adapter ready at {}", self.config.webhook_path);
        Ok(())
    }
}
