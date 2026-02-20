/// Cron delivery — routes cron output to the correct target session.
///
/// Mirrors `src/cron/delivery.ts` from OpenClaw.
/// A cron job can specify a `delivery_target` which is either:
///   - A session ID (send to that exact session)
///   - A channel string (send to that channel's active session)
///   - None (output is discarded / logged only)
use anyhow::Result;
use tracing::{info, warn};

/// Delivery target resolution result.
#[derive(Debug, Clone)]
pub enum DeliveryTarget {
    /// Deliver to a specific session by ID.
    Session(String),
    /// Deliver to a channel (find or create the active session).
    Channel(String),
    /// No delivery (log only).
    Discard,
}

/// Parse a delivery_target string from a cron job config.
pub fn parse_delivery_target(raw: &Option<String>) -> DeliveryTarget {
    match raw {
        None => DeliveryTarget::Discard,
        Some(s) if s.is_empty() => DeliveryTarget::Discard,
        Some(s) if s.starts_with("session:") => {
            DeliveryTarget::Session(s.trim_start_matches("session:").to_string())
        }
        Some(s) if s.starts_with("channel:") => {
            DeliveryTarget::Channel(s.trim_start_matches("channel:").to_string())
        }
        // Bare value — treat as channel name
        Some(s) => DeliveryTarget::Channel(s.clone()),
    }
}

/// Deliver the cron result to the resolved target.
/// In a real implementation this would call into the channel bus or session manager.
/// Here we log the delivery for traceability.
pub async fn deliver_result(target: &DeliveryTarget, content: &str, job_id: &str) -> Result<()> {
    match target {
        DeliveryTarget::Session(id) => {
            info!("[CronDelivery] job={} → session={}: {}", job_id, id, content);
            // TODO: call session_bus.send(session_id, content)
        }
        DeliveryTarget::Channel(ch) => {
            info!("[CronDelivery] job={} → channel={}: {}", job_id, ch, content);
            // TODO: call channel_adapter.send_message(ch, content)
        }
        DeliveryTarget::Discard => {
            warn!("[CronDelivery] job={}: no delivery target, discarding output", job_id);
        }
    }
    Ok(())
}
