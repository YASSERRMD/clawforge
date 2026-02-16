use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use tracing::info;

use clawforge_core::{Event, AgentSpec};

/// SQLite-backed event store for immutable event-sourcing.
pub struct EventStore {
    conn: Mutex<Connection>,
}

impl EventStore {
    /// Open or create the event store at the given path.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open SQLite database")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        info!(path = %path, "Event store opened");
        Ok(store)
    }

    /// Create an in-memory store (for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("Failed to open in-memory SQLite")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                kind TEXT NOT NULL,
                payload TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_events_run_id ON events(run_id);
            CREATE INDEX IF NOT EXISTS idx_events_agent_id ON events(agent_id);
            CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                spec TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    /// Insert an event into the store.
    pub fn insert(&self, event: &Event) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let payload = serde_json::to_string(&event.payload)?;
        conn.execute(
            "INSERT INTO events (id, run_id, agent_id, timestamp, kind, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id.to_string(),
                event.run_id.to_string(),
                event.agent_id.to_string(),
                event.timestamp.to_rfc3339(),
                event.kind.to_string(),
                payload,
            ],
        )?;
        Ok(())
    }

    /// Query events for a given run.
    pub fn get_run_events(&self, run_id: &uuid::Uuid) -> Result<Vec<Event>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, run_id, agent_id, timestamp, kind, payload
             FROM events WHERE run_id = ?1 ORDER BY timestamp ASC",
        )?;

        let events = stmt
            .query_map(params![run_id.to_string()], |row| {
                let id: String = row.get(0)?;
                let run_id: String = row.get(1)?;
                let agent_id: String = row.get(2)?;
                let timestamp: String = row.get(3)?;
                let kind: String = row.get(4)?;
                let payload: String = row.get(5)?;
                Ok((id, run_id, agent_id, timestamp, kind, payload))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(id, run_id, agent_id, timestamp, kind, payload)| {
                Some(Event {
                    id: uuid::Uuid::parse_str(&id).ok()?,
                    run_id: uuid::Uuid::parse_str(&run_id).ok()?,
                    agent_id: uuid::Uuid::parse_str(&agent_id).ok()?,
                    timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp)
                        .ok()?
                        .with_timezone(&chrono::Utc),
                    kind: serde_json::from_value(serde_json::Value::String(kind)).ok()?,
                    payload: serde_json::from_str(&payload).ok()?,
                })
            })
            .collect();

        Ok(events)
    }

    /// Count all events in the store.
    pub fn count(&self) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let count: usize = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Get recent events across all runs.
    pub fn get_recent(&self, limit: usize) -> Result<Vec<Event>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, run_id, agent_id, timestamp, kind, payload
             FROM events ORDER BY timestamp DESC LIMIT ?1",
        )?;

        let events = stmt
            .query_map(params![limit], |row| {
                let id: String = row.get(0)?;
                let run_id: String = row.get(1)?;
                let agent_id: String = row.get(2)?;
                let timestamp: String = row.get(3)?;
                let kind: String = row.get(4)?;
                let payload: String = row.get(5)?;
                Ok((id, run_id, agent_id, timestamp, kind, payload))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(id, run_id, agent_id, timestamp, kind, payload)| {
                Some(Event {
                    id: uuid::Uuid::parse_str(&id).ok()?,
                    run_id: uuid::Uuid::parse_str(&run_id).ok()?,
                    agent_id: uuid::Uuid::parse_str(&agent_id).ok()?,
                    timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp)
                        .ok()?
                        .with_timezone(&chrono::Utc),
                    kind: serde_json::from_value(serde_json::Value::String(kind)).ok()?,
                    payload: serde_json::from_str(&payload).ok()?,
                })
            })
            .collect();

        Ok(events)
    }

    /// Save an agent specification.
    pub fn save_agent(&self, agent: &AgentSpec) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let spec_json = serde_json::to_string(agent)?;
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO agents (id, name, spec, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                spec = excluded.spec,
                updated_at = excluded.updated_at",
            params![
                agent.id.to_string(),
                agent.name,
                spec_json,
                now,
            ],
        )?;
        Ok(())
    }

    /// Get an agent by ID.
    pub fn get_agent(&self, id: &uuid::Uuid) -> Result<Option<AgentSpec>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let mut stmt = conn.prepare("SELECT spec FROM agents WHERE id = ?1")?;
        
        let mut rows = stmt.query(params![id.to_string()])?;
        if let Some(row) = rows.next()? {
            let spec_json: String = row.get(0)?;
            let agent = serde_json::from_str(&spec_json)?;
            Ok(Some(agent))
        } else {
            Ok(None)
        }
    }

    /// List all agents.
    pub fn list_agents(&self) -> Result<Vec<AgentSpec>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let mut stmt = conn.prepare("SELECT spec FROM agents ORDER BY name ASC")?;
        
        let agents = stmt
            .query_map([], |row| {
                let spec_json: String = row.get(0)?;
                Ok(spec_json)
            })?
            .filter_map(|r| r.ok())
            .filter_map(|json| serde_json::from_str(&json).ok())
            .collect();
            
        Ok(agents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawforge_core::EventKind;
    use uuid::Uuid;

    #[test]
    fn test_insert_and_query() {
        let store = EventStore::in_memory().unwrap();
        let run_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();

        let event = Event::new(
            run_id,
            agent_id,
            EventKind::RunStarted,
            serde_json::json!({"test": true}),
        );
        store.insert(&event).unwrap();

        let events = store.get_run_events(&run_id).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::RunStarted);
    }

    #[test]
    fn test_count() {
        let store = EventStore::in_memory().unwrap();
        assert_eq!(store.count().unwrap(), 0);

        let run_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        for kind in [EventKind::RunStarted, EventKind::ActionExecuted, EventKind::RunCompleted] {
            store
                .insert(&Event::new(run_id, agent_id, kind, serde_json::json!({})))
                .unwrap();
        }
        assert_eq!(store.count().unwrap(), 3);
    }

    #[test]
    fn test_get_recent() {
        let store = EventStore::in_memory().unwrap();
        let run_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();

        for _ in 0..5 {
            store
                .insert(&Event::new(
                    run_id,
                    agent_id,
                    EventKind::ActionExecuted,
                    serde_json::json!({}),
                ))
                .unwrap();
        }

        let recent = store.get_recent(3).unwrap();
        assert_eq!(recent.len(), 3);
    }
}
