//! Permissions Enforcer
//!
//! Strictly audits capabilities demanded by a plugin against the scopes approved by the instance admin.

use anyhow::{bail, Result};
use tracing::warn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionScope {
    ReadFilesystem,
    WriteFilesystem,
    NetworkAccess,
    ExecuteShell,
    ReadEnvironment,
}

pub struct PermissionEnforcer {
    granted_scopes: Vec<PermissionScope>,
}

impl PermissionEnforcer {
    pub fn new(scopes: Vec<PermissionScope>) -> Self {
        Self { granted_scopes: scopes }
    }

    /// Asserts the plugin has adequate authorization to invoke a sensitive API.
    pub fn assert_permission(&self, required: PermissionScope) -> Result<()> {
        if !self.granted_scopes.contains(&required) {
            warn!("Plugin permission denied. Required: {:?}", required);
            bail!("Unauthorized: Plugin lacks the {:?} permission.", required);
        }
        Ok(())
    }
}
