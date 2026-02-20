/// Plugin registry — loads and tracks installed plugins.
///
/// Mirrors `src/plugins/registry.ts` from OpenClaw.
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::manifest::PluginManifest;

#[derive(Default)]
pub struct PluginRegistry {
    plugins: HashMap<String, LoadedPlugin>,
    plugins_dir: PathBuf,
}

pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub install_path: PathBuf,
    pub enabled: bool,
}

impl PluginRegistry {
    pub fn new(plugins_dir: impl Into<PathBuf>) -> Self {
        Self { plugins: HashMap::new(), plugins_dir: plugins_dir.into() }
    }

    /// Discover and load all plugins from the plugins directory.
    pub fn discover(&mut self) -> Result<usize> {
        let dir = &self.plugins_dir;
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in std::fs::read_dir(dir).context("read plugins dir")? {
            let entry = entry?;
            let plugin_path = entry.path();
            if !plugin_path.is_dir() {
                continue;
            }
            match self.load_from_path(&plugin_path) {
                Ok(manifest) => {
                    info!("[Plugins] Loaded: {} v{}", manifest.id, manifest.version);
                    self.plugins.insert(manifest.id.clone(), LoadedPlugin {
                        manifest,
                        install_path: plugin_path,
                        enabled: true,
                    });
                    count += 1;
                }
                Err(e) => {
                    warn!("[Plugins] Failed to load {:?}: {}", entry.file_name(), e);
                }
            }
        }
        Ok(count)
    }

    fn load_from_path(&self, path: &Path) -> Result<PluginManifest> {
        let manifest_path = path.join("clawforge-plugin.json");
        let raw = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("read manifest at {:?}", manifest_path))?;
        let manifest: PluginManifest = serde_json::from_str(&raw)
            .context("parse plugin manifest")?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn get(&self, id: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(id)
    }

    pub fn list(&self) -> Vec<&PluginManifest> {
        self.plugins.values().map(|p| &p.manifest).collect()
    }

    pub fn enable(&mut self, id: &str) -> bool {
        if let Some(p) = self.plugins.get_mut(id) { p.enabled = true; true } else { false }
    }

    pub fn disable(&mut self, id: &str) -> bool {
        if let Some(p) = self.plugins.get_mut(id) { p.enabled = false; true } else { false }
    }

    pub fn unload(&mut self, id: &str) -> bool {
        self.plugins.remove(id).is_some()
    }
}
