//! The security gateway engine.
//!
//! `evaluate` runs every pre-execution check against an [`ActionRequest`] and
//! returns a [`SecurityDecision`]. Checks are additive: each one may append a
//! denial reason; the action is allowed only if no check objected.

use chrono::Utc;

use crate::constants::LifecycleStatus;

use super::decision::SecurityDecision;
use super::policy::SecurityPolicy;
use super::request::ActionRequest;

/// Stateless policy evaluator. Construct once with a [`SecurityPolicy`] and
/// reuse across requests.
pub struct SecurityGateway {
    policy: SecurityPolicy,
}

impl SecurityGateway {
    /// Create a gateway enforcing the given policy.
    pub fn new(policy: SecurityPolicy) -> Self {
        Self { policy }
    }

    /// The policy this gateway enforces.
    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    /// Run all pre-execution checks and return the decision.
    pub fn evaluate(&self, req: &ActionRequest) -> SecurityDecision {
        let mut denials: Vec<String> = Vec::new();

        self.check_agent_state(req, &mut denials);
        self.check_tool(req, &mut denials);
        self.check_mcp(req, &mut denials);
        self.check_model(req, &mut denials);
        self.check_data_access(req, &mut denials);

        SecurityDecision::new(denials, 0, Utc::now().timestamp())
    }

    /// The tool, if any, must be on the agent's allow-list.
    fn check_tool(&self, req: &ActionRequest, denials: &mut Vec<String>) {
        if let Some(tool) = &req.tool {
            if !req.agent.tools_allowed.iter().any(|t| t == tool) {
                denials.push(format!("tool '{tool}' is not allowed for this agent"));
            }
        }
    }

    /// The MCP server, if any, must be on the agent's allow-list.
    fn check_mcp(&self, req: &ActionRequest, denials: &mut Vec<String>) {
        if let Some(server) = &req.mcp_server {
            if !req.agent.mcp_servers_allowed.iter().any(|s| s == server) {
                denials.push(format!("MCP server '{server}' is not allowed for this agent"));
            }
        }
    }

    /// The model, if specified, must match the agent's approved model.
    fn check_model(&self, req: &ActionRequest, denials: &mut Vec<String>) {
        if let Some(model) = &req.model {
            if model != &req.agent.model_name {
                denials.push(format!(
                    "model '{model}' is not the agent's approved model '{}'",
                    req.agent.model_name
                ));
            }
        }
    }

    /// The action's data sensitivity must not exceed the agent's clearance nor
    /// the policy's ceiling (`DataAccessLevel` is ordered).
    fn check_data_access(&self, req: &ActionRequest, denials: &mut Vec<String>) {
        if req.data_access_level > req.agent.data_access_level {
            denials.push(format!(
                "action data access {:?} exceeds agent clearance {:?}",
                req.data_access_level, req.agent.data_access_level
            ));
        }
        if req.data_access_level > self.policy.max_data_access_level {
            denials.push(format!(
                "action data access {:?} exceeds policy ceiling {:?}",
                req.data_access_level, self.policy.max_data_access_level
            ));
        }
    }

    /// Base check: the agent must be active and not blocked/deactivated.
    fn check_agent_state(&self, req: &ActionRequest, denials: &mut Vec<String>) {
        match req.agent.status {
            LifecycleStatus::Active => {}
            LifecycleStatus::Blocked => denials.push("agent is blocked".into()),
            LifecycleStatus::Deactivated => denials.push("agent is deactivated".into()),
            other => denials.push(format!("agent is not active (status: {other:?})")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DataAccessLevel, RiskLevel};
    use crate::registry::{AgentRecord, NewAgent};

    pub(super) fn agent(status: LifecycleStatus) -> AgentRecord {
        let mut a = AgentRecord::from_new(NewAgent {
            name: "Bot".into(),
            description: "d".into(),
            owner: "o".into(),
            department: "IT".into(),
            framework: "openclaw".into(),
            model_provider: "anthropic".into(),
            model_name: "claude-opus-4-8".into(),
            tools_allowed: vec!["search".into()],
            mcp_servers_allowed: vec!["records-mcp".into()],
            data_access_level: DataAccessLevel::Internal,
            risk_level: RiskLevel::Medium,
        });
        a.status = status;
        a
    }

    #[test]
    fn inactive_agent_is_denied() {
        let gw = SecurityGateway::new(SecurityPolicy::default());
        let decision = gw.evaluate(&ActionRequest::for_agent(agent(LifecycleStatus::Draft)));
        assert!(!decision.allowed);
    }

    #[test]
    fn active_agent_passes_base_check() {
        let gw = SecurityGateway::new(SecurityPolicy::permissive());
        let decision = gw.evaluate(&ActionRequest::for_agent(agent(LifecycleStatus::Active)));
        assert!(decision.allowed);
    }
}
