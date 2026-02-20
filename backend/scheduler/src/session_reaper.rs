/// Session reaper — garbage-collects expired cron sessions.
///
/// Mirrors `src/cron/session-reaper.ts` from OpenClaw.
/// Cron jobs create ephemeral sessions; this reaper cleans them up after
/// `max_age_secs` seconds to prevent unbounded session accumulation.
use anyhow::Result;
use tracing::{info, warn};

pub struct SessionReaper {
    /// Maximum age in seconds before a cron session is reaped.
    pub max_age_secs: i64,
    /// SQLite path (shares the cron store DB).
    db_path: String,
}

impl SessionReaper {
    pub fn new(db_path: impl Into<String>, max_age_secs: i64) -> Self {
        Self { db_path: db_path.into(), max_age_secs }
    }

    /// Reap all cron sessions older than `max_age_secs`.
    /// Returns the number of sessions reaped.
    pub fn reap(&self) -> Result<usize> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        // Ensure the table exists (created by CronStore)
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS cron_sessions (
                session_id  TEXT PRIMARY KEY,
                job_id      TEXT NOT NULL,
                started_at  INTEGER NOT NULL,
                status      TEXT NOT NULL DEFAULT 'running'
            );
            "#,
        )?;

        let cutoff = chrono::Utc::now().timestamp() - self.max_age_secs;
        let n = conn.execute(
            "DELETE FROM cron_sessions WHERE started_at < ?1 AND status != 'running'",
            rusqlite::params![cutoff],
        )?;
        if n > 0 {
            info!("[SessionReaper] Reaped {} expired cron sessions", n);
        }
        Ok(n)
    }

    /// Register a new cron session.
    pub fn register_session(&self, session_id: &str, job_id: &str) -> Result<()> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO cron_sessions (session_id, job_id, started_at, status)
             VALUES (?1, ?2, ?3, 'running')",
            rusqlite::params![session_id, job_id, chrono::Utc::now().timestamp()],
        )?;
        Ok(())
    }

    /// Mark a cron session as completed or errored.
    pub fn complete_session(&self, session_id: &str, status: &str) -> Result<()> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        let n = conn.execute(
            "UPDATE cron_sessions SET status = ?1 WHERE session_id = ?2",
            rusqlite::params![status, session_id],
        )?;
        if n == 0 {
            warn!("[SessionReaper] complete_session: session {} not found", session_id);
        }
        Ok(())
    }
}
