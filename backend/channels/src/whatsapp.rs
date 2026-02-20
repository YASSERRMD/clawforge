use crate::ChannelAdapter;
use async_trait::async_trait;
use axum::{
    extract::{State, Json},
    routing::{post, get},
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{error, info};
use clawforge_core::{Message, EventKind, Event};
use uuid::Uuid;
use std::sync::Arc;
use std::net::SocketAddr;

#[derive(Clone)]
struct AppState {
    supervisor_tx: mpsc::Sender<Message>,
    verify_token: String,
}

// Basic Meta/WhatsApp Cloud API Webhook payloads
#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub object: String,
    pub entry: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub id: String,
    pub changes: Vec<Change>,
}

#[derive(Debug, Deserialize)]
pub struct Change {
    pub field: String,
    pub value: ChangeValue,
}

#[derive(Debug, Deserialize)]
pub struct ChangeValue {
    pub messaging_product: String,
    pub metadata: MetaData,
    #[serde(default)]
    pub messages: Vec<WhatsAppMessage>,
}

#[derive(Debug, Deserialize)]
pub struct MetaData {
    pub display_phone_number: String,
    pub phone_number_id: String,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppMessage {
    pub from: String,
    pub id: String,
    pub timestamp: String,
    pub text: Option<TextData>,
    #[serde(rename = "type")]
    pub msg_type: String,
}

#[derive(Debug, Deserialize)]
pub struct TextData {
    pub body: String,
}

// Verification payload
#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    #[serde(rename = "hub.mode")]
    pub mode: String,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: String,
    #[serde(rename = "hub.challenge")]
    pub challenge: String,
}

pub struct WhatsAppAdapter {
    port: u16,
    verify_token: String,
}

impl WhatsAppAdapter {
    pub fn new(port: u16, verify_token: String) -> Self {
        Self { port, verify_token }
    }
}

#[async_trait]
impl ChannelAdapter for WhatsAppAdapter {
    fn name(&self) -> &str { "whatsapp" }

    async fn start(&self, supervisor_tx: mpsc::Sender<Message>) -> anyhow::Result<()> {
        info!("Starting WhatsApp webhook adapter on port {}", self.port);
        
        let state = AppState {
            supervisor_tx,
            verify_token: self.verify_token.clone(),
        };

        let app = Router::new()
            .route("/webhook/whatsapp", axum::routing::get(verify_webhook).post(handle_webhook))
            .with_state(state);

        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        
        info!("WhatsApp webhook listening on {}", addr);
        
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("WhatsApp server error: {}", e);
            }
        });

        Ok(())
    }
}

impl WhatsAppAdapter {
    pub async fn send_message(&self, _chat_id: &str, _text: &str) -> anyhow::Result<()> {
        info!("WhatsApp send_message via Graph API not fully implemented in adapter yet.");
        Ok(())
    }
}


async fn verify_webhook(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<VerifyRequest>,
) -> Result<String, axum::http::StatusCode> {
    if params.mode == "subscribe" && params.verify_token == state.verify_token {
        info!("WhatsApp webhook verified successfully");
        Ok(params.challenge)
    } else {
        error!("WhatsApp webhook verification failed");
        Err(axum::http::StatusCode::FORBIDDEN)
    }
}

async fn handle_webhook(
    State(state): State<AppState>,
    Json(payload): Json<WebhookPayload>,
) -> axum::http::StatusCode {
    info!("Received WhatsApp webhook payload");
    
    if payload.object == "whatsapp_business_account" {
        for entry in payload.entry {
            for change in entry.changes {
                if change.field == "messages" {
                    for msg in change.value.messages {
                        if msg.msg_type == "text" {
                            if let Some(text_data) = msg.text {
                                let from = msg.from;
                                let text = text_data.body;
                                
                                info!("WhatsApp message from {}: {}", from, text);
                                
                                let event = Event::new(
                                    Uuid::new_v4(), // Dummy Run ID
                                    Uuid::new_v4(), // Dummy Agent ID
                                    EventKind::RunStarted,
                                    serde_json::json!({
                                        "source": "whatsapp",
                                        "from": from,
                                        "text": text
                                    })
                                );
                                
                                let _ = state.supervisor_tx.send(Message::AuditEvent(clawforge_core::AuditEventPayload { event })).await;
                            }
                        }
                    }
                }
            }
        }
    }
    
    axum::http::StatusCode::OK
}
