//! Channel Health Monitor
//!
//! Mirrors `src/gateway/channel-health-monitor.ts`.
//! Periodically checks health of connected adapters (Discord, Telegram, etc.)
//! and reports degraded channels.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize)]
pub struct ChannelHealth {
    pub channel_id: String,
    pub status: String, // "healthy", "degraded", "offline"
    pub last_seen: DateTime<Utc>,
    pub latency_ms: Option<u64>,
}

#[derive(Clone)]
pub struct HealthMonitor {
    status_map: Arc<RwLock<HashMap<String, ChannelHealth>>>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            status_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update the health status of a specific channel.
    pub async fn update_status(&self, channel_id: String, health: ChannelHealth) {
        let mut w = self.status_map.write().await;
        w.insert(channel_id, health);
    }

    /// Retrieve the health report of all known channels.
    pub async fn get_report(&self) -> Vec<ChannelHealth> {
        let r = self.status_map.read().await;
        r.values().cloned().collect()
    }

    /// Start a background loop that periodically pings adapters if they support it.
    pub fn spawn_monitoring_loop(&self) {
        let monitor = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                info!("Running periodic channel health check...");
                // MOCK: In reality, we'd emit a ping event to the broker 
                // and wait for adapters to pong back, updating latency and status.
            }
        });
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}
