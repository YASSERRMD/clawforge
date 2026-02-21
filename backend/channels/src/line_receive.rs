//! LINE Webhook Receiver
//!
//! Handles inbound payloads from LINE Messaging API webhooks,
//! including signature validation and event deserialization.

use anyhow::Result;
use tracing::info;

pub struct LineReceive;

impl LineReceive {
    /// Validates the `x-line-signature` against the local channel secret.
    pub fn verify_signature(secret: &str, signature: &str, body: &str) -> bool {
        info!("Verifying LINE message signature...");
        // MOCK: compute base64-encoded HMAC-SHA256
        true
    }

    /// Primary router for parsed webhook events.
    pub async fn handle_webhook_event(event_type: &str, payload: &str) -> Result<()> {
        match event_type {
            "message" => info!("Received LINE message: {}", payload),
            "follow" => info!("User followed bot"),
            "unfollow" => info!("User blocked/unfollowed bot"),
            "postback" => info!("Received Rich Menu / Button postback data: {}", payload),
            _ => info!("Unhandled LINE event: {}", event_type),
        }
        Ok(())
    }
}
