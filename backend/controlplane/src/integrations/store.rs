//! SQLite-backed integration registry.
//!
//! Tracks registered enterprise/government integrations, their governance
//! status, and the *reference* to their credentials (never the secret itself).

use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::constants::LifecycleStatus;
use crate::error::{ControlPlaneError, Result};

use super::model::{IntegrationProvider, NewIntegration};

/// Persistent registry of enterprise integrations.
pub struct IntegrationRegistry {
    pub(crate) conn: Mutex<Connection>,
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS integrations (
        id          TEXT PRIMARY KEY,
        name        TEXT NOT NULL,
        kind        TEXT NOT NULL,
        description TEXT NOT NULL,
        owner       TEXT NOT NULL,
        department  TEXT NOT NULL,
        endpoint    TEXT NOT NULL,
        credential  TEXT NOT NULL,
        permissions TEXT NOT NULL,
        risk_level  TEXT NOT NULL,
        status      TEXT NOT NULL,
        created_at  INTEGER NOT NULL,
        updated_at  INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_integ_kind ON integrations(kind);
    CREATE INDEX IF NOT EXISTS idx_integ_status ON integrations(status);
";

const COLUMNS: &str = "id, name, kind, description, owner, department, endpoint, credential, \
    permissions, risk_level, status, created_at, updated_at";

fn row_to_integration(row: &rusqlite::Row) -> rusqlite::Result<IntegrationProvider> {
    Ok(IntegrationProvider {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: de(&row.get::<_, String>(2)?, 2)?,
        description: row.get(3)?,
        owner: row.get(4)?,
        department: row.get(5)?,
        endpoint: row.get(6)?,
        credential: de(&row.get::<_, String>(7)?, 7)?,
        permissions: de(&row.get::<_, String>(8)?, 8)?,
        risk_level: de(&row.get::<_, String>(9)?, 9)?,
        status: de(&row.get::<_, String>(10)?, 10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn de<T: serde::de::DeserializeOwned>(s: &str, col: usize) -> rusqlite::Result<T> {
    serde_json::from_str(s)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(col, rusqlite::types::Type::Text, Box::new(e)))
}

impl IntegrationRegistry {
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

    /// Register a new integration; it starts in `PendingApproval`.
    pub fn register(&self, input: NewIntegration) -> Result<IntegrationProvider> {
        if input.name.trim().is_empty() {
            return Err(ControlPlaneError::validation("integration name must not be empty"));
        }
        let integration = IntegrationProvider::from_new(input);
        self.upsert(&integration)?;
        cp_info!("integration.register", id = %integration.id, name = %integration.name);
        Ok(integration)
    }

    /// List all integrations, newest first.
    pub fn list(&self) -> Result<Vec<IntegrationProvider>> {
        let conn = self.conn.lock().expect("integration mutex poisoned");
        let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM integrations ORDER BY created_at DESC"))?;
        let rows = stmt.query_map([], row_to_integration)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Fetch an integration by id.
    pub fn get(&self, id: &str) -> Result<IntegrationProvider> {
        let conn = self.conn.lock().expect("integration mutex poisoned");
        conn.query_row(
            &format!("SELECT {COLUMNS} FROM integrations WHERE id = ?1"),
            params![id],
            row_to_integration,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => ControlPlaneError::not_found("integration", id),
            other => other.into(),
        })
    }

    /// Approve an integration, making it usable (sets status `Active`).
    pub fn approve(&self, id: &str) -> Result<IntegrationProvider> {
        self.set_status(id, LifecycleStatus::Active)
    }

    /// Block an integration, taking it out of service (sets status `Blocked`).
    pub fn block(&self, id: &str) -> Result<IntegrationProvider> {
        self.set_status(id, LifecycleStatus::Blocked)
    }

    fn set_status(&self, id: &str, status: LifecycleStatus) -> Result<IntegrationProvider> {
        let mut integration = self.get(id)?;
        integration.status = status;
        integration.updated_at = Utc::now().timestamp();
        self.upsert(&integration)?;
        cp_info!("integration.status", id = %id, status = ?status);
        Ok(integration)
    }

    /// Total number of registered integrations.
    pub fn count(&self) -> Result<u64> {
        let conn = self.conn.lock().expect("integration mutex poisoned");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM integrations", [], |r| r.get(0))?;
        Ok(n as u64)
    }

    pub(crate) fn upsert(&self, i: &IntegrationProvider) -> Result<()> {
        let conn = self.conn.lock().expect("integration mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO integrations (
                id, name, kind, description, owner, department, endpoint, credential,
                permissions, risk_level, status, created_at, updated_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                i.id,
                i.name,
                serde_json::to_string(&i.kind)?,
                i.description,
                i.owner,
                i.department,
                i.endpoint,
                serde_json::to_string(&i.credential)?,
                serde_json::to_string(&i.permissions)?,
                serde_json::to_string(&i.risk_level)?,
                serde_json::to_string(&i.status)?,
                i.created_at,
                i.updated_at,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::RiskLevel;
    use crate::integrations::model::{CredentialRef, IntegrationKind, IntegrationPermission};

    pub(super) fn input() -> NewIntegration {
        NewIntegration {
            name: "Resident DB".into(),
            kind: IntegrationKind::Postgres,
            description: "Resident records database".into(),
            owner: "data-platform".into(),
            department: "IT".into(),
            endpoint: "postgres://db.internal:5432/residents".into(),
            credential: CredentialRef::vault("kv/integrations/resident-db"),
            permissions: vec![IntegrationPermission::Connect, IntegrationPermission::Read],
            risk_level: RiskLevel::High,
        }
    }

    #[test]
    fn register_list_get_lifecycle() {
        let reg = IntegrationRegistry::in_memory().unwrap();
        let i = reg.register(input()).unwrap();
        assert_eq!(i.status, LifecycleStatus::PendingApproval);
        assert!(!i.is_usable());
        assert_eq!(reg.list().unwrap().len(), 1);
        assert_eq!(reg.get(&i.id).unwrap().name, "Resident DB");
        assert!(reg.approve(&i.id).unwrap().is_usable());
        assert!(!reg.block(&i.id).unwrap().is_usable());
    }
}
