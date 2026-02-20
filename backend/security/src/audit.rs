/// Audit log — records all agent actions and tool calls for compliance.
///
/// Per-channel audit trail: each event is stored in SQLite in the
/// `audit_events` table with channel, actor, action, and timestamp.
use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub channel: String,
    pub actor: String,
    pub action: String,
    pub tool: Option<String>,
    pub approved: Option<bool>,
    pub detail: serde_json::Value,
    pub timestamp: i64,
}

pub struct AuditLog {
    conn: Mutex<Connection>,
}

impl AuditLog {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS audit_events (
                 id        TEXT PRIMARY KEY,
                 channel   TEXT NOT NULL,
                 actor     TEXT NOT NULL,
                 action    TEXT NOT NULL,
                 tool      TEXT,
                 approved  INTEGER,
                 detail    TEXT NOT NULL,
                 timestamp INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_audit_channel ON audit_events(channel);
             CREATE INDEX IF NOT EXISTS idx_audit_ts ON audit_events(timestamp);",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_events (
                 id TEXT, channel TEXT, actor TEXT, action TEXT,
                 tool TEXT, approved INTEGER, detail TEXT, timestamp INTEGER
             );",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub async fn record(&self, event: AuditEvent) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO audit_events (id, channel, actor, action, tool, approved, detail, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.id.to_string(),
                event.channel,
                event.actor,
                event.action,
                event.tool,
                event.approved.map(|b| b as i32),
                serde_json::to_string(&event.detail)?,
                event.timestamp,
            ],
        )?;
        info!("[Audit] {} {} in {}", event.actor, event.action, event.channel);
        Ok(())
    }

    pub async fn recent(&self, channel: &str, limit: usize) -> Result<Vec<AuditEvent>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, channel, actor, action, tool, approved, detail, timestamp
             FROM audit_events WHERE channel = ?1
             ORDER BY timestamp DESC LIMIT ?2",
        )?;

        struct Row {
            id_str: String,
            channel: String,
            actor: String,
            action: String,
            tool: Option<String>,
            approved_i: Option<i32>,
            detail_str: String,
            timestamp: i64,
        }

        let events: Vec<AuditEvent> = stmt
            .query_map(params![channel, limit as i64], |row| {
                Ok(Row {
                    id_str: row.get(0)?,
                    channel: row.get(1)?,
                    actor: row.get(2)?,
                    action: row.get(3)?,
                    tool: row.get(4)?,
                    approved_i: row.get(5)?,
                    detail_str: row.get(6)?,
                    timestamp: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .filter_map(|r| {
                let id = Uuid::parse_str(&r.id_str).ok()?;
                let detail = serde_json::from_str(&r.detail_str).ok()?;
                Some(AuditEvent {
                    id,
                    channel: r.channel,
                    actor: r.actor,
                    action: r.action,
                    tool: r.tool,
                    approved: r.approved_i.map(|v| v != 0),
                    detail,
                    timestamp: r.timestamp,
                })
            })
            .collect();

        Ok(events)
    }
}

/// Helper to create a new audit event with the current timestamp.
pub fn new_event(channel: &str, actor: &str, action: &str) -> AuditEvent {
    AuditEvent {
        id: Uuid::new_v4(),
        channel: channel.to_string(),
        actor: actor.to_string(),
        action: action.to_string(),
        tool: None,
        approved: None,
        detail: serde_json::json!({}),
        timestamp: Utc::now().timestamp(),
    }
}
