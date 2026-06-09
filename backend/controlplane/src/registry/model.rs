//! Agent Registry domain model.
//!
//! The registry is the single source of truth for every agent an organisation
//! runs. A record captures *who owns it*, *what it is allowed to touch*, and
//! *where it sits in its lifecycle* — independent of any particular runtime.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{DataAccessLevel, LifecycleStatus, RiskLevel};

/// A registered agent and all governance-relevant metadata about it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    /// Stable unique identifier (UUID v4).
    pub id: String,
    /// Human-friendly name.
    pub name: String,
    /// What the agent is for.
    pub description: String,
    /// Accountable owner (person or team).
    pub owner: String,
    /// Owning department / business unit.
    pub department: String,
    /// Framework the agent is built on (e.g. `openclaw`, `langgraph`).
    pub framework: String,
    /// Model provider (e.g. `anthropic`, `openrouter`, `ollama`).
    pub model_provider: String,
    /// Concrete model name (e.g. `claude-opus-4-8`).
    pub model_name: String,
    /// Tools the agent is permitted to invoke.
    pub tools_allowed: Vec<String>,
    /// MCP servers the agent is permitted to use.
    pub mcp_servers_allowed: Vec<String>,
    /// Highest data sensitivity the agent may access.
    pub data_access_level: DataAccessLevel,
    /// Assessed risk level.
    pub risk_level: RiskLevel,
    /// Lifecycle status.
    pub status: LifecycleStatus,
    /// Monotonic version, bumped on each metadata update.
    pub version: u32,
    /// Creation time (unix seconds).
    pub created_at: i64,
    /// Last update time (unix seconds).
    pub updated_at: i64,
}

/// Input used to create a new agent; the registry assigns id, status,
/// version, and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAgent {
    pub name: String,
    pub description: String,
    pub owner: String,
    pub department: String,
    pub framework: String,
    pub model_provider: String,
    pub model_name: String,
    #[serde(default)]
    pub tools_allowed: Vec<String>,
    #[serde(default)]
    pub mcp_servers_allowed: Vec<String>,
    pub data_access_level: DataAccessLevel,
    pub risk_level: RiskLevel,
}

impl AgentRecord {
    /// Materialise a fresh `AgentRecord` from a [`NewAgent`] input.
    ///
    /// New agents start in [`LifecycleStatus::Draft`] at version 1.
    pub fn from_new(input: NewAgent) -> Self {
        let now = Utc::now().timestamp();
        AgentRecord {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            description: input.description,
            owner: input.owner,
            department: input.department,
            framework: input.framework,
            model_provider: input.model_provider,
            model_name: input.model_name,
            tools_allowed: input.tools_allowed,
            mcp_servers_allowed: input.mcp_servers_allowed,
            data_access_level: input.data_access_level,
            risk_level: input.risk_level,
            status: LifecycleStatus::Draft,
            version: 1,
            created_at: now,
            updated_at: now,
        }
    }

    /// Whether the agent may currently execute actions.
    pub fn is_operational(&self) -> bool {
        self.status.is_operational()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input() -> NewAgent {
        NewAgent {
            name: "Permit Bot".into(),
            description: "Handles permit queries".into(),
            owner: "platform-team".into(),
            department: "Licensing".into(),
            framework: "openclaw".into(),
            model_provider: "anthropic".into(),
            model_name: "claude-opus-4-8".into(),
            tools_allowed: vec!["search".into()],
            mcp_servers_allowed: vec![],
            data_access_level: DataAccessLevel::Internal,
            risk_level: RiskLevel::Medium,
        }
    }

    #[test]
    fn from_new_starts_as_draft_v1() {
        let rec = AgentRecord::from_new(sample_input());
        assert_eq!(rec.status, LifecycleStatus::Draft);
        assert_eq!(rec.version, 1);
        assert!(!rec.id.is_empty());
        assert_eq!(rec.created_at, rec.updated_at);
        assert!(!rec.is_operational());
    }
}
