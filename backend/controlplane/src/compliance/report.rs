//! Compliance report generation.
//!
//! A [`ComplianceReport`] is computed from a subject's [`CompliancePolicy`],
//! collected [`AuditEvidence`], and (optionally) its [`ApprovalChain`]. It is a
//! pure function of those inputs — assembled from the other control-plane
//! stores — so a report is always reproducible from the evidence on file.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::model::{ApprovalChain, AuditEvidence, CompliancePolicy, ExportControl, PiiClassification};

/// A point-in-time compliance assessment for a single subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub subject_id: String,
    pub framework: String,
    pub pii_classification: PiiClassification,
    pub data_retention_days: u32,
    pub export_control: ExportControl,
    pub investigation_mode: bool,
    /// Total audit-evidence records on file.
    pub evidence_count: usize,
    /// Evidence records that are digitally signed.
    pub signed_evidence_count: usize,
    /// Whether the approval chain (if any) is fully signed off.
    pub approval_complete: bool,
    /// Findings that require attention; empty means compliant.
    pub findings: Vec<String>,
    pub generated_at: i64,
}

impl ComplianceReport {
    /// Generate a report for a subject from its policy, evidence, and chain.
    pub fn generate(
        policy: &CompliancePolicy,
        evidence: &[AuditEvidence],
        chain: Option<&ApprovalChain>,
    ) -> Self {
        let signed = evidence.iter().filter(|e| e.is_signed()).count();
        let approval_complete = chain.map(|c| c.is_complete()).unwrap_or(true);

        let mut findings = Vec::new();
        if policy.pii_classification.is_regulated() && evidence.is_empty() {
            findings.push("regulated PII handled but no audit evidence collected".into());
        }
        if policy.pii_classification == PiiClassification::SensitivePii && !approval_complete {
            findings.push("sensitive PII without a completed approval chain".into());
        }
        if policy.data_retention_days == 0 && policy.pii_classification.is_regulated() {
            findings.push("regulated PII has no defined retention period".into());
        }
        if !approval_complete {
            findings.push("approval chain is incomplete".into());
        }
        if matches!(policy.export_control, ExportControl::Prohibited) && signed == 0 && !evidence.is_empty() {
            findings.push("export-prohibited data lacks signed evidence".into());
        }

        ComplianceReport {
            subject_id: policy.subject_id.clone(),
            framework: policy.framework.clone(),
            pii_classification: policy.pii_classification,
            data_retention_days: policy.data_retention_days,
            export_control: policy.export_control,
            investigation_mode: policy.investigation_mode,
            evidence_count: evidence.len(),
            signed_evidence_count: signed,
            approval_complete,
            findings,
            generated_at: Utc::now().timestamp(),
        }
    }

    /// Whether the subject is compliant (no outstanding findings).
    pub fn is_compliant(&self) -> bool {
        self.findings.is_empty()
    }
}

/// A department-level roll-up of per-subject [`ComplianceReport`]s.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentComplianceSummary {
    pub department: String,
    /// Number of subjects assessed.
    pub subject_count: usize,
    /// Subjects with no outstanding findings.
    pub compliant_count: usize,
    /// Subjects with one or more findings.
    pub non_compliant_count: usize,
    /// Subjects currently under investigation.
    pub under_investigation: usize,
    /// Every distinct finding across the department.
    pub findings: Vec<String>,
    pub generated_at: i64,
}

impl DepartmentComplianceSummary {
    /// Roll up a set of reports into a single department summary.
    pub fn summarize(department: impl Into<String>, reports: &[ComplianceReport]) -> Self {
        let compliant = reports.iter().filter(|r| r.is_compliant()).count();
        let under_investigation = reports.iter().filter(|r| r.investigation_mode).count();
        let mut findings: Vec<String> = reports
            .iter()
            .flat_map(|r| r.findings.iter().map(|f| format!("{}: {}", r.subject_id, f)))
            .collect();
        findings.sort();
        DepartmentComplianceSummary {
            department: department.into(),
            subject_count: reports.len(),
            compliant_count: compliant,
            non_compliant_count: reports.len() - compliant,
            under_investigation,
            findings,
            generated_at: Utc::now().timestamp(),
        }
    }

    /// Compliance rate (0.0–1.0); 1.0 when there are no subjects.
    pub fn compliance_rate(&self) -> f64 {
        if self.subject_count == 0 {
            1.0
        } else {
            self.compliant_count as f64 / self.subject_count as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_non_pii_subject_is_compliant() {
        let policy = CompliancePolicy::pdpl("dept-it");
        let report = ComplianceReport::generate(&policy, &[], None);
        assert!(report.is_compliant());
    }

    #[test]
    fn regulated_pii_without_evidence_has_findings() {
        let mut policy = CompliancePolicy::pdpl("agent-1");
        policy.pii_classification = PiiClassification::Pii;
        let report = ComplianceReport::generate(&policy, &[], None);
        assert!(!report.is_compliant());
        assert!(report.findings.iter().any(|f| f.contains("no audit evidence")));
    }

    #[test]
    fn incomplete_chain_is_flagged() {
        let mut policy = CompliancePolicy::pdpl("agent-1");
        policy.pii_classification = PiiClassification::SensitivePii;
        policy.data_retention_days = 365;
        let chain = ApprovalChain::from_roles("agent-1", &["data-owner", "dpo"]);
        let report = ComplianceReport::generate(&policy, &[], Some(&chain));
        assert!(!report.approval_complete);
        assert!(!report.is_compliant());
    }
}
