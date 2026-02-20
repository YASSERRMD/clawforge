/// Model auth profile rotation and fallback.
///
/// Mirrors `src/agents/model-auth.ts` + `src/agents/model-fallback.ts` from OpenClaw.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ---------------------------------------------------------------------------
// Auth profile
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProfile {
    pub id: String,
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
    /// Unix ts after which this key can be retried (0 = not in cooldown).
    pub cooldown_until: i64,
    /// Total failures since last success.
    pub fail_count: u32,
}

impl AuthProfile {
    pub fn new(id: impl Into<String>, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            api_key: api_key.into(),
            model: model.into(),
            base_url: None,
            cooldown_until: 0,
            fail_count: 0,
        }
    }

    pub fn is_available(&self) -> bool {
        now_secs() >= self.cooldown_until
    }
}

// ---------------------------------------------------------------------------
// Profile manager
// ---------------------------------------------------------------------------

pub struct AuthProfileManager {
    profiles: Vec<AuthProfile>,
    /// Round-robin cursor.
    cursor: usize,
    /// Cooldown in seconds after a failure.
    cooldown_secs: i64,
}

impl AuthProfileManager {
    pub fn new(profiles: Vec<AuthProfile>, cooldown_secs: i64) -> Self {
        Self { profiles, cursor: 0, cooldown_secs }
    }

    /// Get the next available profile (round-robin, skipping cooled-down keys).
    pub fn next_profile(&mut self) -> Option<&AuthProfile> {
        let n = self.profiles.len();
        for _ in 0..n {
            let i = self.cursor % n;
            self.cursor += 1;
            if self.profiles[i].is_available() {
                return Some(&self.profiles[i]);
            }
        }
        None
    }

    /// Mark a profile as failed — put it into cooldown.
    pub fn mark_failure(&mut self, profile_id: &str) {
        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == profile_id) {
            p.fail_count += 1;
            p.cooldown_until = now_secs() + self.cooldown_secs;
            warn!("[AuthProfile] {} in cooldown for {}s (fails={})",
                profile_id, self.cooldown_secs, p.fail_count);
        }
    }

    /// Mark a profile as succeeded — reset its cooldown.
    pub fn mark_success(&mut self, profile_id: &str) {
        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == profile_id) {
            p.fail_count = 0;
            p.cooldown_until = 0;
            info!("[AuthProfile] {} recovered", profile_id);
        }
    }
}

// ---------------------------------------------------------------------------
// Fallback config
// ---------------------------------------------------------------------------

/// Ordered list of models to try, from primary → fallback.
#[derive(Debug, Clone)]
pub struct FallbackChain {
    pub models: Vec<String>,
}

impl FallbackChain {
    pub fn new(primary: impl Into<String>) -> Self {
        Self { models: vec![primary.into()] }
    }

    pub fn then(mut self, fallback: impl Into<String>) -> Self {
        self.models.push(fallback.into());
        self
    }
}
