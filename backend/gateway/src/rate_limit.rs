//! Gateway Rate Limiting Module
//!
//! Mirrors `src/gateway/auth-rate-limit.ts`.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use std::time::{Instant, Duration};

/// A naive token-bucket or sliding window rate limiter state.
#[derive(Clone)]
pub struct RateLimiter {
    // ip_address -> (request_count, window_start)
    limits: Arc<RwLock<HashMap<String, (u32, Instant)>>>,
    pub max_requests: u32,
    pub window: Duration,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            max_requests: 100,
            window: Duration::from_secs(60),
        }
    }
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Check if a request from the given IP is allowed.
    pub async fn check_limit(&self, ip: &str) -> bool {
        let mut limits = self.limits.write().await;
        let now = Instant::now();

        let state = limits.entry(ip.to_string()).or_insert((0, now));

        if now.duration_since(state.1) > self.window {
            // Reset window
            state.0 = 1;
            state.1 = now;
            debug!("Rate limit reset for IP {}", ip);
            true
        } else {
            state.0 += 1;
            if state.0 > self.max_requests {
                warn!("Rate limit exceeded for IP {}", ip);
                false
            } else {
                debug!("Rate limit OK for IP {} ({}/{})", ip, state.0, self.max_requests);
                true
            }
        }
    }
}
