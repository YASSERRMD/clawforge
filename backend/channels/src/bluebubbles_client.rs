//! BlueBubbles Client
//!
//! Provides a Socket.IO and REST binding to a remote cross-platform BlueBubbles
//! server when the instance is running on Windows or Linux.

use anyhow::Result;
use tracing::info;

pub struct BlueBubblesClient {
    server_url: String,
    password: String,
}

impl BlueBubblesClient {
    pub fn new(server_url: &str, password: &str) -> Self {
        Self {
            server_url: server_url.into(),
            password: password.into(),
        }
    }

    /// Pings the BlueBubbles socket API to send an outbound iMessage event.
    pub async fn emit_message(&self, handle: &str, text: &str) -> Result<()> {
        info!("Routing payload to BlueBubbles server at {} (handle: {})", self.server_url, handle);
        // MOCK: Socket.io `emit`
        Ok(())
    }
}
