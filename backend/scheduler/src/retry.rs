//! Retry engine: exponential backoff with jitter for scheduled job retries.
//!
//! Mirrors `src/scheduler/retry.ts`.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, warn};

/// Retry policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryPolicy {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Base delay between retries in milliseconds.
    pub base_delay_ms: u64,
    /// Multiplier for each subsequent wait (exponential factor).
    pub backoff_factor: f64,
    /// Maximum delay cap in milliseconds.
    pub max_delay_ms: u64,
    /// Add random jitter (±25% of computed delay) to avoid thundering herd.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1_000,
            backoff_factor: 2.0,
            max_delay_ms: 60_000,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Compute the delay before attempt `attempt_number` (1-indexed).
    pub fn delay_for(&self, attempt_number: u32) -> Duration {
        if attempt_number == 0 { return Duration::ZERO; }
        let delay_ms = self.base_delay_ms as f64
            * self.backoff_factor.powi((attempt_number - 1) as i32);
        let delay_ms = delay_ms.min(self.max_delay_ms as f64) as u64;

        let delay_ms = if self.jitter {
            // ±25% random jitter.
            let jitter = (delay_ms / 4) as i64;
            let offset: i64 = if jitter > 0 {
                (rand_offset() % (jitter as u64 * 2)) as i64 - jitter
            } else {
                0
            };
            (delay_ms as i64 + offset).max(0) as u64
        } else {
            delay_ms
        };

        Duration::from_millis(delay_ms)
    }

    pub fn should_retry(&self, attempt_number: u32) -> bool {
        attempt_number < self.max_attempts
    }
}

/// Simple xorshift64 for jitter without pulling in a full rand dep.
fn rand_offset() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(0x123456789abcdef0);
    let x = SEED.load(Ordering::Relaxed);
    let x = x ^ (x << 13);
    let x = x ^ (x >> 7);
    let x = x ^ (x << 17);
    SEED.store(x, Ordering::Relaxed);
    x
}

/// Retry state for tracking an in-progress job's retry lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryState {
    pub attempt: u32,
    pub last_error: Option<String>,
    pub next_retry_at: Option<u64>,
    pub exhausted: bool,
}

impl Default for RetryState {
    fn default() -> Self {
        Self {
            attempt: 0,
            last_error: None,
            next_retry_at: None,
            exhausted: false,
        }
    }
}

impl RetryState {
    /// Record a failure and compute next retry timing.
    pub fn record_failure(&mut self, policy: &RetryPolicy, error: &str) {
        self.attempt += 1;
        self.last_error = Some(error.to_string());

        if policy.should_retry(self.attempt) {
            let delay = policy.delay_for(self.attempt);
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.next_retry_at = Some(now_secs + delay.as_secs());
            warn!(
                attempt = self.attempt,
                max = policy.max_attempts,
                delay_secs = delay.as_secs(),
                "Job failed, will retry"
            );
        } else {
            self.exhausted = true;
            self.next_retry_at = None;
            warn!(
                attempt = self.attempt,
                "Job retry policy exhausted"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_backoff_grows() {
        let policy = RetryPolicy { jitter: false, ..Default::default() };
        let d1 = policy.delay_for(1).as_millis();
        let d2 = policy.delay_for(2).as_millis();
        let d3 = policy.delay_for(3).as_millis();
        assert!(d2 > d1, "delay should grow: {d1} < {d2}");
        assert!(d3 > d2, "delay should grow: {d2} < {d3}");
    }

    #[test]
    fn respects_max_delay() {
        let policy = RetryPolicy {
            max_delay_ms: 5_000,
            jitter: false,
            ..Default::default()
        };
        let d10 = policy.delay_for(10).as_millis();
        assert!(d10 <= 5_000, "delay capped at max: {d10}");
    }

    #[test]
    fn exhaustion_after_max_attempts() {
        let policy = RetryPolicy { max_attempts: 2, jitter: false, ..Default::default() };
        assert!(policy.should_retry(1));
        assert!(!policy.should_retry(2));
    }
}
