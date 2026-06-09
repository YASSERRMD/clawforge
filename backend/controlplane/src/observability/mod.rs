//! Observability — execution events and the metrics dashboards derive from them.

pub mod event;
pub mod model;
pub mod seed;
pub mod store;

pub use event::{EventKind, ExecutionEvent, NewExecutionEvent};
pub use model::AgentMetrics;
pub use seed::seed_for_agent;
pub use store::ObservabilityStore;
