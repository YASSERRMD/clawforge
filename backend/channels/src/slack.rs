/// Slack channel adapter for ClawForge.
///
/// Receives Slack Events API webhooks and sends messages using the
/// Slack Web API (`chat.postMessage`).
///
/// Required env vars:
///   SLACK_SIGNING_SECRET  — used to verify X-Slack-Signature HMAC
///   SLACK_BOT_TOKEN       — Bot User OAuth Token (xoxb-...)
///   SLACK_WEBHOOK_PATH    — path to mount the webhook (default: /webhooks/slack)
use crate::ChannelAdapter;
use anyhow::Result;
use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use clawforge_core::{AuditEventPayload, Event, EventKind, Message};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SlackConfig {
    pub signing_secret: String,
    pub bot_token: String,
    pub webhook_path: String,
}

// ---------------------------------------------------------------------------
// Axum state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    config: SlackConfig,
    supervisor_tx: mpsc::Sender<Message>,
    http_client: Client,
}

// ---------------------------------------------------------------------------
// Slack wire types
// ---------------------------------------------------------------------------

/// Top-level event envelope from the Slack Events API.
#[derive(Deserialize, Debug)]
struct SlackEnvelope {
    #[serde(rename = "type")]
    event_type: String,
    /// Present on `url_verification` challenges.
    challenge: Option<String>,
    /// Present on `event_callback`.
    event: Option<SlackEvent>,
    /// Team ID for routing.
    team_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct SlackEvent {
    #[serde(rename = "type")]
    event_type: String,
    /// May be absent for bot messages.
    user: Option<String>,
    text: Option<String>,
    channel: Option<String>,
    ts: Option<String>,
    /// If set this is a bot message — ignore.
    bot_id: Option<String>,
    /// Thread timestamp for threading replies.
    thread_ts: Option<String>,
}

#[derive(Serialize)]
struct SlackPostMessage<'a> {
    channel: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_ts: Option<&'a str>,
}

// ---------------------------------------------------------------------------
// Adapter struct
// ---------------------------------------------------------------------------

pub struct SlackAdapter {
    config: SlackConfig,
    supervisor_tx: mpsc::Sender<Message>,
    http_client: Client,
}

impl SlackAdapter {
    pub fn new(config: SlackConfig, supervisor_tx: mpsc::Sender<Message>) -> Self {
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
            http_client: self.http_client.clone(),
        };
        Router::new()
            .route(&self.config.webhook_path, post(handle_slack_event))
            .with_state(state)
    }
}

// ---------------------------------------------------------------------------
// Webhook handler
// ---------------------------------------------------------------------------

async fn handle_slack_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // 1. Verify Slack signature (HMAC-SHA256 over timestamp + body)
    if !verify_slack_signature(&headers, &body, &state.config.signing_secret) {
        warn!("[Slack] Invalid signature — rejecting webhook");
        return (StatusCode::UNAUTHORIZED, "invalid_signature").into_response();
    }

    // 2. Parse JSON
    let envelope: SlackEnvelope = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(err) => {
            error!("[Slack] Failed to parse event envelope: {}", err);
            return (StatusCode::BAD_REQUEST, "bad_json").into_response();
        }
    };

    // 3. URL-verification challenge (required at initial setup)
    if envelope.event_type == "url_verification" {
        if let Some(challenge) = envelope.challenge {
            return (StatusCode::OK, challenge).into_response();
        }
    }

    // 4. Handle event callbacks
    if envelope.event_type != "event_callback" {
        return (StatusCode::OK, "ignored").into_response();
    }

    let Some(slack_event) = envelope.event else {
        return (StatusCode::OK, "no_event").into_response();
    };

    // 5. Only handle real user messages (message type, no bot_id)
    if slack_event.event_type != "message" || slack_event.bot_id.is_some() {
        return (StatusCode::OK, "ignored").into_response();
    }

    let Some(text) = slack_event.text else {
        return (StatusCode::OK, "no_text").into_response();
    };

    let channel = slack_event.channel.unwrap_or_else(|| "unknown".into());
    let user = slack_event.user.unwrap_or_else(|| "unknown_user".into());
    let ts = slack_event.ts.unwrap_or_default();

    info!("[Slack] Message from {} in {}: {}", user, channel, text);

    let event = Event::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        EventKind::RunStarted,
        serde_json::json!({
            "source": "slack",
            "team_id": envelope.team_id,
            "channel": channel,
            "user": user,
            "ts": ts,
            "thread_ts": slack_event.thread_ts,
            "text": text,
        }),
    );

    let _ = state
        .supervisor_tx
        .send(Message::AuditEvent(AuditEventPayload { event }))
        .await;

    (StatusCode::OK, "ok").into_response()
}

/// Verify the `X-Slack-Signature` header using HMAC-SHA256.
fn verify_slack_signature(headers: &HeaderMap, body: &[u8], signing_secret: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let sig = match headers
        .get("x-slack-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_owned(),
        None => return false,
    };
    let ts = match headers
        .get("x-slack-request-timestamp")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_owned(),
        None => return false,
    };

    let base = format!("v0:{}:{}", ts, std::str::from_utf8(body).unwrap_or(""));
    let mut mac = match Hmac::<Sha256>::new_from_slice(signing_secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(base.as_bytes());
    let computed = format!("v0={}", hex::encode(mac.finalize().into_bytes()));
    computed == sig
}

// ---------------------------------------------------------------------------
// ChannelAdapter impl
// ---------------------------------------------------------------------------

#[async_trait]
impl ChannelAdapter for SlackAdapter {
    fn name(&self) -> &str { "slack" }

    async fn start(&self, _supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        info!("[Slack] Adapter ready (webhook-based)");
        Ok(())
    }
}

impl SlackAdapter {
    pub async fn send_message(&self, channel: &str, text: &str) -> anyhow::Result<()> {
        let url = "https://slack.com/api/chat.postMessage";
        let body = SlackPostMessage {
            channel,
            text,
            thread_ts: None,
        };
        let res = self
            .http_client
            .post(url)
            .bearer_auth(&self.config.bot_token)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            error!("[Slack] chat.postMessage failed: {}", err);
            anyhow::bail!("Slack send failed: {}", err);
        }
        info!("[Slack] Sent message to channel {}", channel);
        Ok(())
    }
}
