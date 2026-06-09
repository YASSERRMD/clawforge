//! Environment-driven configuration for the control plane.
//!
//! Configuration is intentionally light: the control plane is embedded in the
//! wider ClawForge workspace, so it only owns the settings that are specific to
//! governance, storage, and audit. Everything is read from environment variables
//! with sensible local-first defaults, mirroring the rest of the workspace.

use std::env;

use anyhow::Result;

/// Resolved control-plane configuration.
#[derive(Debug, Clone)]
pub struct ControlPlaneConfig {
    /// Path to the SQLite database backing registries, governance, and audit.
    pub database_path: String,
    /// Deployment environment label (e.g. `local`, `staging`, `gov-prod`).
    pub environment: String,
    /// Owning organisation / tenant name, surfaced in audit records.
    pub organization: String,
    /// Whether human approval is mandatory for high-risk actions.
    pub require_human_approval: bool,
    /// Default per-agent daily spend ceiling, in whole currency units.
    pub default_budget_limit: f64,
}

impl Default for ControlPlaneConfig {
    fn default() -> Self {
        Self {
            database_path: "clawforge-controlplane.db".to_string(),
            environment: "local".to_string(),
            organization: "ClawForge".to_string(),
            require_human_approval: true,
            default_budget_limit: 100.0,
        }
    }
}

impl ControlPlaneConfig {
    /// Build a config from process environment, falling back to defaults.
    ///
    /// Recognised variables:
    /// - `CLAWFORGE_CP_DB`
    /// - `CLAWFORGE_CP_ENV`
    /// - `CLAWFORGE_CP_ORG`
    /// - `CLAWFORGE_CP_REQUIRE_APPROVAL` (`true`/`false`/`1`/`0`)
    /// - `CLAWFORGE_CP_BUDGET_LIMIT`
    pub fn from_env() -> Result<Self> {
        let default = Self::default();
        Ok(Self {
            database_path: env::var("CLAWFORGE_CP_DB").unwrap_or(default.database_path),
            environment: env::var("CLAWFORGE_CP_ENV").unwrap_or(default.environment),
            organization: env::var("CLAWFORGE_CP_ORG").unwrap_or(default.organization),
            require_human_approval: env::var("CLAWFORGE_CP_REQUIRE_APPROVAL")
                .map(|v| parse_bool(&v))
                .unwrap_or(default.require_human_approval),
            default_budget_limit: env::var("CLAWFORGE_CP_BUDGET_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.default_budget_limit),
        })
    }
}

fn parse_bool(value: &str) -> bool {
    matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_local_first() {
        let cfg = ControlPlaneConfig::default();
        assert_eq!(cfg.environment, "local");
        assert!(cfg.require_human_approval);
        assert!(cfg.default_budget_limit > 0.0);
    }

    #[test]
    fn parse_bool_accepts_common_truthy_values() {
        assert!(parse_bool("true"));
        assert!(parse_bool("1"));
        assert!(parse_bool(" ON "));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("0"));
    }
}
