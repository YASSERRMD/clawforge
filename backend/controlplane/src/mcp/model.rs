//! MCP governance domain model.
//!
//! The MCP registry governs the Model Context Protocol servers an organisation
//! exposes to its agents — the same way the agent registry governs agents.

use serde::{Deserialize, Serialize};

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
