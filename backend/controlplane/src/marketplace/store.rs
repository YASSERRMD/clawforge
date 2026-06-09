//! SQLite-backed marketplace store.
//!
//! Holds published listings. Installing a listing is delegated to the caller's
//! [`AgentRegistry`](crate::registry::AgentRegistry) so the marketplace stays
//! decoupled from agent persistence.

use std::sync::Mutex;

use rusqlite::{params, Connection};

use crate::constants::RiskLevel;
use crate::error::{ControlPlaneError, Result};
use crate::registry::{AgentRecord, AgentRegistry};

use super::model::{MarketplaceAgent, NewListing};

/// Store of published marketplace listings.
pub struct Marketplace {
    pub(crate) conn: Mutex<Connection>,
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS marketplace_listings (
        id            TEXT PRIMARY KEY,
        name          TEXT NOT NULL,
        description   TEXT NOT NULL,
        category      TEXT NOT NULL,
        department    TEXT NOT NULL,
        rating        REAL NOT NULL,
        install_count INTEGER NOT NULL,
        risk_level    TEXT NOT NULL,
        verification  TEXT NOT NULL,
        compliance    TEXT NOT NULL,
        template      TEXT NOT NULL,
        published_at  INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_mkt_category ON marketplace_listings(category);
";

const COLUMNS: &str = "id, name, description, category, department, rating, install_count, \
    risk_level, verification, compliance, template, published_at";

fn row_to_listing(row: &rusqlite::Row) -> rusqlite::Result<MarketplaceAgent> {
    Ok(MarketplaceAgent {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        category: row.get(3)?,
        department: row.get(4)?,
        rating: row.get(5)?,
        install_count: row.get::<_, i64>(6)? as u64,
        risk_level: de(&row.get::<_, String>(7)?, 7)?,
        verification: de(&row.get::<_, String>(8)?, 8)?,
        compliance: de(&row.get::<_, String>(9)?, 9)?,
        template: de(&row.get::<_, String>(10)?, 10)?,
        published_at: row.get(11)?,
    })
}

fn de<T: serde::de::DeserializeOwned>(s: &str, col: usize) -> rusqlite::Result<T> {
    serde_json::from_str(s)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(col, rusqlite::types::Type::Text, Box::new(e)))
}

impl Marketplace {
    /// Open (creating if needed) a marketplace backed by a file.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(&format!("PRAGMA journal_mode=WAL;{SCHEMA}"))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open an ephemeral in-memory marketplace (used by tests).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Publish a new listing to the marketplace.
    pub fn publish(&self, input: NewListing) -> Result<MarketplaceAgent> {
        if input.name.trim().is_empty() {
            return Err(ControlPlaneError::validation("listing name must not be empty"));
        }
        let listing = MarketplaceAgent::from_new(input);
        self.upsert(&listing)?;
        cp_info!("marketplace.publish", listing_id = %listing.id, name = %listing.name);
        Ok(listing)
    }

    /// List all listings, most-installed first.
    pub fn list(&self) -> Result<Vec<MarketplaceAgent>> {
        self.query(&format!("SELECT {COLUMNS} FROM marketplace_listings ORDER BY install_count DESC"), [])
    }

    /// List listings in a given category.
    pub fn list_by_category(&self, category: &str) -> Result<Vec<MarketplaceAgent>> {
        self.query(
            &format!("SELECT {COLUMNS} FROM marketplace_listings WHERE category = ?1 ORDER BY install_count DESC"),
            params![category],
        )
    }

    /// List listings at a given risk level.
    pub fn list_by_risk(&self, risk: RiskLevel) -> Result<Vec<MarketplaceAgent>> {
        self.query(
            &format!("SELECT {COLUMNS} FROM marketplace_listings WHERE risk_level = ?1 ORDER BY install_count DESC"),
            params![serde_json::to_string(&risk)?],
        )
    }

    /// Run a SELECT returning listings (internal helper).
    fn query<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<Vec<MarketplaceAgent>> {
        let conn = self.conn.lock().expect("marketplace mutex poisoned");
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params, row_to_listing)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Install a listing into the given agent registry, returning the new
    /// agent record and incrementing the listing's install count.
    pub fn install(
        &self,
        listing_id: &str,
        registry: &AgentRegistry,
        name: &str,
        owner: &str,
        department: &str,
    ) -> Result<AgentRecord> {
        let mut listing = self.get(listing_id)?;
        let new_agent = listing
            .template
            .to_new_agent(name, listing.description.clone(), owner, department);
        let agent = registry.create(new_agent)?;
        listing.install_count += 1;
        self.upsert(&listing)?;
        cp_info!("marketplace.install", listing_id = %listing_id, agent_id = %agent.id);
        Ok(agent)
    }

    /// Fetch a listing by id.
    pub fn get(&self, id: &str) -> Result<MarketplaceAgent> {
        let conn = self.conn.lock().expect("marketplace mutex poisoned");
        conn.query_row(
            &format!("SELECT {COLUMNS} FROM marketplace_listings WHERE id = ?1"),
            params![id],
            row_to_listing,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => ControlPlaneError::not_found("listing", id),
            other => other.into(),
        })
    }

    /// Persist a listing (insert or replace). Internal helper.
    pub(crate) fn upsert(&self, l: &MarketplaceAgent) -> Result<()> {
        let conn = self.conn.lock().expect("marketplace mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO marketplace_listings (
                id, name, description, category, department, rating, install_count,
                risk_level, verification, compliance, template, published_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                l.id,
                l.name,
                l.description,
                l.category,
                l.department,
                l.rating,
                l.install_count as i64,
                serde_json::to_string(&l.risk_level)?,
                serde_json::to_string(&l.verification)?,
                serde_json::to_string(&l.compliance)?,
                serde_json::to_string(&l.template)?,
                l.published_at,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DataAccessLevel;
    use crate::marketplace::model::AgentTemplate;

    pub(super) fn listing() -> NewListing {
        NewListing {
            name: "Permit Intake".into(),
            description: "Triages permit applications".into(),
            category: "licensing".into(),
            department: "Licensing".into(),
            template: AgentTemplate {
                framework: "openclaw".into(),
                model_provider: "anthropic".into(),
                model_name: "claude-opus-4-8".into(),
                required_tools: vec!["search".into()],
                required_mcp_servers: vec!["records-mcp".into()],
                required_model_providers: vec!["anthropic".into()],
                data_access_level: DataAccessLevel::Internal,
                risk_level: RiskLevel::Medium,
            },
        }
    }

    #[test]
    fn publish_then_get() {
        let mkt = Marketplace::in_memory().unwrap();
        let l = mkt.publish(listing()).unwrap();
        assert_eq!(mkt.get(&l.id).unwrap().name, "Permit Intake");
        assert!(!l.is_trusted()); // unverified + pending on publish
    }
}
