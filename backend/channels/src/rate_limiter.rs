//! Channel rate limiter: per-channel, per-user token-bucket rate limiting.
//!
//! Mirrors `src/channels/rate-limiter.ts`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::debug;

/// Rate limit policy for a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitPolicy {
    /// Max messages allowed per window.
    pub max_messages: u32,
    /// Window duration in seconds.
    pub window_secs: u64,
    /// If true, apply per-user limits; if false, apply per-channel globals.
    pub per_user: bool,
}

impl Default for RateLimitPolicy {
    fn default() -> Self {
        Self {
            max_messages: 20,
            window_secs: 60,
            per_user: true,
        }
    }
}

/// Rate limit result.
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    /// Seconds until the limit resets.
    pub reset_in_secs: u64,
    /// Remaining quota in this window.
    pub remaining: u32,
}

/// Per-key token-bucket state.
struct BucketState {
    count: u32,
    window_start: Instant,
}

/// Rate limiter backed by an in-memory token-bucket per key.
pub struct ChannelRateLimiter {
    policy: RateLimitPolicy,
    buckets: Arc<Mutex<HashMap<String, BucketState>>>,
}

impl ChannelRateLimiter {
    pub fn new(policy: RateLimitPolicy) -> Self {
        Self {
            policy,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check and update the rate limit for a (channel_id, user_id) pair.
    /// Returns whether the message is allowed.
    pub async fn check(&self, channel_id: &str, user_id: &str) -> RateLimitResult {
        let key = if self.policy.per_user {
            format!("{channel_id}:{user_id}")
        } else {
            channel_id.to_string()
        };

        let window = Duration::from_secs(self.policy.window_secs);
        let mut buckets = self.buckets.lock().await;
        let now = Instant::now();

        let state = buckets.entry(key.clone()).or_insert_with(|| BucketState {
            count: 0,
            window_start: now,
        });

        // Reset window if expired.
        if now.duration_since(state.window_start) >= window {
            state.count = 0;
            state.window_start = now;
        }

        let elapsed = now.duration_since(state.window_start);
        let reset_in_secs = window.saturating_sub(elapsed).as_secs();

        if state.count < self.policy.max_messages {
            state.count += 1;
            let remaining = self.policy.max_messages - state.count;
            debug!(key = %key, count = state.count, remaining, "Rate limit check: allowed");
            RateLimitResult { allowed: true, reset_in_secs, remaining }
        } else {
            debug!(key = %key, count = state.count, "Rate limit check: denied");
            RateLimitResult { allowed: false, reset_in_secs, remaining: 0 }
        }
    }

    /// Flush all expired buckets to free memory.
    pub async fn cleanup(&self) {
        let window = Duration::from_secs(self.policy.window_secs);
        let now = Instant::now();
        self.buckets.lock().await.retain(|_, state| {
            now.duration_since(state.window_start) < window
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn limits_after_max() {
        let policy = RateLimitPolicy {
            max_messages: 2,
            window_secs: 60,
            per_user: true,
        };
        let limiter = ChannelRateLimiter::new(policy);
        let r1 = limiter.check("telegram", "user1").await;
        let r2 = limiter.check("telegram", "user1").await;
        let r3 = limiter.check("telegram", "user1").await;
        assert!(r1.allowed);
        assert!(r2.allowed);
        assert!(!r3.allowed);
    }

    #[tokio::test]
    async fn different_users_have_separate_limits() {
        let policy = RateLimitPolicy {
            max_messages: 1,
            window_secs: 60,
            per_user: true,
        };
        let limiter = ChannelRateLimiter::new(policy);
        let r1 = limiter.check("telegram", "user1").await;
        let r2 = limiter.check("telegram", "user2").await;
        assert!(r1.allowed);
        assert!(r2.allowed); // Different user, fresh bucket.
    }
}
