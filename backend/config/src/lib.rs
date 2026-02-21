//! `clawforge-config` — ClawForge runtime configuration management.
//!
//! Provides:
//! - Typed config schema (all providers, agents, channels, security)
//! - YAML read/write with atomic backup rotation
//! - `${ENV_VAR}` substitution
//! - Legacy migration engine (4 versions)
//! - Config redaction for safe logging/display
//! - Default value application
//! - Deep schema validation

pub mod defaults;
pub mod env;
pub mod io;
pub mod migration;
pub mod redact;
pub mod schema;
pub mod validation;

// Re-export most-used types at crate root.
pub use schema::ClawForgeConfig;
pub use io::{config_dir, config_file_path, load_config, write_config, apply_merge_patch};
pub use env::{
    collect_referenced_vars, contains_env_var_reference, resolve_env_vars, resolve_env_vars_with,
    MissingEnvVarError,
};
pub use migration::{migrate, CURRENT_VERSION};
pub use redact::{redact, collect_redacted_paths};
pub use defaults::apply_all_defaults;
pub use validation::{validate, ValidationReport, ConfigValidationError};

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

/// Load, migrate, apply env substitution, and apply defaults to a config file.
///
/// This is the main entry point for loading a config at runtime.
pub async fn load_and_prepare(path: &Path) -> Result<ClawForgeConfig> {
    let raw_config = load_config(path).await?;

    // Serialize to Value for migration + env substitution pipeline.
    let mut value: Value = serde_json::to_value(&raw_config)
        .context("Failed to serialize config for processing")?;

    // Determine version from raw YAML (may differ from default).
    let version = value
        .get("_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;

    // Apply migrations.
    if version < CURRENT_VERSION {
        let (migrated, _mutated) = migrate(value, version)?;
        value = migrated;
    }

    // Substitute ${VAR} env vars.
    value = resolve_env_vars(&value).context("Failed to resolve env vars in config")?;

    // Deserialize back to typed config.
    let config: ClawForgeConfig =
        serde_json::from_value(value).context("Failed to deserialize config after processing")?;

    // Apply defaults.
    let config = apply_all_defaults(config);

    // Validate.
    let report = validate(&config);
    for warning in &report.warnings {
        tracing::warn!(path = %warning.path, message = %warning.message, "Config warning");
    }
    for error in &report.errors {
        tracing::error!(path = %error.path, message = %error.message, "Config error");
    }

    Ok(config)
}
