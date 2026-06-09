//! Security decision response.
//!
//! A [`SecurityDecision`] is the gateway's verdict for an [`ActionRequest`]:
//! whether it is allowed, every reason it would be denied, and a risk score.

use serde::{Deserialize, Serialize};

/// The gateway's verdict for a single action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityDecision {
    /// Whether the action may proceed.
    pub allowed: bool,
    /// All reasons the action was denied (empty when allowed).
    pub denials: Vec<String>,
    /// Aggregate risk score for the action (0 = none).
    pub risk_score: u32,
    /// Evaluation time (unix seconds).
    pub evaluated_at: i64,
}

impl SecurityDecision {
    /// Build a decision from collected denials and a risk score.
    pub fn new(denials: Vec<String>, risk_score: u32, evaluated_at: i64) -> Self {
        SecurityDecision {
            allowed: denials.is_empty(),
            denials,
            risk_score,
            evaluated_at,
        }
    }
}
