//! Channel security audit: validates channel configurations for security misconfigs.
//!
//! Mirrors OpenClaw's `security/channel-audit.ts`.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Severity of a security finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// A single audit finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFinding {
    pub severity: AuditSeverity,
    pub code: String,
    pub title: String,
    pub description: String,
    pub field_path: Option<String>,
    pub auto_fixable: bool,
}

/// Result of auditing a channel configuration.
#[derive(Debug, Clone, Serialize)]
pub struct ChannelAuditResult {
    pub channel: String,
    pub findings: Vec<AuditFinding>,
    pub passed: bool,
}

impl ChannelAuditResult {
    pub fn is_healthy(&self) -> bool {
        self.findings
            .iter()
            .all(|f| matches!(f.severity, AuditSeverity::Info | AuditSeverity::Low))
    }
}

/// Audit a Telegram channel configuration JSON.
pub fn audit_telegram(config: &serde_json::Value) -> ChannelAuditResult {
    let mut findings = Vec::new();

    // Check bot token is present.
    if config.get("botToken").and_then(|v| v.as_str()).map(str::is_empty).unwrap_or(true) {
        findings.push(AuditFinding {
            severity: AuditSeverity::Critical,
            code: "TG001".into(),
            title: "Missing bot token".into(),
            description: "Telegram botToken is required for the channel to operate.".into(),
            field_path: Some("channels.telegram.botToken".into()),
            auto_fixable: false,
        });
    }

    // Check allowFrom is set (prevent open access).
    if config.get("allowFrom").is_none() {
        findings.push(AuditFinding {
            severity: AuditSeverity::High,
            code: "TG002".into(),
            title: "No allowFrom restriction".into(),
            description: "allowFrom is not configured; any user can interact with this bot.".into(),
            field_path: Some("channels.telegram.allowFrom".into()),
            auto_fixable: false,
        });
    }

    // Check webhookSecret is set (if webhook mode).
    let mode = config.get("mode").and_then(|v| v.as_str()).unwrap_or("polling");
    if mode == "webhook" && config.get("webhookSecret").and_then(|v| v.as_str()).map(str::is_empty).unwrap_or(true) {
        findings.push(AuditFinding {
            severity: AuditSeverity::High,
            code: "TG003".into(),
            title: "Webhook without secret".into(),
            description: "Webhook mode is enabled but no webhookSecret is set.".into(),
            field_path: Some("channels.telegram.webhookSecret".into()),
            auto_fixable: false,
        });
    }

    let passed = !findings.iter().any(|f| {
        matches!(f.severity, AuditSeverity::High | AuditSeverity::Critical)
    });

    ChannelAuditResult { channel: "telegram".into(), findings, passed }
}

/// Audit a Discord channel configuration JSON.
pub fn audit_discord(config: &serde_json::Value) -> ChannelAuditResult {
    let mut findings = Vec::new();

    if config.get("botToken").and_then(|v| v.as_str()).map(str::is_empty).unwrap_or(true) {
        findings.push(AuditFinding {
            severity: AuditSeverity::Critical,
            code: "DC001".into(),
            title: "Missing bot token".into(),
            description: "Discord botToken is required.".into(),
            field_path: Some("channels.discord.botToken".into()),
            auto_fixable: false,
        });
    }

    // Check applicationId is present (needed for slash commands).
    if config.get("applicationId").and_then(|v| v.as_str()).is_none() {
        findings.push(AuditFinding {
            severity: AuditSeverity::Medium,
            code: "DC002".into(),
            title: "Missing applicationId".into(),
            description: "applicationId is needed for slash command registration.".into(),
            field_path: Some("channels.discord.applicationId".into()),
            auto_fixable: false,
        });
    }

    let passed = !findings.iter().any(|f| {
        matches!(f.severity, AuditSeverity::High | AuditSeverity::Critical)
    });

    ChannelAuditResult { channel: "discord".into(), findings, passed }
}

/// Audit a Slack channel configuration JSON.
pub fn audit_slack(config: &serde_json::Value) -> ChannelAuditResult {
    let mut findings = Vec::new();

    if config.get("botToken").and_then(|v| v.as_str()).map(str::is_empty).unwrap_or(true) {
        findings.push(AuditFinding {
            severity: AuditSeverity::Critical,
            code: "SL001".into(),
            title: "Missing bot token".into(),
            description: "Slack botToken is required.".into(),
            field_path: Some("channels.slack.botToken".into()),
            auto_fixable: false,
        });
    }

    if config.get("signingSecret").and_then(|v| v.as_str()).map(str::is_empty).unwrap_or(true) {
        findings.push(AuditFinding {
            severity: AuditSeverity::High,
            code: "SL002".into(),
            title: "Missing signing secret".into(),
            description: "signingSecret is required to verify Slack request signatures.".into(),
            field_path: Some("channels.slack.signingSecret".into()),
            auto_fixable: false,
        });
    }

    let passed = !findings.iter().any(|f| {
        matches!(f.severity, AuditSeverity::High | AuditSeverity::Critical)
    });

    ChannelAuditResult { channel: "slack".into(), findings, passed }
}

/// Run channel audits across all configured channels.
pub fn audit_all_channels(channels: &serde_json::Value) -> Vec<ChannelAuditResult> {
    let mut results = Vec::new();

    if let Some(tg) = channels.get("telegram") {
        results.push(audit_telegram(tg));
    }
    if let Some(dc) = channels.get("discord") {
        results.push(audit_discord(dc));
    }
    if let Some(sl) = channels.get("slack") {
        results.push(audit_slack(sl));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_missing_telegram_token() {
        let cfg = json!({});
        let result = audit_telegram(&cfg);
        assert!(!result.passed);
        assert!(result.findings.iter().any(|f| f.code == "TG001"));
    }

    #[test]
    fn passes_valid_telegram_config() {
        let cfg = json!({
            "botToken": "1234567890:ABC",
            "allowFrom": ["+1234567890"]
        });
        let result = audit_telegram(&cfg);
        assert!(result.passed);
    }
}
