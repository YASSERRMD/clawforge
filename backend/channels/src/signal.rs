/// Signal adapter — stub for Signal messenger integration via signal-cli or AnySignal.
///
/// Full integration requires running signal-cli as a local daemon or using
/// the AnySignal cloud API. This stub implements the ChannelAdapter interface
/// and provides a placeholder for future wiring.
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{info, warn};

use clawforge_core::{Message};

use crate::ChannelAdapter;

pub struct SignalConfig {
    /// Phone number registered with Signal (e.g. "+14155551234")
    pub phone_number: String,
    /// Optional AnySignal or signal-cli REST API URL
    pub api_url: Option<String>,
    /// Optional API key for cloud providers
    pub api_key: Option<String>,
}

pub struct SignalAdapter {
    config: SignalConfig,
    supervisor_tx: mpsc::Sender<Message>,
}

impl SignalAdapter {
    pub fn new(config: SignalConfig, supervisor_tx: mpsc::Sender<Message>) -> Self {
        Self { config, supervisor_tx }
    }

    /// Send a Signal message to the given recipient.
    pub async fn send_message(&self, recipient: &str, text: &str) -> Result<()> {
        if let (Some(url), Some(key)) = (&self.config.api_url, &self.config.api_key) {
            let client = reqwest::Client::new();
            client.post(format!("{}/v1/send", url))
                .header("Authorization", format!("Bearer {}", key))
                .json(&serde_json::json!({
                    "number": self.config.phone_number,
                    "recipients": [recipient],
                    "message": text
                }))
                .send()
                .await?
                .error_for_status()?;
        } else {
            warn!("[Signal] No API configured — send is a no-op (stub)");
        }
        Ok(())
    }
}

#[async_trait]
impl ChannelAdapter for SignalAdapter {
    fn name(&self) -> &str { "signal" }

    fn build_router(&self) -> axum::Router {
        // Signal uses webhooks or polling — no inbound router needed for stub
        axum::Router::new()
    }

    async fn start(&self, _supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        info!("[Signal] Adapter started (stub) for {}", self.config.phone_number);
        Ok(())
    }
}
