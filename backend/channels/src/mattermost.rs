/// Mattermost adapter — receives slash commands / outgoing webhooks and
/// sends via the Mattermost Incoming Webhook API.
use anyhow::Result;
use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Form, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid;

use clawforge_core::{AuditEventPayload, Event, EventKind, Message};

use crate::ChannelAdapter;

pub struct MattermostConfig {
    pub incoming_webhook_url: Option<String>,
    pub webhook_path: String,
    pub webhook_token: Option<String>,
}

pub struct MattermostAdapter {
    config: MattermostConfig,
    supervisor_tx: mpsc::Sender<Message>,
    http: Client,
}

impl MattermostAdapter {
    pub fn new(config: MattermostConfig, supervisor_tx: mpsc::Sender<Message>) -> Self {
        Self { config, supervisor_tx, http: Client::new() }
    }

    pub async fn send_message(&self, channel_id: &str, text: &str) -> Result<()> {
        if let Some(url) = &self.config.incoming_webhook_url {
            self.http.post(url)
                .json(&serde_json::json!({ "channel_id": channel_id, "text": text }))
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
    token: Option<String>,
}

/// Mattermost slash command form payload.
#[derive(Deserialize)]
struct SlashForm {
    token: Option<String>,
    user_name: Option<String>,
    text: Option<String>,
    channel_id: Option<String>,
}

async fn slash_handler(
    State(state): State<AppState>,
    Form(form): Form<SlashForm>,
) -> impl IntoResponse {
    // Validate token if configured
    if let Some(expected) = &state.token {
        if form.token.as_deref() != Some(expected.as_str()) {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }

    let text = form.text.unwrap_or_default();
    let user = form.user_name.unwrap_or_else(|| "unknown".into());
    let channel = form.channel_id.unwrap_or_default();
    info!("[Mattermost] @{} in #{}: {}", user, channel, text);

    let event = Event::new(
        Uuid::new_v4(), Uuid::new_v4(), EventKind::RunStarted,
        serde_json::json!({ "source": "mattermost", "user": user, "channel": channel, "text": text }),
    );
    let _ = state.supervisor_tx.send(Message::AuditEvent(AuditEventPayload { event })).await;
    (StatusCode::OK, "{\"text\":\"✅\"}").into_response()
}

#[async_trait]
impl ChannelAdapter for MattermostAdapter {
    fn name(&self) -> &str { "mattermost" }

    fn build_router(&self) -> Router {
        let state = AppState {
            supervisor_tx: self.supervisor_tx.clone(),
            token: self.config.webhook_token.clone(),
        };
        Router::new()
            .route(&self.config.webhook_path, post(slash_handler))
            .with_state(state)
    }

    async fn start(&self, _supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        info!("[Mattermost] Adapter ready at {}", self.config.webhook_path);
        Ok(())
    }
}
