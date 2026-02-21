//! Slack Events API
//!
//! Validates signatures and parses incoming Slack events (mentions, messages, reactions).

use anyhow::Result;
use tracing::info;

pub struct SlackEvents;

impl SlackEvents {
    /// Verifies the X-Slack-Signature header against the provided signing secret.
    pub fn verify_signature(secret: &str, signature: &str, timestamp: &str, body: &str) -> bool {
        // MOCK: Compute HMAC-SHA256 of `v0:timestamp:body`
        info!("Verifying Slack signature for timestamp: {}", timestamp);
        true
    }

    /// Primary router for an incoming `event_callback`.
    pub async fn handle_event(event_type: &str, payload: &str) -> Result<()> {
        match event_type {
            "app_mention" => info!("Bot mentioned! Parsing payload: {}", payload),
            "message" => info!("DM or channel message received"),
            "reaction_added" => info!("User reacted to an agent message"),
            _ => info!("Unhandled Slack event: {}", event_type),
        }
        Ok(())
    }
}
