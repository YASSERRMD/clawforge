//! SQLite-backed Governance Engine.
//!
//! Stores approval requests and an append-only change-history (audit) trail.
//! Follows the workspace storage pattern (`open`/`in_memory`).

use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::{ControlPlaneError, Result};

use super::model::{ApprovalRequest, NewApprovalRequest};

/// Approval workflow engine.
pub struct GovernanceEngine {
    pub(crate) conn: Mutex<Connection>,
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS approval_requests (
        id              TEXT PRIMARY KEY,
        kind            TEXT NOT NULL,
        subject_id      TEXT NOT NULL,
        subject_name    TEXT NOT NULL,
        requested_by    TEXT NOT NULL,
        department      TEXT NOT NULL,
        risk_level      TEXT NOT NULL,
        justification   TEXT NOT NULL,
        status          TEXT NOT NULL,
        decided_by      TEXT,
        decision_reason TEXT,
        created_at      INTEGER NOT NULL,
        decided_at      INTEGER
    );
    CREATE INDEX IF NOT EXISTS idx_appr_status ON approval_requests(status);
    CREATE INDEX IF NOT EXISTS idx_appr_owner ON approval_requests(requested_by);

    CREATE TABLE IF NOT EXISTS approval_events (
        id         TEXT PRIMARY KEY,
        request_id TEXT NOT NULL,
        action     TEXT NOT NULL,
        actor      TEXT NOT NULL,
        reason     TEXT,
        at         INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_appr_event_req ON approval_events(request_id);
";

const COLUMNS: &str = "id, kind, subject_id, subject_name, requested_by, department, \
    risk_level, justification, status, decided_by, decision_reason, created_at, decided_at";

fn row_to_request(row: &rusqlite::Row) -> rusqlite::Result<ApprovalRequest> {
    Ok(ApprovalRequest {
        id: row.get(0)?,
        kind: de(&row.get::<_, String>(1)?, 1)?,
        subject_id: row.get(2)?,
        subject_name: row.get(3)?,
        requested_by: row.get(4)?,
        department: row.get(5)?,
        risk_level: de(&row.get::<_, String>(6)?, 6)?,
        justification: row.get(7)?,
        status: de(&row.get::<_, String>(8)?, 8)?,
        decided_by: row.get(9)?,
        decision_reason: row.get(10)?,
        created_at: row.get(11)?,
        decided_at: row.get(12)?,
    })
}

fn de<T: serde::de::DeserializeOwned>(s: &str, col: usize) -> rusqlite::Result<T> {
    serde_json::from_str(s)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(col, rusqlite::types::Type::Text, Box::new(e)))
}

impl GovernanceEngine {
    /// Open (creating if needed) an engine backed by a file.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(&format!("PRAGMA journal_mode=WAL;{SCHEMA}"))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open an ephemeral in-memory engine (used by tests).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Submit a new approval request; it starts in `Pending`.
    pub fn submit(&self, input: NewApprovalRequest) -> Result<ApprovalRequest> {
        if input.justification.trim().is_empty() {
            return Err(ControlPlaneError::validation("justification must not be empty"));
        }
        let now = Utc::now().timestamp();
        let req = ApprovalRequest::from_new(input, Uuid::new_v4().to_string(), now);
        let conn = self.conn.lock().expect("governance mutex poisoned");
        conn.execute(
            "INSERT INTO approval_requests (
                id, kind, subject_id, subject_name, requested_by, department,
                risk_level, justification, status, decided_by, decision_reason,
                created_at, decided_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                req.id,
                serde_json::to_string(&req.kind)?,
                req.subject_id,
                req.subject_name,
                req.requested_by,
                req.department,
                serde_json::to_string(&req.risk_level)?,
                req.justification,
                serde_json::to_string(&req.status)?,
                req.decided_by,
                req.decision_reason,
                req.created_at,
                req.decided_at,
            ],
        )?;
        cp_info!("governance.submit", request_id = %req.id, kind = ?req.kind);
        Ok(req)
    }

    /// Fetch a single request by id.
    pub fn get(&self, id: &str) -> Result<ApprovalRequest> {
        let conn = self.conn.lock().expect("governance mutex poisoned");
        conn.query_row(
            &format!("SELECT {COLUMNS} FROM approval_requests WHERE id = ?1"),
            params![id],
            row_to_request,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => ControlPlaneError::not_found("approval_request", id),
            other => other.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::RiskLevel;
    use crate::governance::model::ApprovalKind;

    fn req() -> NewApprovalRequest {
        NewApprovalRequest {
            kind: ApprovalKind::Agent,
            subject_id: "agent-1".into(),
            subject_name: "Permit Bot".into(),
            requested_by: "platform-team".into(),
            department: "Licensing".into(),
            risk_level: RiskLevel::High,
            justification: "Needed for permit triage".into(),
        }
    }

    #[test]
    fn submit_creates_pending_request() {
        let eng = GovernanceEngine::in_memory().unwrap();
        let r = eng.submit(req()).unwrap();
        assert_eq!(r.status, super::super::model::ApprovalStatus::Pending);
        let fetched = eng.get(&r.id).unwrap();
        assert_eq!(fetched.subject_name, "Permit Bot");
    }

    #[test]
    fn submit_requires_justification() {
        let eng = GovernanceEngine::in_memory().unwrap();
        let mut bad = req();
        bad.justification = "  ".into();
        assert!(eng.submit(bad).is_err());
    }
}
