//! SQLite-backed MCP registry storage.
//!
//! Follows the workspace storage pattern (`open`/`in_memory`); list/enum fields
//! are stored as JSON text for schema stability.

use std::sync::Mutex;

use rusqlite::{params, Connection};

use crate::error::{ControlPlaneError, Result};

use super::model::{McpServer, NewMcpServer};

/// Persistent registry of MCP servers.
pub struct McpRegistry {
    pub(crate) conn: Mutex<Connection>,
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS mcp_servers (
        id                  TEXT PRIMARY KEY,
        name                TEXT NOT NULL,
        description         TEXT NOT NULL,
        owner               TEXT NOT NULL,
        endpoint            TEXT NOT NULL,
        transport           TEXT NOT NULL,
        tools_exposed       TEXT NOT NULL,
        permissions_required TEXT NOT NULL,
        risk_level          TEXT NOT NULL,
        status              TEXT NOT NULL,
        health              TEXT NOT NULL,
        last_health_check   INTEGER,
        usage_count         INTEGER NOT NULL,
        cost_estimate       REAL NOT NULL,
        created_at          INTEGER NOT NULL,
        updated_at          INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_mcp_status ON mcp_servers(status);
    CREATE INDEX IF NOT EXISTS idx_mcp_owner ON mcp_servers(owner);
";

const COLUMNS: &str = "id, name, description, owner, endpoint, transport, tools_exposed, \
    permissions_required, risk_level, status, health, last_health_check, usage_count, \
    cost_estimate, created_at, updated_at";

fn row_to_server(row: &rusqlite::Row) -> rusqlite::Result<McpServer> {
    Ok(McpServer {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        owner: row.get(3)?,
        endpoint: row.get(4)?,
        transport: de(&row.get::<_, String>(5)?, 5)?,
        tools_exposed: de(&row.get::<_, String>(6)?, 6)?,
        permissions_required: de(&row.get::<_, String>(7)?, 7)?,
        risk_level: de(&row.get::<_, String>(8)?, 8)?,
        status: de(&row.get::<_, String>(9)?, 9)?,
        health: de(&row.get::<_, String>(10)?, 10)?,
        last_health_check: row.get(11)?,
        usage_count: row.get::<_, i64>(12)? as u64,
        cost_estimate: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn de<T: serde::de::DeserializeOwned>(s: &str, col: usize) -> rusqlite::Result<T> {
    serde_json::from_str(s)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(col, rusqlite::types::Type::Text, Box::new(e)))
}

impl McpRegistry {
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

    /// Register a new MCP server; it starts in `PendingApproval`.
    pub fn register(&self, input: NewMcpServer) -> Result<McpServer> {
        if input.name.trim().is_empty() {
            return Err(ControlPlaneError::validation("MCP server name must not be empty"));
        }
        if input.endpoint.trim().is_empty() {
            return Err(ControlPlaneError::validation("MCP server endpoint must not be empty"));
        }
        let server = McpServer::from_new(input);
        self.upsert(&server)?;
        cp_info!("mcp.register", server_id = %server.id, name = %server.name);
        Ok(server)
    }

    /// Fetch a server by id, or [`ControlPlaneError::NotFound`].
    pub fn get(&self, id: &str) -> Result<McpServer> {
        let conn = self.conn.lock().expect("mcp mutex poisoned");
        conn.query_row(
            &format!("SELECT {COLUMNS} FROM mcp_servers WHERE id = ?1"),
            params![id],
            row_to_server,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => ControlPlaneError::not_found("mcp_server", id),
            other => other.into(),
        })
    }

    /// Total number of registered servers.
    pub fn count(&self) -> Result<u64> {
        let conn = self.conn.lock().expect("mcp mutex poisoned");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM mcp_servers", [], |r| r.get(0))?;
        Ok(n as u64)
    }

    /// Persist a server record (insert or replace). Internal helper.
    pub(crate) fn upsert(&self, s: &McpServer) -> Result<()> {
        let conn = self.conn.lock().expect("mcp mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO mcp_servers (
                id, name, description, owner, endpoint, transport, tools_exposed,
                permissions_required, risk_level, status, health, last_health_check,
                usage_count, cost_estimate, created_at, updated_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                s.id,
                s.name,
                s.description,
                s.owner,
                s.endpoint,
                serde_json::to_string(&s.transport)?,
                serde_json::to_string(&s.tools_exposed)?,
                serde_json::to_string(&s.permissions_required)?,
                serde_json::to_string(&s.risk_level)?,
                serde_json::to_string(&s.status)?,
                serde_json::to_string(&s.health)?,
                s.last_health_check,
                s.usage_count as i64,
                s.cost_estimate,
                s.created_at,
                s.updated_at,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_registry_is_empty() {
        let reg = McpRegistry::in_memory().unwrap();
        assert_eq!(reg.count().unwrap(), 0);
    }

    #[test]
    fn get_missing_is_not_found() {
        let reg = McpRegistry::in_memory().unwrap();
        assert!(matches!(reg.get("nope"), Err(ControlPlaneError::NotFound { .. })));
    }
}
