//! Government Compliance Pack — PII classification, retention, approval chains,
//! audit evidence, investigation mode, export control, and reporting.

pub mod model;

pub use model::{ApprovalChain, ApprovalStep, AuditEvidence, CompliancePolicy, PiiClassification};
