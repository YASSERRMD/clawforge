//! SQLite-backed storage for the Agent Registry.
//!
//! Follows the workspace storage pattern: `open(path)` for persistence and
//! `in_memory()` for tests, with a `Mutex<Connection>` for interior mutability.
//! Enum and list fields are stored as JSON text so the schema stays stable as
//! the shared vocabularies evolve.

use std::sync::Mutex;

use rusqlite::{params, Connection};

use crate::error::Result;

use super::model::{AgentRecord, NewAgent};
use super::validation::validate_new_agent;

/// Persistent store for [`AgentRecord`](super::model::AgentRecord)s.
pub struct AgentRegistry {
    pub(crate) conn: Mutex<Connection>,
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS agents (
        id                  TEXT PRIMARY KEY,
        name                TEXT NOT NULL,
        description         TEXT NOT NULL,
        owner               TEXT NOT NULL,
        department          TEXT NOT NULL,
        framework           TEXT NOT NULL,
        model_provider      TEXT NOT NULL,
        model_name          TEXT NOT NULL,
        tools_allowed       TEXT NOT NULL,
        mcp_servers_allowed TEXT NOT NULL,
        data_access_level   TEXT NOT NULL,
        risk_level          TEXT NOT NULL,
        status              TEXT NOT NULL,
        version             INTEGER NOT NULL,
        created_at          INTEGER NOT NULL,
        updated_at          INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_agents_department ON agents(department);
    CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
";

impl AgentRegistry {
    /// Open (creating if needed) a registry backed by a file.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(&format!("PRAGMA journal_mode=WAL;{SCHEMA}"))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open an ephemeral in-memory registry (used by tests).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Validate and register a new agent, returning the materialised record.
    pub fn create(&self, input: NewAgent) -> Result<AgentRecord> {
        validate_new_agent(&input)?;
        let record = AgentRecord::from_new(input);
        let conn = self.conn.lock().expect("registry mutex poisoned");
        conn.execute(
            "INSERT INTO agents (
                id, name, description, owner, department, framework,
                model_provider, model_name, tools_allowed, mcp_servers_allowed,
                data_access_level, risk_level, status, version, created_at, updated_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                record.id,
                record.name,
                record.description,
                record.owner,
                record.department,
                record.framework,
                record.model_provider,
                record.model_name,
                serde_json::to_string(&record.tools_allowed)?,
                serde_json::to_string(&record.mcp_servers_allowed)?,
                serde_json::to_string(&record.data_access_level)?,
                serde_json::to_string(&record.risk_level)?,
                serde_json::to_string(&record.status)?,
                record.version,
                record.created_at,
                record.updated_at,
            ],
        )?;
        cp_info!("registry.agent.create", agent_id = %record.id, name = %record.name);
        Ok(record)
    }

    /// Total number of registered agents.
    pub fn count(&self) -> Result<u64> {
        let conn = self.conn.lock().expect("registry mutex poisoned");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM agents", [], |r| r.get(0))?;
        Ok(n as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_registry_is_empty() {
        let reg = AgentRegistry::in_memory().unwrap();
        assert_eq!(reg.count().unwrap(), 0);
    }
}
