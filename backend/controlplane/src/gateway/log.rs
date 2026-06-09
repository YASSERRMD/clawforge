//! Blocked execution log.
//!
//! When the gateway denies an action, the attempt is recorded here for the
//! audit trail and for the observability `blocked_executions` metric. The log
//! is append-only and follows the workspace storage pattern.

use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

use super::decision::SecurityDecision;

/// A persisted record of a blocked (denied) action attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedExecution {
    pub id: String,
    pub agent_id: String,
    /// Joined denial reasons.
    pub reasons: String,
    pub risk_score: u32,
    pub at: i64,
}

/// Append-only store of blocked executions.
pub struct BlockedExecutionLog {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS blocked_executions (
        id         TEXT PRIMARY KEY,
        agent_id   TEXT NOT NULL,
        reasons    TEXT NOT NULL,
        risk_score INTEGER NOT NULL,
        at         INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_blocked_agent ON blocked_executions(agent_id);
";

impl BlockedExecutionLog {
    /// Open (creating if needed) a log backed by a file.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(&format!("PRAGMA journal_mode=WAL;{SCHEMA}"))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open an ephemeral in-memory log (used by tests).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Record a denied decision. No-op (returns `None`) if the decision was
    /// actually allowed, so callers can pass every decision unconditionally.
    pub fn record(
        &self,
        agent_id: &str,
        decision: &SecurityDecision,
    ) -> Result<Option<BlockedExecution>> {
        if decision.allowed {
            return Ok(None);
        }
        let entry = BlockedExecution {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            reasons: decision.denials.join("; "),
            risk_score: decision.risk_score,
            at: decision.evaluated_at,
        };
        let conn = self.conn.lock().expect("blocked-log mutex poisoned");
        conn.execute(
            "INSERT INTO blocked_executions (id, agent_id, reasons, risk_score, at)
             VALUES (?1,?2,?3,?4,?5)",
            params![
                entry.id,
                entry.agent_id,
                entry.reasons,
                entry.risk_score,
                entry.at
            ],
        )?;
        cp_blocked!("gateway.blocked", agent_id = %entry.agent_id, reasons = %entry.reasons);
        Ok(Some(entry))
    }

    /// List blocked executions, newest first.
    pub fn list(&self) -> Result<Vec<BlockedExecution>> {
        let conn = self.conn.lock().expect("blocked-log mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, reasons, risk_score, at FROM blocked_executions ORDER BY at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BlockedExecution {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                reasons: row.get(2)?,
                risk_score: row.get(3)?,
                at: row.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_only_denied_decisions() {
        let log = BlockedExecutionLog::in_memory().unwrap();
        let allowed = SecurityDecision::new(vec![], 0, 1);
        assert!(log.record("a", &allowed).unwrap().is_none());

        let denied = SecurityDecision::new(vec!["tool not allowed".into()], 3, 2);
        assert!(log.record("a", &denied).unwrap().is_some());
        assert_eq!(log.list().unwrap().len(), 1);
    }
}
