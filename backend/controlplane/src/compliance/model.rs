//! Government compliance domain model.
//!
//! The compliance pack expresses, per subject (an agent or a department), the
//! regulatory posture ClawForge enforces: data-protection framework, PII
//! handling, retention, export control, and investigation state. It is
//! deliberately framework-aware (e.g. UAE PDPL) without hard-coding any single
//! jurisdiction.

use serde::{Deserialize, Serialize};

/// A compliance policy applied to a subject (agent id or department name).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompliancePolicy {
    /// The agent id or department this policy governs.
    pub subject_id: String,
    /// Regulatory framework reference (e.g. `UAE-PDPL`).
    pub framework: String,
}

impl CompliancePolicy {
    /// A baseline policy for a subject under the given framework.
    pub fn new(subject_id: impl Into<String>, framework: impl Into<String>) -> Self {
        CompliancePolicy {
            subject_id: subject_id.into(),
            framework: framework.into(),
        }
    }

    /// A baseline UAE PDPL policy for a subject.
    pub fn pdpl(subject_id: impl Into<String>) -> Self {
        Self::new(subject_id, "UAE-PDPL")
    }
}
