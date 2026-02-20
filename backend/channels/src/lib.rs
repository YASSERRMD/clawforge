use async_trait::async_trait;
use clawforge_core::Message;
use tokio::sync::mpsc;

pub mod telegram;
pub mod discord;
pub mod whatsapp;
pub mod bluebubbles;
pub mod slack;
pub mod matrix;

#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// Starts the channel adapter, listening for incoming messages from the platform
    /// and routing them via the `supervisor_tx` to the backend.
    async fn start(&self, supervisor_tx: mpsc::Sender<Message>) -> anyhow::Result<()>;
    
    /// Sends a text message back to the given conversation/chat ID on this platform.
    async fn send_message(&self, chat_id: &str, text: &str) -> anyhow::Result<()>;
}
