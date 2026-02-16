pub mod channel;
pub mod error;
pub mod event;
pub mod message;
pub mod traits;
pub mod types;

pub use channel::ClawBus;
pub use error::ClawError;
pub use event::{Event, EventKind};
pub use message::{
    ActionProposal, AuditEventPayload, JobTrigger, Message, PlanRequest, ProposedAction,
};
pub use traits::{Component, LlmProvider, LlmRequest, LlmResponse};
pub use types::{
    ActionType, AgentSpec, Capabilities, FailurePolicy, LlmPolicy, TriggerSpec, WorkflowStep,
};
