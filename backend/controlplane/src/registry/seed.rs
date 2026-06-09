//! Sample seed agents for demos and local development.
//!
//! These mirror realistic government / municipality and enterprise use cases so
//! the registry, governance, and observability views have meaningful data to
//! show out of the box.

use crate::constants::{DataAccessLevel, RiskLevel};
use crate::error::Result;

use super::model::{AgentRecord, NewAgent};
use super::store::AgentRegistry;

/// The built-in catalogue of example agents.
pub fn sample_agents() -> Vec<NewAgent> {
    vec![
        NewAgent {
            name: "Permit Intake Assistant".into(),
            description: "Triages building-permit applications for the Licensing department.".into(),
            owner: "licensing-platform".into(),
            department: "Licensing".into(),
            framework: "openclaw".into(),
            model_provider: "anthropic".into(),
            model_name: "claude-opus-4-8".into(),
            tools_allowed: vec!["search".into(), "document.read".into()],
            mcp_servers_allowed: vec!["records-mcp".into()],
            data_access_level: DataAccessLevel::Internal,
            risk_level: RiskLevel::Medium,
        },
        NewAgent {
            name: "Citizen Records Lookup".into(),
            description: "Answers staff queries against the resident records system.".into(),
            owner: "service-desk".into(),
            department: "Customer Happiness".into(),
            framework: "openclaw".into(),
            model_provider: "anthropic".into(),
            model_name: "claude-sonnet-4-6".into(),
            tools_allowed: vec!["search".into()],
            mcp_servers_allowed: vec!["records-mcp".into()],
            data_access_level: DataAccessLevel::Restricted,
            risk_level: RiskLevel::High,
        },
        NewAgent {
            name: "IT Ops Runbook Agent".into(),
            description: "Executes approved remediation runbooks for the IT operations team.".into(),
            owner: "it-ops".into(),
            department: "Information Technology".into(),
            framework: "openclaw".into(),
            model_provider: "openrouter".into(),
            model_name: "anthropic/claude-opus-4-8".into(),
            tools_allowed: vec!["shell".into(), "http.get".into()],
            mcp_servers_allowed: vec!["servicenow-mcp".into()],
            data_access_level: DataAccessLevel::Confidential,
            risk_level: RiskLevel::Critical,
        },
    ]
}

/// Insert the sample agents into a registry, returning the created records.
pub fn seed(registry: &AgentRegistry) -> Result<Vec<AgentRecord>> {
    let mut created = Vec::new();
    for input in sample_agents() {
        created.push(registry.create(input)?);
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_populates_registry() {
        let reg = AgentRegistry::in_memory().unwrap();
        let created = seed(&reg).unwrap();
        assert_eq!(created.len(), 3);
        assert_eq!(reg.count().unwrap(), 3);
    }
}
