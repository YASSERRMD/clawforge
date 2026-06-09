//! Security policy model.
//!
//! A [`SecurityPolicy`] expresses the capabilities an agent is *permitted* to
//! use at execution time. The gateway evaluates each [`ActionRequest`] against
//! the policy before the runtime is allowed to proceed.

use serde::{Deserialize, Serialize};

use crate::constants::DataAccessLevel;

/// Capability and limit policy applied by the security gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Require a human approval gate for high/critical-risk actions.
    pub require_human_approval: bool,
    /// Allow the action to reach the external network.
    pub allow_external_network: bool,
    /// Allow exporting files out of the environment.
    pub allow_file_export: bool,
    /// Allow writes to databases.
    pub allow_database_write: bool,
    /// Allow access to PII / regulated data.
    pub allow_pii_access: bool,
    /// Highest data sensitivity any action may touch.
    pub max_data_access_level: DataAccessLevel,
    /// Daily spend ceiling, in whole currency units.
    pub budget_limit: f64,
}

impl Default for SecurityPolicy {
    /// A conservative, deny-by-default-ish posture suitable for government use.
    fn default() -> Self {
        SecurityPolicy {
            require_human_approval: true,
            allow_external_network: false,
            allow_file_export: false,
            allow_database_write: false,
            allow_pii_access: false,
            max_data_access_level: DataAccessLevel::Internal,
            budget_limit: 100.0,
        }
    }
}

impl SecurityPolicy {
    /// A permissive policy for trusted, low-risk internal automation.
    pub fn permissive() -> Self {
        SecurityPolicy {
            require_human_approval: false,
            allow_external_network: true,
            allow_file_export: true,
            allow_database_write: true,
            allow_pii_access: true,
            max_data_access_level: DataAccessLevel::Restricted,
            budget_limit: 10_000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_conservative() {
        let p = SecurityPolicy::default();
        assert!(p.require_human_approval);
        assert!(!p.allow_pii_access);
        assert!(!p.allow_external_network);
    }
}
