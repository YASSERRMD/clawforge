//! Message tool — send messages to channels from within an agent.
//!
//! Mirrors `src/agents/tools/message-tool.ts`.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Supported target channel types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageChannel {
    Telegram,
    Discord,
    Slack,
    WhatsApp,
    Signal,
    Line,
    IMessage,
}

/// Input for the message tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageToolInput {
    /// Target channel.
    pub channel: MessageChannel,
    /// Recipient or channel ID (phone number, user ID, channel ID).
    pub recipient: String,
    /// Message text to send (supports markdown where the channel supports it).
    pub text: String,
    /// Optional list of file paths to attach as media.
    #[serde(default)]
    pub attachments: Vec<String>,
    /// For threaded channels (Slack, Discord) — reply to this thread/message ID.
    #[serde(default)]
    pub thread_id: Option<String>,
    /// Optional agent session ID to bridge from (for routing).
    #[serde(default)]
    pub from_session: Option<String>,
}

/// Output from the message tool.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageToolOutput {
    pub ok: bool,
    /// Platform-specific message ID.
    pub message_id: Option<String>,
    /// Human-readable status.
    pub status: String,
}

/// Trait for channel message senders.
#[async_trait::async_trait]
pub trait MessageSender: Send + Sync {
    fn channel(&self) -> MessageChannel;
    async fn send(&self, input: &MessageToolInput) -> Result<MessageToolOutput>;
}

/// Registry of all available message senders (keyed by channel type).
pub struct MessageToolRegistry {
    senders: Vec<Box<dyn MessageSender>>,
}

impl MessageToolRegistry {
    pub fn new() -> Self {
        Self { senders: Vec::new() }
    }

    pub fn register(&mut self, sender: Box<dyn MessageSender>) {
        self.senders.push(sender);
    }

    /// Send a message to the appropriate channel.
    pub async fn send(&self, input: MessageToolInput) -> Result<MessageToolOutput> {
        let sender = self
            .senders
            .iter()
            .find(|s| s.channel() == input.channel)
            .ok_or_else(|| {
                anyhow::anyhow!("No sender registered for channel {:?}", input.channel)
            })?;
        sender.send(&input).await
    }
}

impl Default for MessageToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
