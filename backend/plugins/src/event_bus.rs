//! Event Bus
//!
//! Implements a publish-subscribe router allowing plugins to listen to global ClawForge events.

use anyhow::Result;
use tokio::sync::broadcast;
use tracing::info;

#[derive(Debug, Clone)]
pub enum SystemEvent {
    SessionStarted(String),
    MessageReceived(String, String), // session, content
    AgentThoughts(String, String),  // session, structured_thought
}

pub struct EventBus {
    sender: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { sender: tx }
    }

    /// Dispatches a high-level system event to all subscribed plugins.
    pub fn publish(&self, event: SystemEvent) {
        info!("Publishing SystemEvent to plugin bus: {:?}", event);
        let _ = self.sender.send(event);
    }

    /// Provides a reciever stream for a plugin to await events.
    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
