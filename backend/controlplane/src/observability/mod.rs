//! Observability — execution events and the metrics dashboards derive from them.

pub mod event;
pub mod model;

pub use event::{EventKind, ExecutionEvent, NewExecutionEvent};
pub use model::AgentMetrics;
