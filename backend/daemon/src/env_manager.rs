//! Environment variable manager for the ClawForge daemon service.
//!
//! Manages the runtime environment: reads, sets, and persists env vars
//! used by daemon processes across service restarts.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs;
use tracing::{debug, info};

/// A persisted environment variable entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
    /// If true, the value is sensitive and should not be logged.
    pub sensitive: bool,
}

/// The daemon environment store.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvStore {
    pub vars: HashMap<String, EnvVar>,
}

impl EnvStore {
    /// Load env store from a JSON file.
    pub async fn load(path: &std::path::Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read env store: {}", path.display()))?;
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    /// Save env store to a JSON file.
    pub async fn save(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json).await?;
        fs::rename(&tmp, path).await?;
        Ok(())
    }

    /// Set an environment variable and apply it to the current process.
    pub fn set(&mut self, key: &str, value: &str, sensitive: bool) {
        // SAFETY: ClawForge is single-threaded at init time; safe to set env vars.
        unsafe { std::env::set_var(key, value); }
        self.vars.insert(
            key.to_string(),
            EnvVar {
                key: key.to_string(),
                value: value.to_string(),
                sensitive,
            },
        );
        if sensitive {
            debug!(key = %key, "Env var set (sensitive, value hidden)");
        } else {
            debug!(key = %key, value = %value, "Env var set");
        }
    }

    /// Remove an environment variable.
    pub fn unset(&mut self, key: &str) {
        // SAFETY: single-threaded at init time.
        unsafe { std::env::remove_var(key); }
        self.vars.remove(key);
        debug!(key = %key, "Env var removed");
    }

    /// Apply all stored env vars to the current process.
    pub fn apply_all(&self) {
        for var in self.vars.values() {
            // SAFETY: called before spawning threads.
            unsafe { std::env::set_var(&var.key, &var.value); }
        }
        info!(count = self.vars.len(), "Applied daemon env vars to process");
    }

    /// Get a snapshot of all non-sensitive env vars.
    pub fn public_snapshot(&self) -> HashMap<String, String> {
        self.vars
            .values()
            .filter(|v| !v.sensitive)
            .map(|v| (v.key.clone(), v.value.clone()))
            .collect()
    }
}
