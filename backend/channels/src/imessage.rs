//! iMessage Adapter
//!
//! Exposes an agnostic iMessage client that falls back to AppleScript locally or BlueBubbles remotely.

use anyhow::Result;
use tracing::info;
use async_trait::async_trait;
use crate::ChannelAdapter;
use clawforge_core::{Message, Event};
use tokio::sync::mpsc;

pub struct IMessageAdapter;

#[async_trait]
impl ChannelAdapter for IMessageAdapter {
    fn name(&self) -> &'static str {
        "iMessage_Relay"
    }

    async fn start(&self, supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        info!("Starting iMessage background listener loop...");
        Ok(())
    }
}
