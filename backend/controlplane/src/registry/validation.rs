//! Validation rules for agent registry inputs.
//!
//! Validation runs before an agent is persisted so the registry never stores a
//! record that governance or the security gateway could not reason about.

use crate::constants::{DataAccessLevel, RiskLevel};
use crate::error::{ControlPlaneError, Result};

use super::model::NewAgent;

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
    if input.data_access_level == DataAccessLevel::Restricted && input.risk_level == RiskLevel::Low {
        return Err(ControlPlaneError::validation(
            "agents with restricted data access must be at least medium risk",
        ));
    }

    Ok(())
}

fn require_non_empty(field: &'static str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(ControlPlaneError::validation(format!("{field} must not be empty")));
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
