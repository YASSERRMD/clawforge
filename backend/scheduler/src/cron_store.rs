/// Durable SQLite-backed storage for cron job configurations.
///
/// Mirrors `src/cron/store.ts` from OpenClaw.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub agent_id: String,
    pub channel: String,
    /// 5-field cron expression
    pub schedule: String,
    /// Optional delivery target (session_id or channel string)
    pub delivery_target: Option<String>,
    /// Prompt text to send when the job fires
    pub prompt: String,
    /// Whether the job is currently active
    pub enabled: bool,
    /// Stagger window in seconds (0 = no stagger)
    pub stagger_secs: u64,
    /// Maximum number of runs (None = unlimited)
    pub max_runs: Option<u64>,
    /// Count of completed runs
    pub run_count: u64,
    pub created_at: i64,
}

pub struct CronStore {
    conn: rusqlite::Connection,
}

impl CronStore {
    pub fn open(db_path: &str) -> Result<Self> {
        let conn = rusqlite::Connection::open(db_path)
            .context("open cron store")?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS cron_jobs (
                id              TEXT PRIMARY KEY,
                agent_id        TEXT NOT NULL,
                channel         TEXT NOT NULL,
                schedule        TEXT NOT NULL,
                delivery_target TEXT,
                prompt          TEXT NOT NULL,
                enabled         INTEGER NOT NULL DEFAULT 1,
                stagger_secs    INTEGER NOT NULL DEFAULT 0,
                max_runs        INTEGER,
                run_count       INTEGER NOT NULL DEFAULT 0,
                created_at      INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn upsert(&self, job: &CronJob) -> Result<()> {
        self.conn.execute(
            r#"INSERT INTO cron_jobs
               (id, agent_id, channel, schedule, delivery_target, prompt,
                enabled, stagger_secs, max_runs, run_count, created_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
               ON CONFLICT(id) DO UPDATE SET
                 schedule=excluded.schedule,
                 delivery_target=excluded.delivery_target,
                 prompt=excluded.prompt,
                 enabled=excluded.enabled,
                 stagger_secs=excluded.stagger_secs,
                 max_runs=excluded.max_runs"#,
            rusqlite::params![
                job.id, job.agent_id, job.channel, job.schedule,
                job.delivery_target, job.prompt,
                job.enabled as i32, job.stagger_secs as i64,
                job.max_runs.map(|v| v as i64),
                job.run_count as i64, job.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_enabled(&self) -> Result<Vec<CronJob>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent_id, channel, schedule, delivery_target, prompt,
                    enabled, stagger_secs, max_runs, run_count, created_at
             FROM cron_jobs WHERE enabled = 1"
        )?;
        let jobs = stmt.query_map([], |row| {
            Ok(CronJob {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                channel: row.get(2)?,
                schedule: row.get(3)?,
                delivery_target: row.get(4)?,
                prompt: row.get(5)?,
                enabled: row.get::<_, i32>(6)? != 0,
                stagger_secs: row.get::<_, i64>(7)? as u64,
                max_runs: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                run_count: row.get::<_, i64>(9)? as u64,
                created_at: row.get(10)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(jobs)
    }

    pub fn increment_run_count(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE cron_jobs SET run_count = run_count + 1 WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    pub fn disable(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE cron_jobs SET enabled = 0 WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM cron_jobs WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }
}
