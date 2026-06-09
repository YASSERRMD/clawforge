//! Stable product constants and shared enum vocabularies.
//!
//! These types are deliberately defined once here and reused across every
//! domain module (registry, governance, security, MCP, marketplace, …) so the
//! control plane speaks a single vocabulary for risk, data sensitivity, and
//! lifecycle state.

use serde::{Deserialize, Serialize};

/// Product name, used in audit records and report headers.
pub const PRODUCT_NAME: &str = "ClawForge Control Plane";

/// Short one-line positioning statement.
pub const POSITIONING: &str = "Kubernetes + ServiceNow + Splunk for AI Agents";

/// Schema version for persisted records; bumped on breaking storage changes.
pub const SCHEMA_VERSION: u32 = 1;

/// Risk classification shared by agents, tools, MCP servers, and integrations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    /// Numeric weight for risk scoring (higher = riskier).
    pub fn weight(&self) -> u8 {
        match self {
            RiskLevel::Low => 1,
            RiskLevel::Medium => 2,
            RiskLevel::High => 3,
            RiskLevel::Critical => 4,
        }
    }

    /// Whether this risk level requires a human approval gate by default.
    pub fn requires_approval(&self) -> bool {
        matches!(self, RiskLevel::High | RiskLevel::Critical)
    }
}

/// Data sensitivity tier an agent is permitted to access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataAccessLevel {
    /// No access to stored data.
    None,
    /// Non-sensitive, freely shareable data.
    Public,
    /// Business-internal data.
    Internal,
    /// Confidential data restricted to a department.
    Confidential,
    /// Personally identifiable / regulated data.
    Restricted,
}

/// Lifecycle status for a registered entity (agent, MCP server, integration).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LifecycleStatus {
    /// Created but not yet reviewed.
    Draft,
    /// Submitted and awaiting governance approval.
    PendingApproval,
    /// Approved and operational.
    Active,
    /// Temporarily suspended.
    Suspended,
    /// Permanently retired.
    Deactivated,
    /// Explicitly blocked by security or governance.
    Blocked,
}

impl LifecycleStatus {
    /// Whether an entity in this status may execute actions.
    pub fn is_operational(&self) -> bool {
        matches!(self, LifecycleStatus::Active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_weight_orders_correctly() {
        assert!(RiskLevel::Critical.weight() > RiskLevel::Low.weight());
        assert!(RiskLevel::High.requires_approval());
        assert!(!RiskLevel::Low.requires_approval());
    }

    #[test]
    fn data_access_levels_are_ordered() {
        assert!(DataAccessLevel::Restricted > DataAccessLevel::Public);
        assert!(DataAccessLevel::None < DataAccessLevel::Internal);
    }

    #[test]
    fn only_active_is_operational() {
        assert!(LifecycleStatus::Active.is_operational());
        assert!(!LifecycleStatus::Blocked.is_operational());
    }
}
