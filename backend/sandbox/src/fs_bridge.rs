//! File-system bridge: sync host workspace files in/out of sandbox containers.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Manages host ↔ container path mappings for a sandbox session.
pub struct FsBridge {
    /// Host-side workspace root.
    host_workspace: PathBuf,
    /// Container-side workspace mount point.
    container_workspace: PathBuf,
}

impl FsBridge {
    pub fn new(host_workspace: impl Into<PathBuf>, container_workspace: impl Into<PathBuf>) -> Self {
        Self {
            host_workspace: host_workspace.into(),
            container_workspace: container_workspace.into(),
        }
    }

    /// Translate a host path to the equivalent container path.
    pub fn host_to_container(&self, host_path: &Path) -> Option<PathBuf> {
        let rel = host_path.strip_prefix(&self.host_workspace).ok()?;
        Some(self.container_workspace.join(rel))
    }

    /// Translate a container path to the equivalent host path.
    pub fn container_to_host(&self, container_path: &Path) -> Option<PathBuf> {
        let rel = container_path.strip_prefix(&self.container_workspace).ok()?;
        Some(self.host_workspace.join(rel))
    }

    /// Copy a list of host files into the container workspace.
    pub async fn sync_in(
        &self,
        container_id: &str,
        files: &[PathBuf],
    ) -> Result<()> {
        for file in files {
            let container_path = self
                .host_to_container(file)
                .with_context(|| format!("Path {} is outside workspace", file.display()))?;

            // Ensure parent dir exists inside container.
            if let Some(parent) = container_path.parent() {
                let _ = tokio::process::Command::new("docker")
                    .args(["exec", container_id, "mkdir", "-p", &parent.to_string_lossy()])
                    .output()
                    .await;
            }

            debug!(src = %file.display(), dst = %container_path.display(), "Syncing file → container");
            let output = tokio::process::Command::new("docker")
                .args(["cp", &file.to_string_lossy(), &format!("{container_id}:{}", container_path.display())])
                .output()
                .await
                .context("docker cp failed")?;

            if !output.status.success() {
                anyhow::bail!(
                    "Failed to copy {} into container: {}",
                    file.display(),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        info!(count = files.len(), container = %container_id, "Synced files → container");
        Ok(())
    }

    /// Copy a list of container-side files back to the host workspace.
    pub async fn sync_out(
        &self,
        container_id: &str,
        container_files: &[PathBuf],
    ) -> Result<Vec<PathBuf>> {
        let mut synced = Vec::new();
        for cf in container_files {
            let host_path = self
                .container_to_host(cf)
                .with_context(|| format!("Container path {} is outside workspace", cf.display()))?;

            // Ensure host parent dir exists.
            if let Some(parent) = host_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            debug!(src = %cf.display(), dst = %host_path.display(), "Syncing file ← container");
            let output = tokio::process::Command::new("docker")
                .args(["cp", &format!("{container_id}:{}", cf.display()), &host_path.to_string_lossy()])
                .output()
                .await
                .context("docker cp failed")?;

            if !output.status.success() {
                anyhow::bail!(
                    "Failed to copy {} from container: {}",
                    cf.display(),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            synced.push(host_path);
        }
        info!(count = synced.len(), container = %container_id, "Synced files ← container");
        Ok(synced)
    }

    pub fn host_workspace(&self) -> &Path {
        &self.host_workspace
    }

    pub fn container_workspace(&self) -> &Path {
        &self.container_workspace
    }
}
