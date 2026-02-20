/// Durable run log for cron jobs.
///
/// Mirrors `src/cron/run-log.ts` from OpenClaw.
/// Every time a cron job fires, a row is written here with the result.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLogEntry {
    pub id: String,
    pub job_id: String,
    pub fired_at: i64,
    pub status: String, // "ok" | "error" | "skipped"
    pub output_summary: Option<String>,
    pub error: Option<String>,
}

pub struct RunLog {
    conn: rusqlite::Connection,
}

impl RunLog {
    pub fn open(db_path: &str) -> Result<Self> {
        let conn = rusqlite::Connection::open(db_path).context("open run log")?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS cron_run_log (
                id             TEXT PRIMARY KEY,
                job_id         TEXT NOT NULL,
                fired_at       INTEGER NOT NULL,
                status         TEXT NOT NULL,
                output_summary TEXT,
                error          TEXT
            );
            CREATE INDEX IF NOT EXISTS cron_run_log_job_id ON cron_run_log(job_id);
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn record(&self, entry: &RunLogEntry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO cron_run_log (id, job_id, fired_at, status, output_summary, error)
             VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![
                entry.id, entry.job_id, entry.fired_at,
                entry.status, entry.output_summary, entry.error,
            ],
        )?;
        Ok(())
    }

    pub fn recent(&self, job_id: &str, limit: usize) -> Result<Vec<RunLogEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, job_id, fired_at, status, output_summary, error
             FROM cron_run_log WHERE job_id = ?1
             ORDER BY fired_at DESC LIMIT ?2",
        )?;
        let entries = stmt.query_map(rusqlite::params![job_id, limit as i64], |row| {
            Ok(RunLogEntry {
                id: row.get(0)?,
                job_id: row.get(1)?,
                fired_at: row.get(2)?,
                status: row.get(3)?,
                output_summary: row.get(4)?,
                error: row.get(5)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(entries)
    }

    /// Prune entries older than `max_age_secs`.
    pub fn prune(&self, max_age_secs: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - max_age_secs;
        let n = self.conn.execute(
            "DELETE FROM cron_run_log WHERE fired_at < ?1",
            rusqlite::params![cutoff],
        )?;
        Ok(n)
    }
}
