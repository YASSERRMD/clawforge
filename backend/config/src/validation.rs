//! Config validation: deep schema checks with user-friendly error messages.

use crate::schema::ClawForgeConfig;
use thiserror::Error;

/// A config validation error with field path and message.
#[derive(Debug, Error)]
#[error("Config validation error at '{path}': {message}")]
pub struct ConfigValidationError {
    pub path: String,
    pub message: String,
}

/// A collection of validation errors found in one pass.
#[derive(Debug, Default)]
pub struct ValidationReport {
    pub errors: Vec<ConfigValidationError>,
    pub warnings: Vec<ConfigValidationError>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    fn error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.errors.push(ConfigValidationError {
            path: path.into(),
            message: message.into(),
        });
    }

    fn warn(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.warnings.push(ConfigValidationError {
            path: path.into(),
            message: message.into(),
        });
    }
}

/// Validate the config and return a report of all errors and warnings.
pub fn validate(config: &ClawForgeConfig) -> ValidationReport {
    let mut report = ValidationReport::default();
    validate_auth(config, &mut report);
    validate_models(config, &mut report);
    validate_gateway(config, &mut report);
    validate_channels(config, &mut report);
    validate_agents(config, &mut report);
    validate_memory(config, &mut report);
    report
}

/// Validate that at least one auth profile is configured.
fn validate_auth(config: &ClawForgeConfig, report: &mut ValidationReport) {
    let Some(auth) = &config.auth else { return };
    if auth.profiles.is_empty() {
        report.warn("auth.profiles", "No auth profiles configured; agent calls will fail");
    }
    for (id, _profile) in &auth.profiles {
        if id.trim().is_empty() {
            report.error("auth.profiles", "Profile ID cannot be empty");
        }
    }
}

/// Validate model provider configurations.
fn validate_models(config: &ClawForgeConfig, report: &mut ValidationReport) {
    let Some(models) = &config.models else { return };
    for (provider_id, provider) in &models.providers {
        let path = format!("models.providers.{provider_id}");
        if provider.models.is_empty() && provider.disabled != Some(true) {
            report.warn(&path, "Provider has no models defined");
        }
        for model in &provider.models {
            if model.id.trim().is_empty() {
                report.error(format!("{path}.models"), "Model id cannot be empty");
            }
            if let Some(ctx) = model.context_window {
                if ctx == 0 {
                    report.error(
                        format!("{path}.models.{id}", id = model.id),
                        "contextWindow must be > 0",
                    );
                }
            }
        }
    }
}

/// Validate gateway configuration.
fn validate_gateway(config: &ClawForgeConfig, report: &mut ValidationReport) {
    let Some(gw) = &config.gateway else { return };
    if let Some(port) = gw.port {
        if port < 1024 && port != 80 && port != 443 {
            report.warn(
                "gateway.port",
                format!("Port {port} requires elevated privileges; consider using a port >= 1024"),
            );
        }
    }
    if let Some(tls) = &gw.tls {
        if tls.cert.is_none() || tls.key.is_none() {
            report.error("gateway.tls", "Both cert and key are required for TLS");
        }
    }
}

/// Validate channel configurations.
fn validate_channels(config: &ClawForgeConfig, report: &mut ValidationReport) {
    let Some(channels) = &config.channels else { return };

    if let Some(tg) = &channels.telegram {
        if tg.bot_token.as_deref().map(str::is_empty).unwrap_or(true) {
            report.error("channels.telegram.botToken", "Telegram bot token is required");
        }
    }

    if let Some(dc) = &channels.discord {
        if dc.bot_token.as_deref().map(str::is_empty).unwrap_or(true) {
            report.error("channels.discord.botToken", "Discord bot token is required");
        }
    }

    if let Some(sl) = &channels.slack {
        if sl.bot_token.as_deref().map(str::is_empty).unwrap_or(true) {
            report.error("channels.slack.botToken", "Slack bot token is required");
        }
    }
}

/// Validate agent configuration.
fn validate_agents(config: &ClawForgeConfig, report: &mut ValidationReport) {
    let Some(agents) = &config.agents else { return };
    if let Some(defaults) = &agents.defaults {
        if let Some(mc) = defaults.max_concurrent {
            if mc == 0 {
                report.error("agents.defaults.maxConcurrent", "maxConcurrent must be >= 1");
            }
        }
        if let Some(sandbox) = &defaults.sandbox {
            if let Some(driver) = &sandbox.driver {
                if !matches!(driver.as_str(), "none" | "docker" | "bwrap") {
                    report.error(
                        "agents.defaults.sandbox.driver",
                        format!("Unknown sandbox driver '{driver}'. Use 'none', 'docker', or 'bwrap'"),
                    );
                }
            }
        }
    }
}

/// Validate memory configuration.
fn validate_memory(config: &ClawForgeConfig, report: &mut ValidationReport) {
    let Some(memory) = &config.memory else { return };
    for (i, coll) in memory.collections.iter().enumerate() {
        if coll.name.trim().is_empty() {
            report.error(format!("memory.collections[{i}].name"), "Collection name cannot be empty");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{GatewayConfig, GatewayTls};

    #[test]
    fn empty_config_is_valid() {
        let report = validate(&ClawForgeConfig::default());
        assert!(report.is_valid(), "errors: {:?}", report.errors);
    }

    #[test]
    fn tls_missing_key_is_error() {
        let mut cfg = ClawForgeConfig::default();
        cfg.gateway = Some(GatewayConfig {
            tls: Some(GatewayTls {
                cert: Some("/path/to/cert.pem".to_string()),
                key: None,
            }),
            ..Default::default()
        });
        let report = validate(&cfg);
        assert!(!report.is_valid());
        assert!(report.errors[0].path.contains("tls"));
    }
}
