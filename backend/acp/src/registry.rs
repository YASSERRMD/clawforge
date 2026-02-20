/// Sub-agent registry — tracks all spawned sub-agent sessions,
/// enforces depth limits, and manages lifecycle.
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{bail, Result};
use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::types::{SubAgentSession, SubAgentStatus};

/// Maximum nesting depth for sub-agents (prevents infinite recursion).
pub const MAX_SUBAGENT_DEPTH: usize = 5;

pub struct SubAgentRegistry {
    sessions: Arc<RwLock<HashMap<Uuid, SubAgentSession>>>,
}

impl SubAgentRegistry {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new sub-agent session, enforcing depth limits.
    pub async fn register(
        &self,
        parent_session_id: Option<Uuid>,
        agent_id: Option<Uuid>,
        prompt: String,
    ) -> Result<SubAgentSession> {
        let depth = if let Some(parent_id) = parent_session_id {
            let sessions = self.sessions.read().await;
            let parent = sessions.get(&parent_id).map(|s| s.depth).unwrap_or(0);
            parent + 1
        } else {
            0
        };

        if depth > MAX_SUBAGENT_DEPTH {
            bail!(
                "Sub-agent depth limit exceeded ({}/{})",
                depth,
                MAX_SUBAGENT_DEPTH
            );
        }

        let now = Utc::now().timestamp();
        let session = SubAgentSession {
            session_id: Uuid::new_v4(),
            parent_session_id,
            agent_id,
            depth,
            status: SubAgentStatus::Starting,
            prompt,
            result: None,
            created_at: now,
            updated_at: now,
        };

        info!(
            "Registered sub-agent session {} (depth {})",
            session.session_id, depth
        );

        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id, session.clone());
        Ok(session)
    }

    /// Update the status of a session.
    pub async fn update_status(&self, id: Uuid, status: SubAgentStatus, message: Option<String>) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&id) {
            session.status = status.clone();
            if let Some(msg) = message {
                session.result = Some(msg);
            }
            session.updated_at = Utc::now().timestamp();
            info!("Sub-agent {} → {:?}", id, status);
        } else {
            warn!("Sub-agent {} not found in registry", id);
        }
    }

    /// Get a session by ID.
    pub async fn get(&self, id: Uuid) -> Option<SubAgentSession> {
        self.sessions.read().await.get(&id).cloned()
    }

    /// List all sessions for a given parent (direct children only).
    pub async fn children(&self, parent_id: Uuid) -> Vec<SubAgentSession> {
        self.sessions
            .read()
            .await
            .values()
            .filter(|s| s.parent_session_id == Some(parent_id))
            .cloned()
            .collect()
    }

    /// List all active (non-terminal) sessions.
    pub async fn active(&self) -> Vec<SubAgentSession> {
        self.sessions
            .read()
            .await
            .values()
            .filter(|s| {
                !matches!(
                    s.status,
                    SubAgentStatus::Completed | SubAgentStatus::Failed | SubAgentStatus::Cancelled
                )
            })
            .cloned()
            .collect()
    }

    /// Remove completed/failed sessions older than `max_age_secs`.
    pub async fn gc(&self, max_age_secs: i64) {
        let now = Utc::now().timestamp();
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();
        sessions.retain(|_, s| {
            let terminal = matches!(
                s.status,
                SubAgentStatus::Completed | SubAgentStatus::Failed | SubAgentStatus::Cancelled
            );
            !terminal || (now - s.updated_at) < max_age_secs
        });
        let removed = before - sessions.len();
        if removed > 0 {
            info!("GC removed {} stale sub-agent sessions", removed);
        }
    }
}

impl Default for SubAgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
