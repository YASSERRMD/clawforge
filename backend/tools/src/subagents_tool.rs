//! Subagents tool — spawn, steer, and stop sub-agent sessions from an agent.
//!
//! Mirrors `src/agents/tools/subagents-tool.ts`.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

/// A running sub-agent entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentEntry {
    pub id: String,
    pub parent_session_id: String,
    pub agent_name: Option<String>,
    pub status: SubagentStatus,
    pub task: Option<String>,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubagentStatus {
    Running,
    Stopped,
    Completed,
    Failed,
}

/// Input for spawn-subagent.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnSubagentInput {
    /// Name of the agent to run (from config).
    pub agent: Option<String>,
    /// Task/system prompt for the sub-agent.
    pub task: String,
    /// Initial user message.
    pub message: String,
    /// Max nesting depth (default: inherit from config).
    pub max_depth: Option<u32>,
    /// Environment variables to pass to the sub-agent.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Output from spawn-subagent.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnSubagentOutput {
    pub subagent_id: String,
    pub status: SubagentStatus,
}

/// Input for send-to-subagent (steer).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteerSubagentInput {
    pub subagent_id: String,
    pub message: String,
}

/// Input for stop-subagent.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopSubagentInput {
    pub subagent_id: String,
    pub reason: Option<String>,
}

/// Trait for backends that manage subagent lifecycle.
#[async_trait::async_trait]
pub trait SubagentBackend: Send + Sync {
    async fn spawn(&self, parent_session_id: &str, input: SpawnSubagentInput, depth: u32) -> Result<SubagentEntry>;
    async fn steer(&self, input: SteerSubagentInput) -> Result<()>;
    async fn stop(&self, input: StopSubagentInput) -> Result<()>;
    async fn get(&self, subagent_id: &str) -> Result<Option<SubagentEntry>>;
    async fn list(&self, parent_session_id: &str) -> Result<Vec<SubagentEntry>>;
}

/// In-process subagent registry (stores entries; actual execution done by backend).
pub struct SubagentRegistry {
    entries: Arc<RwLock<HashMap<String, SubagentEntry>>>,
    backend: Arc<dyn SubagentBackend>,
    max_depth: u32,
}

impl SubagentRegistry {
    pub fn new(backend: Arc<dyn SubagentBackend>, max_depth: u32) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            backend,
            max_depth,
        }
    }

    pub async fn spawn(
        &self,
        parent_session_id: &str,
        current_depth: u32,
        input: SpawnSubagentInput,
    ) -> Result<SpawnSubagentOutput> {
        if current_depth >= self.max_depth {
            anyhow::bail!(
                "Max subagent depth {} reached (current depth: {})",
                self.max_depth,
                current_depth
            );
        }

        let entry = self
            .backend
            .spawn(parent_session_id, input, current_depth + 1)
            .await?;

        let id = entry.id.clone();
        self.entries.write().await.insert(id.clone(), entry);

        Ok(SpawnSubagentOutput {
            subagent_id: id,
            status: SubagentStatus::Running,
        })
    }

    pub async fn steer(&self, input: SteerSubagentInput) -> Result<()> {
        self.backend.steer(input).await
    }

    pub async fn stop(&self, input: StopSubagentInput) -> Result<()> {
        let id = input.subagent_id.clone();
        self.backend.stop(input).await?;
        if let Some(entry) = self.entries.write().await.get_mut(&id) {
            entry.status = SubagentStatus::Stopped;
        }
        Ok(())
    }

    pub async fn list(&self, parent_session_id: &str) -> Result<Vec<SubagentEntry>> {
        self.backend.list(parent_session_id).await
    }
}

/// Generate a unique subagent ID.
pub fn new_subagent_id() -> String {
    format!("sub-{}", Uuid::new_v4().simple())
}
