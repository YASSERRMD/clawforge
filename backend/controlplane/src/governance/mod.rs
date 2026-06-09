//! Governance Engine — approval workflow with a human gate and change history.

pub mod model;
pub mod store;

pub use model::{ApprovalKind, ApprovalRequest, ApprovalStatus, NewApprovalRequest};
pub use store::GovernanceEngine;
