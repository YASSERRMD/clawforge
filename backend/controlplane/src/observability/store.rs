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

    #[test]
    fn log_and_count_events() {
        let store = ObservabilityStore::in_memory().unwrap();
        store.log_event(NewExecutionEvent::task("agent-1", true, 120, 0.02)).unwrap();
        store
            .log_event(NewExecutionEvent {
                agent_id: "agent-2".into(),
                kind: EventKind::ToolCall,
                name: Some("search".into()),
                success: Some(true),
                latency_ms: None,
                cost: None,
                detail: serde_json::Value::Null,
            })
            .unwrap();
        assert_eq!(store.event_count(None).unwrap(), 2);
        assert_eq!(store.event_count(Some("agent-1")).unwrap(), 1);
    }
}
