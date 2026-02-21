//! Auto-fix engine: applies safe automatic remediations to audited misconfigurations.
//!
//! Mirrors OpenClaw's `security/auto-fix.ts`.

use crate::channel_audit::{AuditFinding, AuditSeverity};
use serde_json::Value;
use tracing::info;

/// Result of applying an auto-fix.
#[derive(Debug, Clone)]
pub struct AutoFixResult {
    pub finding_code: String,
    pub applied: bool,
    pub description: String,
}

/// Apply all auto-fixable findings to a mutable config JSON value.
///
/// Returns a list of applied (and skipped) fix results.
pub fn auto_fix(config: &mut Value, findings: &[AuditFinding]) -> Vec<AutoFixResult> {
    let mut results = Vec::new();

    for finding in findings.iter().filter(|f| f.auto_fixable) {
        let result = apply_fix(config, finding);
        results.push(result);
    }

    results
}

fn apply_fix(config: &mut Value, finding: &AuditFinding) -> AutoFixResult {
    match finding.code.as_str() {
        // Set default ackReactionScope if missing.
        "CFG001" => {
            if let Some(msgs) = config.get_mut("messages") {
                if msgs.get("ackReactionScope").is_none() {
                    if let Some(obj) = msgs.as_object_mut() {
                        obj.insert(
                            "ackReactionScope".to_string(),
                            Value::String("group-mentions".to_string()),
                        );
                        info!(code = "CFG001", "Auto-fixed: set ackReactionScope=group-mentions");
                        return AutoFixResult {
                            finding_code: finding.code.clone(),
                            applied: true,
                            description: "Set ackReactionScope to 'group-mentions'".to_string(),
                        };
                    }
                }
            }
            AutoFixResult {
                finding_code: finding.code.clone(),
                applied: false,
                description: "Could not apply CFG001 fix".to_string(),
            }
        }

        // Set default compaction mode if missing.
        "CFG002" => {
            if let Some(agents) = config.get_mut("agents") {
                if let Some(defaults) = agents.get_mut("defaults") {
                    if defaults.get("compaction").and_then(|c| c.get("mode")).is_none() {
                        let compaction = defaults
                            .get_mut("compaction")
                            .and_then(|v| v.as_object_mut());
                        if let Some(c) = compaction {
                            c.insert("mode".to_string(), Value::String("safeguard".to_string()));
                        } else if let Some(d) = defaults.as_object_mut() {
                            d.insert(
                                "compaction".to_string(),
                                serde_json::json!({ "mode": "safeguard" }),
                            );
                        }
                        info!(code = "CFG002", "Auto-fixed: set compaction.mode=safeguard");
                        return AutoFixResult {
                            finding_code: finding.code.clone(),
                            applied: true,
                            description: "Set compaction.mode to 'safeguard'".to_string(),
                        };
                    }
                }
            }
            AutoFixResult {
                finding_code: finding.code.clone(),
                applied: false,
                description: "Could not apply CFG002 fix".to_string(),
            }
        }

        _ => AutoFixResult {
            finding_code: finding.code.clone(),
            applied: false,
            description: format!("No auto-fix available for code '{}'", finding.code),
        },
    }
}

/// Check if any critical or high findings remain after auto-fix.
pub fn has_blocking_findings(findings: &[AuditFinding], applied: &[AutoFixResult]) -> bool {
    let fixed_codes: std::collections::HashSet<&str> =
        applied.iter().filter(|r| r.applied).map(|r| r.finding_code.as_str()).collect();

    findings.iter().any(|f| {
        matches!(f.severity, AuditSeverity::High | AuditSeverity::Critical)
            && !fixed_codes.contains(f.code.as_str())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel_audit::AuditFinding;

    #[test]
    fn auto_fix_cfg001() {
        let mut config = serde_json::json!({ "messages": {} });
        let findings = vec![AuditFinding {
            severity: AuditSeverity::Low,
            code: "CFG001".into(),
            title: "Missing ackReactionScope".into(),
            description: "".into(),
            field_path: None,
            auto_fixable: true,
        }];
        let results = auto_fix(&mut config, &findings);
        assert!(results[0].applied);
        assert_eq!(config["messages"]["ackReactionScope"], "group-mentions");
    }
}
