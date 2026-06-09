//! SQLite-backed store for execution events and metric aggregation.
//!
//! Events are append-only. Metrics are computed on demand with SQL aggregates,
//! so the dashboard is always consistent with the underlying event log.

use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::Result;

use super::event::{EventKind, ExecutionEvent, NewExecutionEvent};
use super::model::AgentMetrics;

/// Store of execution events; the source for all observability metrics.
pub struct ObservabilityStore {
    pub(crate) conn: Mutex<Connection>,
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS execution_events (
        id         TEXT PRIMARY KEY,
        agent_id   TEXT NOT NULL,
        kind       TEXT NOT NULL,
        name       TEXT,
        success    INTEGER,
        latency_ms INTEGER,
        cost       REAL,
        detail     TEXT NOT NULL,
        timestamp  INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_events_agent ON execution_events(agent_id);
    CREATE INDEX IF NOT EXISTS idx_events_kind ON execution_events(kind);
";

impl ObservabilityStore {
    /// Open (creating if needed) a store backed by a file.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(&format!("PRAGMA journal_mode=WAL;{SCHEMA}"))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open an ephemeral in-memory store (used by tests).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Record a new execution event, returning the persisted record.
    pub fn log_event(&self, input: NewExecutionEvent) -> Result<ExecutionEvent> {
        let event = ExecutionEvent {
            id: Uuid::new_v4().to_string(),
            agent_id: input.agent_id,
            kind: input.kind,
            name: input.name,
            success: input.success,
            latency_ms: input.latency_ms,
            cost: input.cost,
            detail: input.detail,
            timestamp: Utc::now().timestamp(),
        };
        let conn = self.conn.lock().expect("observability mutex poisoned");
        conn.execute(
            "INSERT INTO execution_events
                (id, agent_id, kind, name, success, latency_ms, cost, detail, timestamp)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                event.id,
                event.agent_id,
                serde_json::to_string(&event.kind)?,
                event.name,
                event.success,
                event.latency_ms,
                event.cost,
                serde_json::to_string(&event.detail)?,
                event.timestamp,
            ],
        )?;
        Ok(event)
    }

    /// Build the full dashboard summary for one agent, or the whole fleet when
    /// `agent` is `None`.
    pub fn summary(&self, agent: Option<&str>) -> Result<AgentMetrics> {
        Ok(AgentMetrics {
            agent_id: agent.unwrap_or("*").to_string(),
            task_count: self.task_count(agent)?,
            successful_tasks: self.successful_tasks(agent)?,
            failed_tasks: self.failed_tasks(agent)?,
            tool_failure_rate: self.tool_failure_rate(agent)?,
            model_failure_rate: self.model_failure_rate(agent)?,
            average_latency_ms: self.average_latency_ms(agent)?,
            average_cost: self.average_cost(agent)?,
            total_cost: self.total_cost(agent)?,
            human_intervention_count: self.human_intervention_count(agent)?,
            approval_waiting_count: self.approval_waiting_count(agent)?,
            blocked_executions: self.blocked_count(agent)?,
            risk_events: self.risk_event_count(agent)?,
            hallucination_flag_count: self.hallucination_count(agent)?,
            mcp_call_count: self.mcp_call_count(agent)?,
        })
    }

    /// Count events of a kind (optionally scoped to one agent).
    fn count_kind(&self, agent: Option<&str>, kind: EventKind) -> Result<u64> {
        let k = serde_json::to_string(&kind)?;
        let conn = self.conn.lock().expect("observability mutex poisoned");
        let n: i64 = match agent {
            Some(a) => conn.query_row(
                "SELECT COUNT(*) FROM execution_events WHERE kind = ?1 AND agent_id = ?2",
                params![k, a],
                |r| r.get(0),
            )?,
            None => conn.query_row(
                "SELECT COUNT(*) FROM execution_events WHERE kind = ?1",
                params![k],
                |r| r.get(0),
            )?,
        };
        Ok(n as u64)
    }

    /// Count events of a kind with a given outcome (optionally scoped).
    fn count_kind_success(&self, agent: Option<&str>, kind: EventKind, success: bool) -> Result<u64> {
        let k = serde_json::to_string(&kind)?;
        let s = success as i64;
        let conn = self.conn.lock().expect("observability mutex poisoned");
        let n: i64 = match agent {
            Some(a) => conn.query_row(
                "SELECT COUNT(*) FROM execution_events WHERE kind = ?1 AND success = ?2 AND agent_id = ?3",
                params![k, s, a],
                |r| r.get(0),
            )?,
            None => conn.query_row(
                "SELECT COUNT(*) FROM execution_events WHERE kind = ?1 AND success = ?2",
                params![k, s],
                |r| r.get(0),
            )?,
        };
        Ok(n as u64)
    }

    /// Number of successful task executions.
    pub fn successful_tasks(&self, agent: Option<&str>) -> Result<u64> {
        self.count_kind_success(agent, EventKind::Task, true)
    }

    /// Number of failed task executions.
    pub fn failed_tasks(&self, agent: Option<&str>) -> Result<u64> {
        self.count_kind_success(agent, EventKind::Task, false)
    }

    /// Total number of task executions (successful + failed).
    pub fn task_count(&self, agent: Option<&str>) -> Result<u64> {
        self.count_kind(agent, EventKind::Task)
    }

    /// Mean task latency in milliseconds (0.0 when no timed tasks).
    pub fn average_latency_ms(&self, agent: Option<&str>) -> Result<f64> {
        self.task_aggregate("AVG(latency_ms)", agent)
    }

    /// Run a scalar aggregate (`AVG`/`SUM`) over task events (internal helper).
    fn task_aggregate(&self, expr: &str, agent: Option<&str>) -> Result<f64> {
        let kind = serde_json::to_string(&EventKind::Task)?;
        let conn = self.conn.lock().expect("observability mutex poisoned");
        let sql_all = format!("SELECT COALESCE({expr}, 0.0) FROM execution_events WHERE kind = ?1");
        let sql_agent =
            format!("SELECT COALESCE({expr}, 0.0) FROM execution_events WHERE kind = ?1 AND agent_id = ?2");
        let v: f64 = match agent {
            Some(a) => conn.query_row(&sql_agent, params![kind, a], |r| r.get(0))?,
            None => conn.query_row(&sql_all, params![kind], |r| r.get(0))?,
        };
        Ok(v)
    }

    /// Mean task cost (0.0 when no tasks).
    pub fn average_cost(&self, agent: Option<&str>) -> Result<f64> {
        self.task_aggregate("AVG(cost)", agent)
    }

    /// Total task cost across all recorded tasks.
    pub fn total_cost(&self, agent: Option<&str>) -> Result<f64> {
        self.task_aggregate("SUM(cost)", agent)
    }

    /// Failure rate (0.0–1.0) for a call kind; 0.0 when there are no calls.
    pub fn failure_rate(&self, agent: Option<&str>, kind: EventKind) -> Result<f64> {
        let total = self.count_kind(agent, kind)?;
        if total == 0 {
            return Ok(0.0);
        }
        let failed = self.count_kind_success(agent, kind, false)?;
        Ok(failed as f64 / total as f64)
    }

    /// Fraction of tool calls that failed.
    pub fn tool_failure_rate(&self, agent: Option<&str>) -> Result<f64> {
        self.failure_rate(agent, EventKind::ToolCall)
    }

    /// Fraction of model calls that failed.
    pub fn model_failure_rate(&self, agent: Option<&str>) -> Result<f64> {
        self.failure_rate(agent, EventKind::ModelCall)
    }

    /// Number of MCP server calls recorded.
    pub fn mcp_call_count(&self, agent: Option<&str>) -> Result<u64> {
        self.count_kind(agent, EventKind::McpCall)
    }

    /// Number of risk events flagged.
    pub fn risk_event_count(&self, agent: Option<&str>) -> Result<u64> {
        self.count_kind(agent, EventKind::RiskEvent)
    }

    /// Number of suspected-hallucination flags.
    pub fn hallucination_count(&self, agent: Option<&str>) -> Result<u64> {
        self.count_kind(agent, EventKind::Hallucination)
    }

    /// Number of human interventions.
    pub fn human_intervention_count(&self, agent: Option<&str>) -> Result<u64> {
        self.count_kind(agent, EventKind::HumanIntervention)
    }

    /// Number of executions that waited on approval.
    pub fn approval_waiting_count(&self, agent: Option<&str>) -> Result<u64> {
        self.count_kind(agent, EventKind::ApprovalWait)
    }

    /// Number of executions blocked by the security gateway.
    pub fn blocked_count(&self, agent: Option<&str>) -> Result<u64> {
        self.count_kind(agent, EventKind::Blocked)
    }

    /// Total number of events recorded (optionally scoped to one agent).
    pub fn event_count(&self, agent: Option<&str>) -> Result<u64> {
        let conn = self.conn.lock().expect("observability mutex poisoned");
        let n: i64 = match agent {
            Some(a) => conn.query_row(
                "SELECT COUNT(*) FROM execution_events WHERE agent_id = ?1",
                params![a],
                |r| r.get(0),
            )?,
            None => conn.query_row("SELECT COUNT(*) FROM execution_events", [], |r| r.get(0))?,
        };
        Ok(n as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(agent: &str, ok: bool) -> NewExecutionEvent {
        NewExecutionEvent {
            agent_id: agent.into(),
            kind: EventKind::ToolCall,
            name: Some("search".into()),
            success: Some(ok),
            latency_ms: None,
            cost: None,
            detail: serde_json::Value::Null,
        }
    }

    #[test]
    fn log_and_count_events() {
        let store = ObservabilityStore::in_memory().unwrap();
        store.log_event(NewExecutionEvent::task("agent-1", true, 120, 0.02)).unwrap();
        store.log_event(tool("agent-2", true)).unwrap();
        assert_eq!(store.event_count(None).unwrap(), 2);
        assert_eq!(store.event_count(Some("agent-1")).unwrap(), 1);
    }

    #[test]
    fn task_metrics_compute() {
        let store = ObservabilityStore::in_memory().unwrap();
        store.log_event(NewExecutionEvent::task("a", true, 100, 0.10)).unwrap();
        store.log_event(NewExecutionEvent::task("a", true, 300, 0.30)).unwrap();
        store.log_event(NewExecutionEvent::task("a", false, 200, 0.20)).unwrap();
        assert_eq!(store.task_count(Some("a")).unwrap(), 3);
        assert_eq!(store.successful_tasks(Some("a")).unwrap(), 2);
        assert_eq!(store.failed_tasks(Some("a")).unwrap(), 1);
        assert_eq!(store.average_latency_ms(Some("a")).unwrap(), 200.0);
        assert!((store.total_cost(Some("a")).unwrap() - 0.60).abs() < 1e-9);
        assert!((store.average_cost(Some("a")).unwrap() - 0.20).abs() < 1e-9);
    }

    #[test]
    fn failure_rates_compute() {
        let store = ObservabilityStore::in_memory().unwrap();
        store.log_event(tool("a", true)).unwrap();
        store.log_event(tool("a", true)).unwrap();
        store.log_event(tool("a", true)).unwrap();
        store.log_event(tool("a", false)).unwrap();
        assert!((store.tool_failure_rate(Some("a")).unwrap() - 0.25).abs() < 1e-9);
        // No model calls => rate is 0.0, not NaN.
        assert_eq!(store.model_failure_rate(Some("a")).unwrap(), 0.0);
    }

    #[test]
    fn empty_scope_summary_is_safe() {
        let store = ObservabilityStore::in_memory().unwrap();
        let s = store.summary(Some("nobody")).unwrap();
        assert_eq!(s.task_count, 0);
        assert_eq!(s.tool_failure_rate, 0.0);
        assert_eq!(s.agent_id, "nobody");
    }

    #[test]
    fn fleet_summary_aggregates_all_agents() {
        let store = ObservabilityStore::in_memory().unwrap();
        store.log_event(NewExecutionEvent::task("a", true, 100, 0.1)).unwrap();
        store.log_event(NewExecutionEvent::task("b", false, 100, 0.1)).unwrap();
        let fleet = store.summary(None).unwrap();
        assert_eq!(fleet.agent_id, "*");
        assert_eq!(fleet.task_count, 2);
        assert_eq!(fleet.successful_tasks, 1);
        assert_eq!(fleet.failed_tasks, 1);
    }
}
