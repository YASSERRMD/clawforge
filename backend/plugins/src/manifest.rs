/// Plugin manifest — describes a ClawForge plugin package.
///
/// Mirrors `src/plugins/manifest.ts` from OpenClaw.
use serde::{Deserialize, Serialize};

/// The permissions a plugin requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub network: bool,
    pub filesystem: bool,
    pub shell: bool,
}

/// A tool slot exposed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolSlot {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// A hook entry declared by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHookEntry {
    pub phase: String,
    pub handler: String,
}

/// Full plugin manifest (parsed from `clawforge-plugin.json` in the package).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub homepage: Option<String>,
    /// Entry point script relative to package root.
    pub main: String,
    pub permissions: PluginPermissions,
    #[serde(default)]
    pub tools: Vec<PluginToolSlot>,
    #[serde(default)]
    pub hooks: Vec<PluginHookEntry>,
    /// Channels this plugin is compatible with (empty = all).
    #[serde(default)]
    pub channels: Vec<String>,
}

impl PluginManifest {
    /// Validate the manifest for required fields.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.id.is_empty() {
            anyhow::bail!("Plugin manifest missing 'id'");
        }
        if self.name.is_empty() {
            anyhow::bail!("Plugin manifest missing 'name'");
        }
        if self.main.is_empty() {
            anyhow::bail!("Plugin manifest missing 'main'");
        }
        Ok(())
    }
}
