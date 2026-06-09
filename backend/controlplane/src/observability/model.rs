//! Observability data model: the dashboard metric summary.
//!
//! [`AgentMetrics`] is the aggregated view the Splunk-style dashboard renders.
//! It is computed from raw [`ExecutionEvent`](super::event::ExecutionEvent)s, so
//! the summary is always derivable from the underlying audit-grade event log.

use serde::{Deserialize, Serialize};

/// Aggregated operational metrics for an agent (or the whole fleet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Agent id the metrics belong to, or `"*"` for a fleet-wide summary.
    pub agent_id: String,
    /// Total task executions recorded.
    pub task_count: u64,
    /// Tasks that completed successfully.
    pub successful_tasks: u64,
    /// Tasks that failed.
    pub failed_tasks: u64,
    /// Fraction of tool calls that failed (0.0–1.0).
    pub tool_failure_rate: f64,
    /// Fraction of model calls that failed (0.0–1.0).
    pub model_failure_rate: f64,
    /// Mean task latency in milliseconds.
    pub average_latency_ms: f64,
    /// Mean task cost.
    pub average_cost: f64,
    /// Total task cost.
    pub total_cost: f64,
    /// Number of times a human had to intervene.
    pub human_intervention_count: u64,
    /// Number of executions that waited on approval.
    pub approval_waiting_count: u64,
    /// Number of executions blocked by the security gateway.
    pub blocked_executions: u64,
    /// Number of risk events flagged.
    pub risk_events: u64,
    /// Number of suspected-hallucination flags.
    pub hallucination_flag_count: u64,
    /// Number of MCP server calls made.
    pub mcp_call_count: u64,
}

impl AgentMetrics {
    /// An all-zero summary for the given scope.
    pub fn empty(agent_id: impl Into<String>) -> Self {
        AgentMetrics {
            agent_id: agent_id.into(),
            task_count: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            tool_failure_rate: 0.0,
            model_failure_rate: 0.0,
            average_latency_ms: 0.0,
            average_cost: 0.0,
            total_cost: 0.0,
            human_intervention_count: 0,
            approval_waiting_count: 0,
            blocked_executions: 0,
            risk_events: 0,
            hallucination_flag_count: 0,
            mcp_call_count: 0,
        }
    }

    /// Task success rate (0.0–1.0); 1.0 when there are no tasks.
    pub fn success_rate(&self) -> f64 {
        if self.task_count == 0 {
            1.0
        } else {
            self.successful_tasks as f64 / self.task_count as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_summary_is_zeroed() {
        let m = AgentMetrics::empty("agent-1");
        assert_eq!(m.task_count, 0);
        assert_eq!(m.success_rate(), 1.0);
    }
}
