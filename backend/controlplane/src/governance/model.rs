//! Governance domain model: what gets approved and by whom.
//!
//! The governance engine sits between "someone wants an agent / tool / MCP
//! server / model to be used" and "it is allowed to run". Every such request is
//! captured as an [`ApprovalRequest`] carrying department ownership, risk level,
//! and a justification, then routed through a human approval gate.

use serde::{Deserialize, Serialize};

use crate::constants::RiskLevel;

/// The kind of subject an approval request concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalKind {
    /// Approval to operate a registered agent.
    Agent,
    /// Approval to allow a tool.
    Tool,
    /// Approval to allow an MCP server.
    Mcp,
    /// Approval to allow a model.
    Model,
}

/// Lifecycle of an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    /// Awaiting a human decision.
    Pending,
    /// Approved by a human gate.
    Approved,
    /// Rejected by a human gate.
    Rejected,
}

impl ApprovalStatus {
    /// Whether a decision has been made (no longer actionable).
    pub fn is_decided(&self) -> bool {
        !matches!(self, ApprovalStatus::Pending)
    }
}

/// Input used to submit a new approval request. The engine assigns the id,
/// status, and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewApprovalRequest {
    /// What is being approved.
    pub kind: ApprovalKind,
    /// Identifier of the subject (e.g. agent id, tool name, model name).
    pub subject_id: String,
    /// Human-friendly subject name.
    pub subject_name: String,
    /// Who is requesting approval.
    pub requested_by: String,
    /// Owning department / business unit.
    pub department: String,
    /// Risk level of the subject.
    pub risk_level: RiskLevel,
    /// Justification for the request (why it should be approved).
    pub justification: String,
}

/// A persisted approval request and its decision state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Stable unique identifier (UUID v4).
    pub id: String,
    /// What is being approved.
    pub kind: ApprovalKind,
    /// Identifier of the subject.
    pub subject_id: String,
    /// Human-friendly subject name.
    pub subject_name: String,
    /// Who requested approval.
    pub requested_by: String,
    /// Owning department.
    pub department: String,
    /// Risk level of the subject.
    pub risk_level: RiskLevel,
    /// Justification supplied at submission.
    pub justification: String,
    /// Current status.
    pub status: ApprovalStatus,
    /// Who made the decision (set once decided).
    pub decided_by: Option<String>,
    /// Reason recorded with the approval/rejection decision.
    pub decision_reason: Option<String>,
    /// Submission time (unix seconds).
    pub created_at: i64,
    /// Decision time (unix seconds), if decided.
    pub decided_at: Option<i64>,
}

impl ApprovalRequest {
    /// Build a fresh `Pending` request from a [`NewApprovalRequest`] input.
    pub fn from_new(input: NewApprovalRequest, id: String, now: i64) -> Self {
        ApprovalRequest {
            id,
            kind: input.kind,
            subject_id: input.subject_id,
            subject_name: input.subject_name,
            requested_by: input.requested_by,
            department: input.department,
            risk_level: input.risk_level,
            justification: input.justification,
            status: ApprovalStatus::Pending,
            decided_by: None,
            decision_reason: None,
            created_at: now,
            decided_at: None,
        }
    }
}
