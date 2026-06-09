//! Agent marketplace domain model.
//!
//! The marketplace is a verified, internal catalogue of reusable agent
//! templates. Publishing puts an agent blueprint on the shelf; installing
//! stamps out a concrete agent into the [`registry`](crate::registry).

use serde::{Deserialize, Serialize};

use crate::constants::{DataAccessLevel, RiskLevel};
use crate::registry::NewAgent;

/// The reusable blueprint behind a marketplace listing: everything needed to
/// instantiate a concrete agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    /// Framework the instantiated agent runs on.
    pub framework: String,
    /// Default model provider.
    pub model_provider: String,
    /// Default model name.
    pub model_name: String,
    /// Tools the template requires.
    #[serde(default)]
    pub required_tools: Vec<String>,
    /// MCP servers the template requires.
    #[serde(default)]
    pub required_mcp_servers: Vec<String>,
    /// Model providers the template is approved against.
    #[serde(default)]
    pub required_model_providers: Vec<String>,
    /// Data sensitivity the instantiated agent will access.
    pub data_access_level: DataAccessLevel,
    /// Risk level of the instantiated agent.
    pub risk_level: RiskLevel,
}

impl AgentTemplate {
    /// Produce a [`NewAgent`] from this template for the given owner/department.
    pub fn to_new_agent(
        &self,
        name: impl Into<String>,
        description: impl Into<String>,
        owner: impl Into<String>,
        department: impl Into<String>,
    ) -> NewAgent {
        NewAgent {
            name: name.into(),
            description: description.into(),
            owner: owner.into(),
            department: department.into(),
            framework: self.framework.clone(),
            model_provider: self.model_provider.clone(),
            model_name: self.model_name.clone(),
            tools_allowed: self.required_tools.clone(),
            mcp_servers_allowed: self.required_mcp_servers.clone(),
            data_access_level: self.data_access_level,
            risk_level: self.risk_level,
        }
    }
}
