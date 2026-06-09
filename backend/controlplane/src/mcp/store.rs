//! SQLite-backed MCP registry storage.
//!
//! Follows the workspace storage pattern (`open`/`in_memory`); list/enum fields
//! are stored as JSON text for schema stability.

use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::constants::LifecycleStatus;
use crate::error::{ControlPlaneError, Result};

use super::model::{HealthStatus, McpServer, NewMcpServer};

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
    serde_json::from_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(col, rusqlite::types::Type::Text, Box::new(e))
    })
}

impl McpRegistry {
    /// Open (creating if needed) a registry backed by a file.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(&format!("PRAGMA journal_mode=WAL;{SCHEMA}"))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open an ephemeral in-memory registry (used by tests).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Register a new MCP server; it starts in `PendingApproval`.
    pub fn register(&self, input: NewMcpServer) -> Result<McpServer> {
        if input.name.trim().is_empty() {
            return Err(ControlPlaneError::validation(
                "MCP server name must not be empty",
            ));
        }
        if input.endpoint.trim().is_empty() {
            return Err(ControlPlaneError::validation(
                "MCP server endpoint must not be empty",
            ));
        }
        let server = McpServer::from_new(input);
        self.upsert(&server)?;
        cp_info!("mcp.register", server_id = %server.id, name = %server.name);
        Ok(server)
    }

    /// List all registered MCP servers, newest first.
    pub fn list(&self) -> Result<Vec<McpServer>> {
        let conn = self.conn.lock().expect("mcp mutex poisoned");
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM mcp_servers ORDER BY created_at DESC"
        ))?;
        let rows = stmt.query_map([], row_to_server)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// List servers with a given lifecycle status (e.g. all `PendingApproval`).
    pub fn list_by_status(&self, status: LifecycleStatus) -> Result<Vec<McpServer>> {
        let conn = self.conn.lock().expect("mcp mutex poisoned");
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM mcp_servers WHERE status = ?1 ORDER BY created_at DESC"
        ))?;
        let rows = stmt.query_map(params![serde_json::to_string(&status)?], row_to_server)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Approve a server, making it usable by agents (sets status `Active`).
    pub fn approve(&self, id: &str) -> Result<McpServer> {
        self.set_status(id, LifecycleStatus::Active)
    }

    /// Block a server, preventing agents from using it (sets status `Blocked`).
    pub fn block(&self, id: &str) -> Result<McpServer> {
        self.set_status(id, LifecycleStatus::Blocked)
    }

    /// Record a single usage of a server, incrementing its call count and
    /// accumulating the estimated cost. Returns the updated record.
    pub fn record_usage(&self, id: &str, cost: f64) -> Result<McpServer> {
        let mut server = self.get(id)?;
        server.usage_count += 1;
        server.cost_estimate += cost;
        server.updated_at = Utc::now().timestamp();
        self.upsert(&server)?;
        Ok(server)
    }

    /// Record a health-check result, updating the server's health and the
    /// `last_health_check` timestamp. Returns the updated record.
    pub fn record_health(&self, id: &str, health: HealthStatus) -> Result<McpServer> {
        let mut server = self.get(id)?;
        let now = Utc::now().timestamp();
        server.health = health;
        server.last_health_check = Some(now);
        server.updated_at = now;
        self.upsert(&server)?;
        cp_info!("mcp.health", server_id = %id, health = ?health);
        Ok(server)
    }

    /// Update a server's lifecycle status (internal helper).
    fn set_status(&self, id: &str, status: LifecycleStatus) -> Result<McpServer> {
        let mut server = self.get(id)?;
        server.status = status;
        server.updated_at = Utc::now().timestamp();
        self.upsert(&server)?;
        cp_info!("mcp.status", server_id = %id, status = ?status);
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
        assert!(matches!(
            reg.get("nope"),
            Err(ControlPlaneError::NotFound { .. })
        ));
    }

    use crate::constants::RiskLevel;
    use crate::mcp::model::{HealthStatus, McpTool, NewMcpServer, TransportType};

    fn input() -> NewMcpServer {
        NewMcpServer {
            name: "records-mcp".into(),
            description: "Resident records access".into(),
            owner: "data-platform".into(),
            endpoint: "https://mcp.internal/records".into(),
            transport: TransportType::Http,
            tools_exposed: vec![
                McpTool {
                    name: "lookup".into(),
                    description: "read records".into(),
                    permissions: vec!["read".into()],
                },
                McpTool {
                    name: "write".into(),
                    description: "update records".into(),
                    permissions: vec!["write".into(), "pii".into()],
                },
            ],
            permissions_required: vec!["read".into(), "write".into()],
            risk_level: RiskLevel::High,
        }
    }

    #[test]
    fn register_starts_pending_and_unusable() {
        let reg = McpRegistry::in_memory().unwrap();
        let s = reg.register(input()).unwrap();
        assert_eq!(s.status, LifecycleStatus::PendingApproval);
        assert!(!s.is_usable());
        assert_eq!(reg.get(&s.id).unwrap().name, "records-mcp");
        assert_eq!(s.sensitive_tool_count(), 1);
        assert!(s.requires_governance_review());
    }

    #[test]
    fn register_rejects_empty_endpoint() {
        let reg = McpRegistry::in_memory().unwrap();
        let mut bad = input();
        bad.endpoint = "  ".into();
        assert!(reg.register(bad).is_err());
    }

    #[test]
    fn approve_then_block_changes_usability() {
        let reg = McpRegistry::in_memory().unwrap();
        let s = reg.register(input()).unwrap();
        assert!(reg.approve(&s.id).unwrap().is_usable());
        assert!(!reg.block(&s.id).unwrap().is_usable());
    }

    #[test]
    fn list_and_filter_by_status() {
        let reg = McpRegistry::in_memory().unwrap();
        let a = reg.register(input()).unwrap();
        reg.register(input()).unwrap();
        reg.approve(&a.id).unwrap();
        assert_eq!(reg.list().unwrap().len(), 2);
        assert_eq!(
            reg.list_by_status(LifecycleStatus::Active).unwrap().len(),
            1
        );
        assert_eq!(
            reg.list_by_status(LifecycleStatus::PendingApproval)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn usage_and_health_accumulate() {
        let reg = McpRegistry::in_memory().unwrap();
        let s = reg.register(input()).unwrap();
        reg.record_usage(&s.id, 0.05).unwrap();
        let s = reg.record_usage(&s.id, 0.05).unwrap();
        assert_eq!(s.usage_count, 2);
        assert!((s.cost_estimate - 0.10).abs() < 1e-9);

        let s = reg.record_health(&s.id, HealthStatus::Healthy).unwrap();
        assert_eq!(s.health, HealthStatus::Healthy);
        assert!(s.last_health_check.is_some());
    }
}
