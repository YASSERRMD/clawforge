//! Config file read/write with atomic backup rotation.

use crate::schema::ClawForgeConfig;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// Default config file name within the config directory.
const CONFIG_FILE_NAME: &str = "config.yaml";

/// Number of rolling backups to keep.
const MAX_BACKUPS: usize = 5;

/// Resolve the ClawForge config directory.
/// Priority: `CLAWFORGE_CONFIG_DIR` env > `~/.clawforge/` > `~/.openclaw/`
pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CLAWFORGE_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(home) = dirs::home_dir() {
        let cf = home.join(".clawforge");
        if cf.exists() {
            return cf;
        }
        // Compatibility: also accept OpenClaw state dir
        let oc = home.join(".openclaw");
        if oc.exists() {
            return oc;
        }
        return cf; // default to .clawforge even if it doesn't exist yet
    }
    PathBuf::from(".clawforge")
}

/// Resolve the full path to the main config file.
pub fn config_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(CONFIG_FILE_NAME)
}

/// Load and parse the config from disk.
///
/// Returns `Ok(Default::default())` if the file doesn't exist (first run).
pub async fn load_config(path: &Path) -> Result<ClawForgeConfig> {
    if !path.exists() {
        debug!(path = %path.display(), "Config file does not exist; using defaults");
        return Ok(ClawForgeConfig::default());
    }

    let raw = fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: ClawForgeConfig = serde_yaml::from_str(&raw)
        .with_context(|| format!("Failed to parse config YAML at: {}", path.display()))?;

    info!(path = %path.display(), "Loaded config");
    Ok(config)
}

/// Write config to disk atomically (write to temp file, rename).
///
/// Creates a rolling backup of the previous config before overwriting.
pub async fn write_config(config: &ClawForgeConfig, path: &Path) -> Result<()> {
    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.with_context(|| {
            format!("Failed to create config directory: {}", parent.display())
        })?;
    }

    // Create backup of existing config.
    if path.exists() {
        rotate_backups(path).await?;
    }

    let yaml = serde_yaml::to_string(config)
        .with_context(|| "Failed to serialize config to YAML")?;

    // Write to temp file, then rename for atomicity.
    let tmp_path = path.with_extension("yaml.tmp");
    fs::write(&tmp_path, yaml.as_bytes())
        .await
        .with_context(|| format!("Failed to write temp config: {}", tmp_path.display()))?;

    fs::rename(&tmp_path, path).await.with_context(|| {
        format!("Failed to rename temp config to: {}", path.display())
    })?;

    info!(path = %path.display(), "Wrote config");
    Ok(())
}

/// Rotate backup files: config.yaml.bak.1 → .bak.2 → ... → .bak.N
async fn rotate_backups(path: &Path) -> Result<()> {
    // Shift existing backups up.
    for i in (1..MAX_BACKUPS).rev() {
        let old = path.with_extension(format!("yaml.bak.{}", i));
        let new = path.with_extension(format!("yaml.bak.{}", i + 1));
        if old.exists() {
            if let Err(e) = fs::rename(&old, &new).await {
                warn!("Failed to rotate backup {}: {}", old.display(), e);
            }
        }
    }

    // Copy current config to .bak.1
    let bak = path.with_extension("yaml.bak.1");
    if let Err(e) = fs::copy(path, &bak).await {
        warn!("Failed to create backup {}: {}", bak.display(), e);
    }

    Ok(())
}

/// Patch config with a JSON Merge Patch (RFC 7396).
///
/// The patch is applied to the serialized JSON of the config,
/// then deserialized back. This allows partial updates.
pub fn apply_merge_patch(config: &ClawForgeConfig, patch: &serde_json::Value) -> Result<ClawForgeConfig> {
    let mut value = serde_json::to_value(config)
        .context("Failed to serialize config for merge patch")?;
    json_merge_patch(&mut value, patch);
    let updated: ClawForgeConfig = serde_json::from_value(value)
        .context("Failed to deserialize config after merge patch")?;
    Ok(updated)
}

/// RFC 7396 JSON Merge Patch algorithm.
fn json_merge_patch(target: &mut serde_json::Value, patch: &serde_json::Value) {
    if let serde_json::Value::Object(patch_map) = patch {
        if let serde_json::Value::Object(target_map) = target {
            for (key, patch_val) in patch_map {
                if patch_val.is_null() {
                    target_map.remove(key);
                } else {
                    let entry = target_map
                        .entry(key.clone())
                        .or_insert(serde_json::Value::Null);
                    json_merge_patch(entry, patch_val);
                }
            }
        } else {
            // Target is not an object — replace entirely.
            *target = patch.clone();
        }
    } else {
        // Patch is a scalar/array — replace entirely.
        *target = patch.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_patch_adds_key() {
        let base: ClawForgeConfig = Default::default();
        let patch = serde_json::json!({ "logging": { "level": "debug" } });
        let result = apply_merge_patch(&base, &patch).unwrap();
        assert_eq!(result.logging.unwrap().level.unwrap(), "debug");
    }

    #[test]
    fn test_merge_patch_removes_key() {
        let mut base: ClawForgeConfig = Default::default();
        base.logging = Some(crate::schema::LoggingConfig {
            level: Some("info".to_string()),
            ..Default::default()
        });
        // Patch with null to remove
        let patch = serde_json::json!({ "logging": null });
        let result = apply_merge_patch(&base, &patch).unwrap();
        assert!(result.logging.is_none());
    }
}
