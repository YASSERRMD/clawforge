# Observability

The observability layer (`clawforge_controlplane::observability`) is the
**Splunk** of the control plane. Agents emit append-only `ExecutionEvent`s; the
dashboard `AgentMetrics` summary is computed on demand from those events, so the
numbers are always reconcilable with the raw log.

## Events

Every observable action is one `ExecutionEvent` with an `EventKind`:

| Kind | Carries | Feeds |
|------|---------|-------|
| `task` | success, latency_ms, cost | task counts, latency, cost |
| `tool_call` | name, success | tool failure rate |
| `model_call` | name, success | model failure rate |
| `mcp_call` | name, success | MCP call count |
| `risk_event` | — | risk events |
| `hallucination` | — | hallucination flag count |
| `human_intervention` | — | intervention count |
| `approval_wait` | — | approval waiting count |
| `blocked` | — | blocked executions |

## Metrics (`AgentMetrics`)

`task_count`, `successful_tasks`, `failed_tasks`, `tool_failure_rate`,
`model_failure_rate`, `average_latency_ms`, `average_cost`, `total_cost`,
`human_intervention_count`, `approval_waiting_count`, `blocked_executions`,
`risk_events`, `hallucination_flag_count`, `mcp_call_count` — plus a derived
`success_rate()`.

## API

```rust
use clawforge_controlplane::observability::{ObservabilityStore, NewExecutionEvent, EventKind};

let obs = ObservabilityStore::open("clawforge-controlplane.db")?;

// Log events as the runtime executes.
obs.log_event(NewExecutionEvent::task("agent-1", true, 120, 0.02))?;
obs.log_event(NewExecutionEvent {
    agent_id: "agent-1".into(),
    kind: EventKind::ToolCall,
    name: Some("search".into()),
    success: Some(false),
    latency_ms: Some(80),
    cost: None,
    detail: serde_json::json!({ "error": "timeout" }),
})?;

// Per-agent dashboard summary…
let agent = obs.summary(Some("agent-1"))?;
// …or fleet-wide (agent_id == "*").
let fleet = obs.summary(None)?;
```

## Scoping

Every metric method takes `agent: Option<&str>`: `Some(id)` scopes to one agent,
`None` aggregates across the whole fleet. Rates return `0.0` (never `NaN`) when
there are no underlying calls.

## Seed data

`observability::seed::seed_for_agent(&store, agent_id)` logs a realistic spread
of events (10 tasks, tool/MCP/model calls, and one of each governance signal)
for demos.
