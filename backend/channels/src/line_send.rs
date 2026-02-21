//! LINE Senders
//!
//! Transforms agent markup into text, flex messages, and quick reply panels.

use anyhow::Result;
use tracing::info;

pub struct LineSend;

impl LineSend {
    /// Dispatches a standard text string to a LINE user or group.
    pub async fn send_text(reply_token: &str, text: &str) -> Result<()> {
        info!("Replying to LINE user (token: {}). Text: {}", reply_token, text);
        Ok(())
    }

    /// Dispatches a rich Flex Message layout (JSON payload)
    pub async fn send_flex_message(reply_token: &str, alt_text: &str, json_layout: &str) -> Result<()> {
        info!("Sending Flex Message (alt: {}) to {}", alt_text, reply_token);
        Ok(())
    }

    /// Spawns interactive Quick Reply chips along the bottom of the chat viewport.
    pub async fn append_quick_replies(text: &str, replies: &[&str]) -> Result<()> {
        info!("Appending Quick Replies to message '{}': {:?}", text, replies);
        Ok(())
    }
}
