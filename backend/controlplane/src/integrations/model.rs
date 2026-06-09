//! Enterprise integration domain model.
//!
//! ClawForge governs *connections* to enterprise and government systems. The
//! control plane never stores secrets — only a [`CredentialRef`] pointing at
//! where the secret actually lives (a vault, an env var, an SSO provider).

use serde::{Deserialize, Serialize};

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
