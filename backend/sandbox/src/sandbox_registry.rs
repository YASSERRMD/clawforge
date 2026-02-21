//! Sandbox container registry: tracks active containers per session.

use crate::docker::DockerSandbox;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// An active sandbox entry.
pub struct SandboxEntry {
    pub session_id: String,
    pub sandbox: DockerSandbox,
}

/// Global registry of active sandbox containers (keyed by session_id).
pub struct SandboxRegistry {
    entries: Arc<RwLock<HashMap<String, SandboxEntry>>>,
}

impl SandboxRegistry {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a started sandbox for a session.
    pub async fn register(&self, session_id: String, sandbox: DockerSandbox) {
        self.entries
            .write()
            .await
            .insert(session_id.clone(), SandboxEntry { session_id, sandbox });
        info!(count = self.entries.read().await.len(), "Sandbox registered");
    }

    /// Remove a sandbox entry.
    pub async fn remove(&self, session_id: &str) -> Option<SandboxEntry> {
        self.entries.write().await.remove(session_id)
    }

    /// Stop and remove all sandboxes (called at shutdown).
    pub async fn stop_all(&self) -> Result<()> {
        let mut entries = self.entries.write().await;
        for (id, mut entry) in entries.drain() {
            warn!(session_id = %id, "Force-stopping sandbox at shutdown");
            entry.sandbox.stop().await.ok();
        }
        Ok(())
    }

    /// Count of currently active sandboxes.
    pub async fn active_count(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Check whether a session has an active sandbox.
    pub async fn has_sandbox(&self, session_id: &str) -> bool {
        self.entries.read().await.contains_key(session_id)
    }
}

impl Default for SandboxRegistry {
    fn default() -> Self {
        Self::new()
    }
}
