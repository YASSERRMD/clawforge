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
        self.check_capabilities(req, &mut denials);
        self.check_budget(req, &mut denials);
        self.check_human_approval(req, &mut denials);

        let risk_score = Self::risk_score(req, denials.len());
        SecurityDecision::new(denials, risk_score, Utc::now().timestamp())
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

    /// Sensitive capabilities must each be enabled by policy.
    fn check_capabilities(&self, req: &ActionRequest, denials: &mut Vec<String>) {
        if req.requires_external_network && !self.policy.allow_external_network {
            denials.push("external network access is not permitted by policy".into());
        }
        if req.is_file_export && !self.policy.allow_file_export {
            denials.push("file export is not permitted by policy".into());
        }
        if req.is_database_write && !self.policy.allow_database_write {
            denials.push("database writes are not permitted by policy".into());
        }
        if req.touches_pii && !self.policy.allow_pii_access {
            denials.push("PII access is not permitted by policy".into());
        }
    }

    /// Projected spend (already spent + this action) must stay within budget.
    fn check_budget(&self, req: &ActionRequest, denials: &mut Vec<String>) {
        let projected = req.spent_so_far + req.estimated_cost;
        if projected > self.policy.budget_limit {
            denials.push(format!(
                "budget exceeded: projected {:.2} > limit {:.2}",
                projected, self.policy.budget_limit
            ));
        }
    }

    /// High/critical-risk actions cannot auto-execute when policy mandates a
    /// human approval gate.
    fn check_human_approval(&self, req: &ActionRequest, denials: &mut Vec<String>) {
        if self.policy.require_human_approval && req.agent.risk_level.requires_approval() {
            denials.push(format!(
                "human approval required for {:?}-risk action",
                req.agent.risk_level
            ));
        }
    }

    /// Compute an aggregate risk score (0–100) for an action.
    ///
    /// Combines the agent's inherent risk, the data sensitivity touched, the
    /// sensitive capabilities requested, and how many checks objected.
    fn risk_score(req: &ActionRequest, denial_count: usize) -> u32 {
        let mut score = req.agent.risk_level.weight() as u32 * 10; // 10..=40
        score += match req.data_access_level {
            crate::constants::DataAccessLevel::Restricted => 20,
            crate::constants::DataAccessLevel::Confidential => 12,
            crate::constants::DataAccessLevel::Internal => 6,
            crate::constants::DataAccessLevel::Public => 2,
            crate::constants::DataAccessLevel::None => 0,
        };
        for flag in [
            req.requires_external_network,
            req.is_file_export,
            req.is_database_write,
            req.touches_pii,
        ] {
            if flag {
                score += 5;
            }
        }
        score += denial_count as u32 * 5;
        score.min(100)
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

    /// A fully valid request under a permissive policy.
    fn valid_request() -> ActionRequest {
        let mut req = ActionRequest::for_agent(agent(LifecycleStatus::Active));
        req.tool = Some("search".into());
        req.mcp_server = Some("records-mcp".into());
        req.model = Some("claude-opus-4-8".into());
        req.data_access_level = DataAccessLevel::Internal;
        req.estimated_cost = 1.0;
        req.spent_so_far = 2.0;
        req
    }

    #[test]
    fn fully_valid_action_is_allowed() {
        let gw = SecurityGateway::new(SecurityPolicy::permissive());
        let decision = gw.evaluate(&valid_request());
        assert!(decision.allowed, "denials: {:?}", decision.denials);
        assert!(decision.denials.is_empty());
        assert!(decision.primary_reason().is_none());
    }

    #[test]
    fn allowed_action_carries_a_risk_score() {
        let gw = SecurityGateway::new(SecurityPolicy::permissive());
        let decision = gw.evaluate(&valid_request());
        // Medium agent (20) + internal data (6) = 26 => "medium" band.
        assert!(decision.risk_score > 0);
        assert_eq!(decision.risk_band(), "medium");
    }

    #[test]
    fn unspecified_optional_action_is_allowed() {
        // No tool/mcp/model and no sensitive capabilities under permissive policy.
        let gw = SecurityGateway::new(SecurityPolicy::permissive());
        let decision = gw.evaluate(&ActionRequest::for_agent(agent(LifecycleStatus::Active)));
        assert!(decision.allowed);
    }
}
