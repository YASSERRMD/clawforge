//! Agent Registry — the source of truth for every agent the organisation runs.

pub mod model;
pub mod store;
pub mod validation;

pub use model::{AgentRecord, NewAgent};
pub use store::AgentRegistry;
pub use validation::validate_new_agent;
