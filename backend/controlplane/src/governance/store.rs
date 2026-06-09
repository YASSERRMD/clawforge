//! SQLite-backed Governance Engine.
//!
//! Stores approval requests and an append-only change-history (audit) trail.
//! Follows the workspace storage pattern (`open`/`in_memory`).

use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::{ControlPlaneError, Result};

use super::model::{ApprovalEvent, ApprovalRequest, ApprovalStatus, NewApprovalRequest};

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
        drop(conn);
        self.record_event(&req.id, "submitted", &req.requested_by, Some(&req.justification))?;
        Ok(req)
    }

    /// Append a change-history (audit) event for a request.
    fn record_event(&self, request_id: &str, action: &str, actor: &str, reason: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().expect("governance mutex poisoned");
        conn.execute(
            "INSERT INTO approval_events (id, request_id, action, actor, reason, at)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                Uuid::new_v4().to_string(),
                request_id,
                action,
                actor,
                reason,
                Utc::now().timestamp(),
            ],
        )?;
        Ok(())
    }

    /// Full change history for a request, oldest first.
    pub fn history(&self, request_id: &str) -> Result<Vec<ApprovalEvent>> {
        let conn = self.conn.lock().expect("governance mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, request_id, action, actor, reason, at
             FROM approval_events WHERE request_id = ?1 ORDER BY at ASC",
        )?;
        let rows = stmt.query_map(params![request_id], |row| {
            Ok(ApprovalEvent {
                id: row.get(0)?,
                request_id: row.get(1)?,
                action: row.get(2)?,
                actor: row.get(3)?,
                reason: row.get(4)?,
                at: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Approve a pending request, recording who decided and why.
    pub fn approve(&self, id: &str, decided_by: &str, reason: &str) -> Result<ApprovalRequest> {
        self.decide(id, ApprovalStatus::Approved, decided_by, reason)
    }

    /// Reject a pending request, recording who decided and why.
    pub fn reject(&self, id: &str, decided_by: &str, reason: &str) -> Result<ApprovalRequest> {
        self.decide(id, ApprovalStatus::Rejected, decided_by, reason)
    }

    /// Apply a terminal decision to a pending request (internal helper).
    fn decide(&self, id: &str, status: ApprovalStatus, decided_by: &str, reason: &str) -> Result<ApprovalRequest> {
        // Every decision must carry a policy reason for the audit trail.
        if reason.trim().is_empty() {
            return Err(ControlPlaneError::validation("a decision reason is required"));
        }
        let current = self.get(id)?;
        if current.status.is_decided() {
            return Err(ControlPlaneError::Conflict(format!(
                "request {id} already {:?}",
                current.status
            )));
        }
        let now = Utc::now().timestamp();
        {
            let conn = self.conn.lock().expect("governance mutex poisoned");
            conn.execute(
                "UPDATE approval_requests
                 SET status = ?2, decided_by = ?3, decision_reason = ?4, decided_at = ?5
                 WHERE id = ?1",
                params![id, serde_json::to_string(&status)?, decided_by, reason, now],
            )?;
        }
        let action = match status {
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Rejected => "rejected",
            ApprovalStatus::Pending => "pending",
        };
        self.record_event(id, action, decided_by, Some(reason))?;
        cp_info!("governance.decide", request_id = %id, status = ?status, actor = %decided_by);
        self.get(id)
    }

    /// List all approval requests, newest first.
    pub fn list(&self) -> Result<Vec<ApprovalRequest>> {
        self.query_requests(&format!("SELECT {COLUMNS} FROM approval_requests ORDER BY created_at DESC"), [])
    }

    /// List approval requests with a given status (e.g. all `Pending`).
    pub fn list_by_status(&self, status: ApprovalStatus) -> Result<Vec<ApprovalRequest>> {
        self.query_requests(
            &format!("SELECT {COLUMNS} FROM approval_requests WHERE status = ?1 ORDER BY created_at DESC"),
            params![serde_json::to_string(&status)?],
        )
    }

    /// List approval requests submitted by a given owner/requester.
    pub fn list_by_owner(&self, requested_by: &str) -> Result<Vec<ApprovalRequest>> {
        self.query_requests(
            &format!("SELECT {COLUMNS} FROM approval_requests WHERE requested_by = ?1 ORDER BY created_at DESC"),
            params![requested_by],
        )
    }

    /// Run a SELECT returning approval requests (internal helper).
    fn query_requests<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<Vec<ApprovalRequest>> {
        let conn = self.conn.lock().expect("governance mutex poisoned");
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params, row_to_request)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
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

    #[test]
    fn approve_sets_decision_fields() {
        let eng = GovernanceEngine::in_memory().unwrap();
        let r = eng.submit(req()).unwrap();
        let approved = eng.approve(&r.id, "ciso", "meets policy").unwrap();
        assert_eq!(approved.status, ApprovalStatus::Approved);
        assert_eq!(approved.decided_by.as_deref(), Some("ciso"));
        assert_eq!(approved.decision_reason.as_deref(), Some("meets policy"));
        assert!(approved.decided_at.is_some());
    }

    #[test]
    fn reject_sets_rejected_status() {
        let eng = GovernanceEngine::in_memory().unwrap();
        let r = eng.submit(req()).unwrap();
        let rejected = eng.reject(&r.id, "ciso", "insufficient justification").unwrap();
        assert_eq!(rejected.status, ApprovalStatus::Rejected);
    }

    #[test]
    fn decision_requires_reason() {
        let eng = GovernanceEngine::in_memory().unwrap();
        let r = eng.submit(req()).unwrap();
        assert!(eng.approve(&r.id, "ciso", "  ").is_err());
    }

    #[test]
    fn cannot_decide_twice() {
        let eng = GovernanceEngine::in_memory().unwrap();
        let r = eng.submit(req()).unwrap();
        eng.approve(&r.id, "ciso", "ok").unwrap();
        let err = eng.reject(&r.id, "ciso", "changed mind").unwrap_err();
        assert!(matches!(err, ControlPlaneError::Conflict(_)));
    }

    #[test]
    fn history_tracks_submit_and_decision() {
        let eng = GovernanceEngine::in_memory().unwrap();
        let r = eng.submit(req()).unwrap();
        eng.approve(&r.id, "ciso", "ok").unwrap();
        let hist = eng.history(&r.id).unwrap();
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].action, "submitted");
        assert_eq!(hist[1].action, "approved");
    }

    #[test]
    fn filters_by_status_and_owner() {
        let eng = GovernanceEngine::in_memory().unwrap();
        let a = eng.submit(req()).unwrap();
        let mut other = req();
        other.requested_by = "other-team".into();
        eng.submit(other).unwrap();
        eng.approve(&a.id, "ciso", "ok").unwrap();

        assert_eq!(eng.list().unwrap().len(), 2);
        assert_eq!(eng.list_by_status(ApprovalStatus::Pending).unwrap().len(), 1);
        assert_eq!(eng.list_by_status(ApprovalStatus::Approved).unwrap().len(), 1);
        assert_eq!(eng.list_by_owner("other-team").unwrap().len(), 1);
    }
}
