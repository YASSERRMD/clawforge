//! Government compliance domain model.
//!
//! The compliance pack expresses, per subject (an agent or a department), the
//! regulatory posture ClawForge enforces: data-protection framework, PII
//! handling, retention, export control, and investigation state. It is
//! deliberately framework-aware (e.g. UAE PDPL) without hard-coding any single
//! jurisdiction.

use serde::{Deserialize, Serialize};

/// PII handling classification for a subject's data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiClassification {
    /// No personal data handled.
    NonPii,
    /// Ordinary personal data.
    Pii,
    /// Special-category / sensitive personal data (health, biometric, etc.).
    SensitivePii,
}

impl PiiClassification {
    /// Whether this classification triggers heightened PDPL controls.
    pub fn is_regulated(&self) -> bool {
        !matches!(self, PiiClassification::NonPii)
    }
}

/// A compliance policy applied to a subject (agent id or department name).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompliancePolicy {
    /// The agent id or department this policy governs.
    pub subject_id: String,
    /// Regulatory framework reference (e.g. `UAE-PDPL`).
    pub framework: String,
    /// PII handling classification.
    #[serde(default = "default_pii")]
    pub pii_classification: PiiClassification,
    /// Data retention period in days; `0` means "retain indefinitely".
    #[serde(default)]
    pub data_retention_days: u32,
}

fn default_pii() -> PiiClassification {
    PiiClassification::NonPii
}

impl CompliancePolicy {
    /// A baseline policy for a subject under the given framework.
    pub fn new(subject_id: impl Into<String>, framework: impl Into<String>) -> Self {
        CompliancePolicy {
            subject_id: subject_id.into(),
            framework: framework.into(),
            pii_classification: PiiClassification::NonPii,
            data_retention_days: 0,
        }
    }

    /// Whether a record `age_days` old is past this policy's retention window.
    /// Indefinite retention (`0`) is never past due.
    pub fn is_past_retention(&self, age_days: u32) -> bool {
        self.data_retention_days != 0 && age_days > self.data_retention_days
    }

    /// A baseline UAE PDPL policy for a subject.
    pub fn pdpl(subject_id: impl Into<String>) -> Self {
        Self::new(subject_id, "UAE-PDPL")
    }
}
