//! Validation rules for agent registry inputs.
//!
//! Validation runs before an agent is persisted so the registry never stores a
//! record that governance or the security gateway could not reason about.

use crate::constants::{DataAccessLevel, RiskLevel};
use crate::error::{ControlPlaneError, Result};

use super::model::{AgentRecord, NewAgent};

/// Maximum length for free-text fields, to keep records bounded.
const MAX_FIELD_LEN: usize = 200;

/// Validate a [`NewAgent`] input, returning the first failing rule.
pub fn validate_new_agent(input: &NewAgent) -> Result<()> {
    require_non_empty("name", &input.name)?;
    require_non_empty("owner", &input.owner)?;
    require_non_empty("department", &input.department)?;
    require_non_empty("framework", &input.framework)?;
    require_non_empty("model_provider", &input.model_provider)?;
    require_non_empty("model_name", &input.model_name)?;

    bound_len("name", &input.name)?;
    bound_len("owner", &input.owner)?;
    bound_len("department", &input.department)?;

    // A restricted-data agent that is only low risk is almost always a
    // mis-classification; force an explicit higher risk level.
    if input.data_access_level == DataAccessLevel::Restricted && input.risk_level == RiskLevel::Low
    {
        return Err(ControlPlaneError::validation(
            "agents with restricted data access must be at least medium risk",
        ));
    }

    Ok(())
}

/// Validate an existing record after a metadata patch has been applied.
pub fn validate_record(record: &AgentRecord) -> Result<()> {
    require_non_empty("name", &record.name)?;
    require_non_empty("owner", &record.owner)?;
    require_non_empty("department", &record.department)?;
    bound_len("name", &record.name)?;
    bound_len("owner", &record.owner)?;
    bound_len("department", &record.department)?;
    if record.data_access_level == DataAccessLevel::Restricted
        && record.risk_level == RiskLevel::Low
    {
        return Err(ControlPlaneError::validation(
            "agents with restricted data access must be at least medium risk",
        ));
    }
    Ok(())
}

fn require_non_empty(field: &'static str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(ControlPlaneError::validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn bound_len(field: &'static str, value: &str) -> Result<()> {
    if value.len() > MAX_FIELD_LEN {
        return Err(ControlPlaneError::validation(format!(
            "{field} exceeds maximum length of {MAX_FIELD_LEN}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> NewAgent {
        NewAgent {
            name: "Agent".into(),
            description: "desc".into(),
            owner: "team".into(),
            department: "IT".into(),
            framework: "openclaw".into(),
            model_provider: "anthropic".into(),
            model_name: "claude-opus-4-8".into(),
            tools_allowed: vec![],
            mcp_servers_allowed: vec![],
            data_access_level: DataAccessLevel::Internal,
            risk_level: RiskLevel::Low,
        }
    }

    #[test]
    fn valid_input_passes() {
        assert!(validate_new_agent(&valid()).is_ok());
    }

    #[test]
    fn empty_name_rejected() {
        let mut a = valid();
        a.name = "  ".into();
        assert!(validate_new_agent(&a).is_err());
    }

    #[test]
    fn empty_owner_rejected() {
        let mut a = valid();
        a.owner = String::new();
        assert!(validate_new_agent(&a).is_err());
    }

    #[test]
    fn overlong_field_rejected() {
        let mut a = valid();
        a.name = "x".repeat(MAX_FIELD_LEN + 1);
        assert!(validate_new_agent(&a).is_err());
    }

    #[test]
    fn restricted_data_requires_higher_risk() {
        let mut a = valid();
        a.data_access_level = DataAccessLevel::Restricted;
        a.risk_level = RiskLevel::Low;
        assert!(validate_new_agent(&a).is_err());
        a.risk_level = RiskLevel::High;
        assert!(validate_new_agent(&a).is_ok());
    }
}
