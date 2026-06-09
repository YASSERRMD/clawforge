//! Unified error type for all control-plane operations.
//!
//! Domain modules return [`Result<T>`] so that storage failures, validation
//! errors, governance denials, and not-found conditions are represented with a
//! single, matchable error enum rather than opaque strings.

use thiserror::Error;

/// Convenience alias used throughout the control plane.
pub type Result<T> = std::result::Result<T, ControlPlaneError>;

/// Errors raised by control-plane domain operations.
#[derive(Debug, Error)]
pub enum ControlPlaneError {
    /// A requested entity does not exist.
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },

    /// Input failed a domain validation rule.
    #[error("validation failed: {0}")]
    Validation(String),

    /// An action was denied by governance or the security gateway.
    #[error("denied: {0}")]
    Denied(String),

    /// A conflicting state prevented the operation (e.g. duplicate id).
    #[error("conflict: {0}")]
    Conflict(String),

    /// Underlying storage error.
    #[error("storage error: {0}")]
    Storage(String),

    /// Serialization / deserialization error.
    #[error("serialization error: {0}")]
    Serialization(String),
}

impl ControlPlaneError {
    /// Build a [`ControlPlaneError::NotFound`] for the given entity and id.
    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        ControlPlaneError::NotFound {
            entity,
            id: id.into(),
        }
    }

    /// Build a validation error.
    pub fn validation(msg: impl Into<String>) -> Self {
        ControlPlaneError::Validation(msg.into())
    }

    /// Build a denial error.
    pub fn denied(msg: impl Into<String>) -> Self {
        ControlPlaneError::Denied(msg.into())
    }
}

impl From<rusqlite::Error> for ControlPlaneError {
    fn from(e: rusqlite::Error) -> Self {
        ControlPlaneError::Storage(e.to_string())
    }
}

impl From<serde_json::Error> for ControlPlaneError {
    fn from(e: serde_json::Error) -> Self {
        ControlPlaneError::Serialization(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_formats_entity_and_id() {
        let e = ControlPlaneError::not_found("agent", "abc-123");
        assert_eq!(e.to_string(), "agent not found: abc-123");
    }

    #[test]
    fn denial_carries_reason() {
        let e = ControlPlaneError::denied("tool not allowed");
        assert!(e.to_string().contains("tool not allowed"));
    }
}
