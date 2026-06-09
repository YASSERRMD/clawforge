//! Government compliance domain model.
//!
//! The compliance pack expresses, per subject (an agent or a department), the
//! regulatory posture ClawForge enforces: data-protection framework, PII
//! handling, retention, export control, and investigation state. It is
//! deliberately framework-aware (e.g. UAE PDPL) without hard-coding any single
//! jurisdiction.

use serde::{Deserialize, Serialize};

/// Export-control posture: may the subject's data leave the environment?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportControl {
    /// No export restrictions.
    Unrestricted,
    /// Export allowed only with explicit approval.
    Restricted,
    /// Export is forbidden (data must not leave the boundary).
    Prohibited,
}

impl ExportControl {
    /// Whether an unconditional export is permitted.
    pub fn allows_export(&self) -> bool {
        matches!(self, ExportControl::Unrestricted)
    }
}

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

/// A collected piece of audit evidence — a tamper-evident record an
/// investigator or auditor can rely on. The `content_hash` is a digest of the
/// evidence payload; `signature` is a placeholder for a future digital
/// signature over that hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvidence {
    pub id: String,
    /// Subject the evidence concerns (agent id / department).
    pub subject_id: String,
    /// Short evidence kind (e.g. `decision`, `export`, `access`).
    pub kind: String,
    /// Human-readable summary of what happened.
    pub summary: String,
    /// Digest of the evidence payload (hex).
    pub content_hash: String,
    /// Digital signature over `content_hash` (placeholder; empty until signed).
    #[serde(default)]
    pub signature: String,
    pub collected_at: i64,
}

impl AuditEvidence {
    /// Whether this evidence has been digitally signed.
    pub fn is_signed(&self) -> bool {
        !self.signature.is_empty()
    }
}

/// A single step in a multi-party approval chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalStep {
    /// The role responsible for this step (e.g. `data-owner`, `dpo`, `ciso`).
    pub role: String,
    /// Who signed off, once approved.
    pub approver: Option<String>,
    /// Whether this step has been approved.
    pub approved: bool,
    /// Approval time, if approved.
    pub approved_at: Option<i64>,
}

/// An ordered, multi-party approval chain — high-risk government actions often
/// require sign-off from several roles in sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalChain {
    pub subject_id: String,
    pub steps: Vec<ApprovalStep>,
}

impl ApprovalChain {
    /// Build a chain from an ordered list of role names.
    pub fn from_roles(subject_id: impl Into<String>, roles: &[&str]) -> Self {
        ApprovalChain {
            subject_id: subject_id.into(),
            steps: roles
                .iter()
                .map(|r| ApprovalStep {
                    role: (*r).to_string(),
                    approver: None,
                    approved: false,
                    approved_at: None,
                })
                .collect(),
        }
    }

    /// Index of the next step awaiting approval, if any.
    pub fn next_pending(&self) -> Option<usize> {
        self.steps.iter().position(|s| !s.approved)
    }

    /// Whether every step has been approved.
    pub fn is_complete(&self) -> bool {
        self.steps.iter().all(|s| s.approved)
    }

    /// Approve the next pending step in order. Returns the approved step index,
    /// or `None` if the chain was already complete.
    pub fn approve_next(&mut self, approver: impl Into<String>, at: i64) -> Option<usize> {
        let idx = self.next_pending()?;
        let step = &mut self.steps[idx];
        step.approved = true;
        step.approver = Some(approver.into());
        step.approved_at = Some(at);
        Some(idx)
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
    /// When enabled, the subject is under investigation: retention holds apply
    /// (no deletion) and all activity must be captured as audit evidence.
    #[serde(default)]
    pub investigation_mode: bool,
    /// Export-control posture for the subject's data.
    #[serde(default = "default_export")]
    pub export_control: ExportControl,
}

fn default_export() -> ExportControl {
    ExportControl::Restricted
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
            investigation_mode: false,
            export_control: ExportControl::Restricted,
        }
    }

    /// Whether a record `age_days` old is past this policy's retention window.
    /// Indefinite retention (`0`) and active investigation holds are never past
    /// due (a legal hold overrides routine deletion).
    pub fn is_past_retention(&self, age_days: u32) -> bool {
        !self.investigation_mode && self.data_retention_days != 0 && age_days > self.data_retention_days
    }

    /// A baseline UAE PDPL policy for a subject.
    pub fn pdpl(subject_id: impl Into<String>) -> Self {
        Self::new(subject_id, "UAE-PDPL")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdpl_baseline_defaults() {
        let p = CompliancePolicy::pdpl("agent-1");
        assert_eq!(p.framework, "UAE-PDPL");
        assert_eq!(p.pii_classification, PiiClassification::NonPii);
        assert!(!p.investigation_mode);
        assert_eq!(p.export_control, ExportControl::Restricted);
    }

    #[test]
    fn retention_window() {
        let mut p = CompliancePolicy::pdpl("agent-1");
        p.data_retention_days = 30;
        assert!(!p.is_past_retention(10));
        assert!(p.is_past_retention(31));
        // Indefinite retention is never past due.
        p.data_retention_days = 0;
        assert!(!p.is_past_retention(99999));
    }

    #[test]
    fn investigation_mode_holds_records() {
        let mut p = CompliancePolicy::pdpl("agent-1");
        p.data_retention_days = 30;
        p.investigation_mode = true;
        // Legal hold overrides routine deletion.
        assert!(!p.is_past_retention(365));
    }

    #[test]
    fn pii_and_export_helpers() {
        assert!(PiiClassification::SensitivePii.is_regulated());
        assert!(!PiiClassification::NonPii.is_regulated());
        assert!(ExportControl::Unrestricted.allows_export());
        assert!(!ExportControl::Prohibited.allows_export());
    }

    #[test]
    fn approval_chain_progresses_in_order() {
        let mut chain = ApprovalChain::from_roles("agent-1", &["data-owner", "dpo", "ciso"]);
        assert!(!chain.is_complete());
        assert_eq!(chain.approve_next("alice", 1), Some(0));
        assert_eq!(chain.approve_next("bob", 2), Some(1));
        assert_eq!(chain.approve_next("carol", 3), Some(2));
        assert!(chain.is_complete());
        assert_eq!(chain.approve_next("dave", 4), None);
    }
}
