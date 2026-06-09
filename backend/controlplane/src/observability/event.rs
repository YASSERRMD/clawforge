//! Execution event model — the raw, audit-grade signal stream.
//!
//! Every observable thing an agent does is recorded as an [`ExecutionEvent`].
//! Dashboards ([`AgentMetrics`](super::model::AgentMetrics)) are aggregations
//! over these events, never a separate source of truth.

use serde::{Deserialize, Serialize};

/// Classification of an execution event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A unit of agent work (carries success, latency, cost).
    Task,
    /// A tool invocation (carries `name` and success).
    ToolCall,
    /// An MCP server call (carries `name` and success).
    McpCall,
    /// A model call (carries `name` and success).
    ModelCall,
    /// A flagged risk event.
    RiskEvent,
    /// A suspected hallucination flag.
    Hallucination,
    /// A human had to intervene.
    HumanIntervention,
    /// An execution waited on approval.
    ApprovalWait,
    /// An execution was blocked by the security gateway.
    Blocked,
}

/// A single recorded execution event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    /// Stable unique identifier (UUID v4).
    pub id: String,
    /// Agent the event belongs to.
    pub agent_id: String,
    /// What kind of event this is.
    pub kind: EventKind,
    /// Optional subject name (tool / mcp server / model).
    pub name: Option<String>,
    /// Outcome, for events that have one.
    pub success: Option<bool>,
    /// Latency in milliseconds, for timed events.
    pub latency_ms: Option<i64>,
    /// Cost incurred, for billable events.
    pub cost: Option<f64>,
    /// Free-form structured detail.
    pub detail: serde_json::Value,
    /// Event time (unix seconds).
    pub timestamp: i64,
}

/// Input used to log a new event; the store assigns id and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewExecutionEvent {
    pub agent_id: String,
    pub kind: EventKind,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub success: Option<bool>,
    #[serde(default)]
    pub latency_ms: Option<i64>,
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub detail: serde_json::Value,
}

impl NewExecutionEvent {
    /// Convenience constructor for a completed task event.
    pub fn task(agent_id: impl Into<String>, success: bool, latency_ms: i64, cost: f64) -> Self {
        NewExecutionEvent {
            agent_id: agent_id.into(),
            kind: EventKind::Task,
            name: None,
            success: Some(success),
            latency_ms: Some(latency_ms),
            cost: Some(cost),
            detail: serde_json::Value::Null,
        }
    }
}
