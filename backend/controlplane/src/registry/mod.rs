//! Agent Registry — the source of truth for every agent the organisation runs.

pub mod lifecycle;
pub mod model;
pub mod store;
pub mod validation;

pub use lifecycle::can_transition;
pub use model::{AgentRecord, AgentUpdate, NewAgent};
pub use store::AgentRegistry;
pub use validation::{validate_new_agent, validate_record};
