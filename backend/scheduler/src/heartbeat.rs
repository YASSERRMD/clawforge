/// Heartbeat system — keep-alive pings and ghost reminders.
///
/// Mirrors `src/infra/heartbeat-runner.ts` from OpenClaw.
use std::time::Duration;
use tokio::time;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Active hours config
// ---------------------------------------------------------------------------

/// Defines the active hours window within which heartbeats fire.
#[derive(Debug, Clone)]
pub struct ActiveHours {
    /// Hour of day (0–23) at which the bot becomes active.
    pub start_hour: u8,
    /// Hour of day (0–23) at which the bot goes silent.
    pub end_hour: u8,
    /// IANA timezone string (e.g. "America/New_York").
    pub timezone: String,
}

impl ActiveHours {
    pub fn always() -> Self {
        Self { start_hour: 0, end_hour: 24, timezone: "UTC".to_string() }
    }

    /// Check if the current local time is within active hours.
    pub fn is_active(&self) -> bool {
        // Get current UTC hour via SystemTime
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let now_hour = ((secs / 3600) % 24) as u8;
        if self.start_hour < self.end_hour {
            now_hour >= self.start_hour && now_hour < self.end_hour
        } else {
            now_hour >= self.start_hour || now_hour < self.end_hour
        }
    }
}

// ---------------------------------------------------------------------------
// Heartbeat runner
// ---------------------------------------------------------------------------

pub struct HeartbeatConfig {
    /// Interval at which heartbeats fire.
    pub interval: Duration,
    /// How long before a dormant session gets a ghost reminder.
    pub ghost_reminder_after: Duration,
    pub active_hours: ActiveHours,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
            ghost_reminder_after: Duration::from_secs(3600),
            active_hours: ActiveHours::always(),
        }
    }
}

/// Run the heartbeat loop indefinitely.
/// `on_heartbeat` is called every interval when within active hours.
/// `on_ghost_reminder` is called when a session has been idle > ghost_reminder_after.
pub async fn run_heartbeat_loop(
    config: HeartbeatConfig,
    session_id: String,
    mut on_heartbeat: impl FnMut(&str) + Send + 'static,
    mut on_ghost_reminder: impl FnMut(&str) + Send + 'static,
) {
    let mut idle_secs = 0u64;
    let ghost_threshold = config.ghost_reminder_after.as_secs();
    let mut ghost_fired = false;

    let mut interval = time::interval(config.interval);

    loop {
        interval.tick().await;

        if !config.active_hours.is_active() {
            debug!("[Heartbeat] Outside active hours — skipping");
            continue;
        }

        idle_secs += config.interval.as_secs();
        info!("[Heartbeat] Tick for session {} (idle={}s)", session_id, idle_secs);
        on_heartbeat(&session_id);

        if idle_secs >= ghost_threshold && !ghost_fired {
            warn!("[Heartbeat] Ghost reminder for session {} (idle {}s)", session_id, idle_secs);
            on_ghost_reminder(&session_id);
            ghost_fired = true;
        }
    }
}

// (no external crate imports needed — using SystemTime above)
