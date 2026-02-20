/// Plugin installer — download and install plugin packages.
///
/// Mirrors `src/plugins/install.ts` from OpenClaw.
/// Plugins are distributed as tarballs or directories.
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use tracing::info;

pub struct PluginInstaller {
    pub plugins_dir: PathBuf,
}

impl PluginInstaller {
    pub fn new(plugins_dir: impl Into<PathBuf>) -> Self {
        Self { plugins_dir: plugins_dir.into() }
    }

    /// Install a plugin from a local directory path (copy into plugins_dir).
    pub fn install_from_dir(&self, source: &Path) -> Result<PathBuf> {
        if !source.is_dir() {
            bail!("Source is not a directory: {:?}", source);
        }
        let name = source.file_name()
            .context("source has no filename")?
            .to_string_lossy()
            .to_string();
        let dest = self.plugins_dir.join(&name);
        if dest.exists() {
            bail!("Plugin '{}' is already installed", name);
        }
        std::fs::create_dir_all(&self.plugins_dir)?;
        copy_dir(source, &dest).context("copy plugin dir")?;
        info!("[Installer] Installed plugin '{}' → {:?}", name, dest);
        Ok(dest)
    }

    /// Uninstall a plugin by ID (removes its directory).
    pub fn uninstall(&self, plugin_id: &str) -> Result<()> {
        let path = self.plugins_dir.join(plugin_id);
        if !path.exists() {
            bail!("Plugin '{}' not found in {:?}", plugin_id, self.plugins_dir);
        }
        std::fs::remove_dir_all(&path)
            .with_context(|| format!("remove plugin dir {:?}", path))?;
        info!("[Installer] Uninstalled plugin '{}'", plugin_id);
        Ok(())
    }

    /// List installed plugin directories.
    pub fn list_installed(&self) -> Result<Vec<String>> {
        if !self.plugins_dir.exists() {
            return Ok(vec![]);
        }
        let names = std::fs::read_dir(&self.plugins_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        Ok(names)
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}
