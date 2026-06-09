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
