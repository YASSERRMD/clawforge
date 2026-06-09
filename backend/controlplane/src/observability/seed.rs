//! Sample observability data for demos and local development.
//!
//! Generates a realistic mix of task, tool, MCP, risk, and intervention events
//! so the dashboard summary has something meaningful to show out of the box.

use crate::error::Result;

use super::event::{EventKind, NewExecutionEvent};
use super::store::ObservabilityStore;

/// Log a representative spread of events for a single agent.
pub fn seed_for_agent(store: &ObservabilityStore, agent_id: &str) -> Result<()> {
    // Tasks: mostly successful, a couple failed.
    for i in 0..8 {
        store.log_event(NewExecutionEvent::task(agent_id, true, 100 + i * 10, 0.01 + i as f64 * 0.002))?;
    }
    store.log_event(NewExecutionEvent::task(agent_id, false, 450, 0.03))?;
    store.log_event(NewExecutionEvent::task(agent_id, false, 500, 0.04))?;

    // Tool calls (one failure), MCP calls, a model call.
    for ok in [true, true, true, false] {
        store.log_event(call(agent_id, EventKind::ToolCall, "search", ok))?;
    }
    for _ in 0..3 {
        store.log_event(call(agent_id, EventKind::McpCall, "records-mcp", true))?;
    }
    store.log_event(call(agent_id, EventKind::ModelCall, "claude-opus-4-8", true))?;

    // Governance / risk signals.
    store.log_event(event(agent_id, EventKind::RiskEvent))?;
    store.log_event(event(agent_id, EventKind::Hallucination))?;
    store.log_event(event(agent_id, EventKind::HumanIntervention))?;
    store.log_event(event(agent_id, EventKind::ApprovalWait))?;
    store.log_event(event(agent_id, EventKind::Blocked))?;
    Ok(())
}

fn call(agent_id: &str, kind: EventKind, name: &str, success: bool) -> NewExecutionEvent {
    NewExecutionEvent {
        agent_id: agent_id.into(),
        kind,
        name: Some(name.into()),
        success: Some(success),
        latency_ms: Some(60),
        cost: None,
        detail: serde_json::Value::Null,
    }
}

fn event(agent_id: &str, kind: EventKind) -> NewExecutionEvent {
    NewExecutionEvent {
        agent_id: agent_id.into(),
        kind,
        name: None,
        success: None,
        latency_ms: None,
        cost: None,
        detail: serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_produces_events() {
        let store = ObservabilityStore::in_memory().unwrap();
        seed_for_agent(&store, "agent-1").unwrap();
        let summary = store.summary(Some("agent-1")).unwrap();
        assert_eq!(summary.task_count, 10);
        assert_eq!(summary.successful_tasks, 8);
        assert_eq!(summary.failed_tasks, 2);
        assert_eq!(summary.mcp_call_count, 3);
        assert_eq!(summary.risk_events, 1);
    }
}
