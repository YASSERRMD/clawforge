//! MCP governance domain model.
//!
//! The MCP registry governs the Model Context Protocol servers an organisation
//! exposes to its agents — the same way the agent registry governs agents.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{LifecycleStatus, RiskLevel};

/// Transport an MCP server speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    Stdio,
    Http,
    Sse,
    WebSocket,
}

/// Liveness of an MCP server as of the last health check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Never checked.
    Unknown,
    /// Last check succeeded.
    Healthy,
    /// Last check showed degradation.
    Degraded,
    /// Last check failed.
    Down,
}

/// A single tool exposed by an MCP server, and the permissions it needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name as advertised by the server.
    pub name: String,
    /// What the tool does.
    pub description: String,
    /// Permission scopes the tool requires (e.g. `read`, `network`, `fs`).
    #[serde(default)]
    pub permissions: Vec<String>,
}

impl McpTool {
    /// Whether this tool requests any sensitive permission scope.
    pub fn is_sensitive(&self) -> bool {
        self.permissions
            .iter()
            .any(|p| matches!(p.as_str(), "network" | "fs" | "write" | "exec" | "pii"))
    }
}

/// A registered MCP server and its governance metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    /// Stable unique identifier (UUID v4).
    pub id: String,
    pub name: String,
    pub description: String,
    /// Accountable owner.
    pub owner: String,
    /// Connection endpoint (URL or command, depending on transport).
    pub endpoint: String,
    /// Transport the server speaks.
    pub transport: TransportType,
    /// Tools the server exposes.
    pub tools_exposed: Vec<McpTool>,
    /// Permission scopes the server requires overall.
    pub permissions_required: Vec<String>,
    /// Assessed risk level.
    pub risk_level: RiskLevel,
    /// Governance status (`PendingApproval` / `Active` / `Blocked` / …).
    pub status: LifecycleStatus,
    /// Liveness as of the last health check.
    pub health: HealthStatus,
    /// Unix time of the last health check, if any.
    pub last_health_check: Option<i64>,
    /// Number of times the server has been called.
    pub usage_count: u64,
    /// Accumulated cost estimate.
    pub cost_estimate: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Input used to register a new MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMcpServer {
    pub name: String,
    pub description: String,
    pub owner: String,
    pub endpoint: String,
    pub transport: TransportType,
    #[serde(default)]
    pub tools_exposed: Vec<McpTool>,
    #[serde(default)]
    pub permissions_required: Vec<String>,
    pub risk_level: RiskLevel,
}

impl McpServer {
    /// Materialise a fresh server record; starts in `PendingApproval`.
    pub fn from_new(input: NewMcpServer) -> Self {
        let now = Utc::now().timestamp();
        McpServer {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            description: input.description,
            owner: input.owner,
            endpoint: input.endpoint,
            transport: input.transport,
            tools_exposed: input.tools_exposed,
            permissions_required: input.permissions_required,
            risk_level: input.risk_level,
            status: LifecycleStatus::PendingApproval,
            health: HealthStatus::Unknown,
            last_health_check: None,
            usage_count: 0,
            cost_estimate: 0.0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Whether this server may currently be used by agents.
    pub fn is_usable(&self) -> bool {
        self.status.is_operational()
    }
}
