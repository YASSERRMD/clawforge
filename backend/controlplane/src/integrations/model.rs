//! Enterprise integration domain model.
//!
//! ClawForge governs *connections* to enterprise and government systems. The
//! control plane never stores secrets — only a [`CredentialRef`] pointing at
//! where the secret actually lives (a vault, an env var, an SSO provider).

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{LifecycleStatus, RiskLevel};

/// The category of an enterprise/government integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationKind {
    Oracle,
    SqlServer,
    Postgres,
    MongoDb,
    SharePoint,
    ServiceNow,
    ArcGis,
    ActiveDirectory,
    Sso,
    ApiGateway,
    Email,
    Webhook,
}

/// Where an integration's credentials are kept. This is a *reference*, never
/// the secret material itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRef {
    /// Backing secret store: `vault` | `env` | `sso` | `keychain` | `none`.
    pub store: String,
    /// Lookup key within that store (e.g. a vault path or env var name).
    pub key: String,
    /// Human-friendly note about the credential.
    #[serde(default)]
    pub description: String,
}

impl CredentialRef {
    /// A reference into a secrets vault.
    pub fn vault(path: impl Into<String>) -> Self {
        CredentialRef { store: "vault".into(), key: path.into(), description: String::new() }
    }

    /// A reference to an environment variable.
    pub fn env(var: impl Into<String>) -> Self {
        CredentialRef { store: "env".into(), key: var.into(), description: String::new() }
    }

    /// A placeholder for integrations that need no stored credential.
    pub fn none() -> Self {
        CredentialRef { store: "none".into(), key: String::new(), description: String::new() }
    }

    /// Whether this reference actually points at a secret store.
    pub fn is_present(&self) -> bool {
        self.store != "none" && !self.key.is_empty()
    }
}

/// An operation an integration is permitted to perform. Granting `Write`,
/// `Delete`, or `Admin` is what elevates an integration's risk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationPermission {
    /// Establish a connection / authenticate.
    Connect,
    /// Read data.
    Read,
    /// Write or update data.
    Write,
    /// Delete data.
    Delete,
    /// Administrative / privileged operations.
    Admin,
}

impl IntegrationPermission {
    /// Whether this permission grants mutating or privileged access.
    pub fn is_elevated(&self) -> bool {
        matches!(
            self,
            IntegrationPermission::Write | IntegrationPermission::Delete | IntegrationPermission::Admin
        )
    }
}

/// A registered enterprise/government integration and its governance metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationProvider {
    /// Stable UUID.
    pub id: String,
    pub name: String,
    pub kind: IntegrationKind,
    pub description: String,
    /// Accountable owner.
    pub owner: String,
    /// Owning department.
    pub department: String,
    /// Connection endpoint (host, URL, or service identifier).
    pub endpoint: String,
    /// Reference to where credentials live (never the secret itself).
    pub credential: CredentialRef,
    /// Operations this integration is permitted to perform.
    pub permissions: Vec<IntegrationPermission>,
    /// Assessed risk level.
    pub risk_level: RiskLevel,
    /// Governance status (`pending_approval` / `active` / `blocked` / …).
    pub status: LifecycleStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Input used to register a new integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewIntegration {
    pub name: String,
    pub kind: IntegrationKind,
    pub description: String,
    pub owner: String,
    pub department: String,
    pub endpoint: String,
    pub credential: CredentialRef,
    #[serde(default)]
    pub permissions: Vec<IntegrationPermission>,
    /// Explicit risk level; if omitted, callers can derive one from the kind.
    pub risk_level: RiskLevel,
}

impl IntegrationProvider {
    /// Materialise a fresh integration record; starts in `PendingApproval`.
    pub fn from_new(input: NewIntegration) -> Self {
        let now = Utc::now().timestamp();
        IntegrationProvider {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            kind: input.kind,
            description: input.description,
            owner: input.owner,
            department: input.department,
            endpoint: input.endpoint,
            credential: input.credential,
            permissions: input.permissions,
            risk_level: input.risk_level,
            status: LifecycleStatus::PendingApproval,
            created_at: now,
            updated_at: now,
        }
    }

    /// Whether this integration may currently be used.
    pub fn is_usable(&self) -> bool {
        self.status.is_operational()
    }

    /// Whether any granted permission is elevated (write/delete/admin).
    pub fn has_elevated_permission(&self) -> bool {
        self.permissions.iter().any(|p| p.is_elevated())
    }
}
