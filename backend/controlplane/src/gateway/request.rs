//! Action request model — what the gateway is asked to authorise.
//!
//! An [`ActionRequest`] is everything the gateway needs to make an allow/deny
//! decision without calling back into other stores: the agent record itself
//! plus the specifics of the action being attempted.

use serde::{Deserialize, Serialize};

use crate::constants::DataAccessLevel;
use crate::registry::AgentRecord;

/// A single action an agent wishes to perform, presented for authorisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    /// The agent attempting the action (its current registry record).
    pub agent: AgentRecord,
    /// Tool the action would invoke, if any.
    pub tool: Option<String>,
    /// MCP server the action would call, if any.
    pub mcp_server: Option<String>,
    /// Model the action would use, if any.
    pub model: Option<String>,
    /// Sensitivity of the data this action touches.
    pub data_access_level: DataAccessLevel,
    /// Estimated cost of this action.
    pub estimated_cost: f64,
    /// Spend already consumed by this agent in the budget window.
    pub spent_so_far: f64,
    /// Whether the action reaches the external network.
    pub requires_external_network: bool,
    /// Whether the action exports files out of the environment.
    pub is_file_export: bool,
    /// Whether the action writes to a database.
    pub is_database_write: bool,
    /// Whether the action touches PII / regulated data.
    pub touches_pii: bool,
}

impl ActionRequest {
    /// Build a minimal request for the given agent with no special capabilities.
    pub fn for_agent(agent: AgentRecord) -> Self {
        ActionRequest {
            agent,
            tool: None,
            mcp_server: None,
            model: None,
            data_access_level: DataAccessLevel::None,
            estimated_cost: 0.0,
            spent_so_far: 0.0,
            requires_external_network: false,
            is_file_export: false,
            is_database_write: false,
            touches_pii: false,
        }
    }
}
