/// Stagger: random schedule jitter to spread cron load.
///
/// Mirrors `src/cron/stagger.ts` from OpenClaw.
/// When a job has `stagger_secs > 0`, the actual fire time is delayed
/// by a random amount between 0 and `stagger_secs`.
use std::time::Duration;

/// Returns a random delay in [0, stagger_secs).
pub fn stagger_delay(stagger_secs: u64) -> Duration {
    if stagger_secs == 0 {
        return Duration::ZERO;
    }
    // Simple pseudo-random based on current nanoseconds — no external dep.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let secs = nanos % stagger_secs;
    Duration::from_secs(secs)
}

/// Apply stagger before running a job — sleeps for the computed delay.
pub async fn apply_stagger(stagger_secs: u64) {
    let delay = stagger_delay(stagger_secs);
    if delay.as_secs() > 0 {
        tracing::debug!("[Cron] Stagger: sleeping {}s before job fire", delay.as_secs());
        tokio::time::sleep(delay).await;
    }
}
