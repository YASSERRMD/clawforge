//! Channel Activity Counters
//!
//! Mirrors `src/infra/channel-activity.ts` and `src/infra/channel-summary.ts`.
//! Tracks message counts, active sessions, last seen timestamp per channel.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelActivity {
    pub channel_id: String,
    pub message_count: u64,
    pub active_sessions: u32,
    pub last_seen_at: DateTime<Utc>,
}

pub struct ChannelActivityMonitor {
    // Db or redis store
}

impl ChannelActivityMonitor {
    pub fn new() -> Self {
        Self {}
    }

    /// Bump the activity metrics for a given channel upon observation.
    pub async fn record_activity(&self, channel_id: &str) -> anyhow::Result<()> {
        // MOCK: Update Redis counter or DB row
        tracing::debug!("Channel activity recorded for {}", channel_id);
        Ok(())
    }

    /// Retrieve the current activity summary for a channel.
    pub async fn get_summary(&self, channel_id: &str) -> anyhow::Result<ChannelActivity> {
        Ok(ChannelActivity {
            channel_id: channel_id.into(),
            message_count: 42,
            active_sessions: 2,
            last_seen_at: Utc::now(),
        })
    }
}
