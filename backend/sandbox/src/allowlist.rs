//! Exec approval allowlist — persists and evaluates command approval rules.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

/// Security level for an approval entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalLevel {
    /// Always allow this pattern.
    Allow,
    /// Always deny this pattern.
    Deny,
    /// Ask the user (default for unknown commands).
    Ask,
}

/// A single allowlist entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowlistEntry {
    /// Glob pattern matching the command (e.g., `ls *`, `git diff *`).
    pub pattern: String,
    /// The verdict to apply when this pattern matches.
    pub level: ApprovalLevel,
    /// Human-readable reason for this entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Timestamp when this entry was added (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added_at: Option<String>,
    /// Scope: "session" (temporary) or "persistent" (saved to disk).
    #[serde(default = "default_scope")]
    pub scope: String,
}

fn default_scope() -> String {
    "persistent".to_string()
}

/// In-memory + on-disk allowlist store.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecAllowlist {
    pub version: u32,
    #[serde(default)]
    pub entries: Vec<AllowlistEntry>,
    /// Socket path for interactive approval IPC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_path: Option<String>,
    /// Auth token for socket IPC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_token: Option<String>,
}

impl ExecAllowlist {
    /// Create an empty allowlist with sensible safe defaults.
    pub fn with_safe_defaults() -> Self {
        let safe_bins = [
            "ls", "cat", "echo", "pwd", "date", "whoami", "which", "dirname",
            "basename", "head", "tail", "wc", "sort", "uniq", "grep", "find",
            "git status", "git log", "git diff", "git show",
        ];

        let entries = safe_bins
            .iter()
            .map(|cmd| AllowlistEntry {
                pattern: format!("{cmd}*"),
                level: ApprovalLevel::Allow,
                reason: Some("Safe read-only command".to_string()),
                added_at: None,
                scope: "persistent".to_string(),
            })
            .collect();

        ExecAllowlist {
            version: 1,
            entries,
            socket_path: None,
            socket_token: None,
        }
    }

    /// Add or update an entry for a pattern.
    pub fn upsert(&mut self, entry: AllowlistEntry) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.pattern == entry.pattern) {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// Remove an entry by pattern.
    pub fn remove(&mut self, pattern: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.pattern != pattern);
        self.entries.len() < before
    }

    /// Evaluate a command string against the allowlist.
    ///
    /// Returns the first matching entry's level, or `Ask` if no match.
    pub fn evaluate(&self, command: &str) -> ApprovalLevel {
        for entry in &self.entries {
            if glob_matches(&entry.pattern, command) {
                debug!(
                    command = %command,
                    pattern = %entry.pattern,
                    level = ?entry.level,
                    "Allowlist match"
                );
                return entry.level.clone();
            }
        }
        ApprovalLevel::Ask
    }

    /// Load allowlist from disk.
    pub async fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            debug!("Allowlist file not found, using defaults: {}", path.display());
            return Ok(Self::with_safe_defaults());
        }
        let raw = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read allowlist: {}", path.display()))?;
        let list: Self = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse allowlist JSON: {}", path.display()))?;
        info!("Loaded exec allowlist ({} entries)", list.entries.len());
        Ok(list)
    }

    /// Save allowlist to disk (writes persistent entries only).
    pub async fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let persistent = ExecAllowlist {
            entries: self
                .entries
                .iter()
                .filter(|e| e.scope != "session")
                .cloned()
                .collect(),
            ..self.clone()
        };
        let json = serde_json::to_string_pretty(&persistent)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json.as_bytes()).await?;
        fs::rename(&tmp, path).await?;
        info!("Saved exec allowlist ({} entries)", persistent.entries.len());
        Ok(())
    }

    /// Add a session-scoped (temporary) allow entry.
    pub fn allow_for_session(&mut self, command: &str) {
        self.upsert(AllowlistEntry {
            pattern: command.to_string(),
            level: ApprovalLevel::Allow,
            reason: Some("Session approval".to_string()),
            added_at: None,
            scope: "session".to_string(),
        });
    }

    /// Drop all session-scoped entries (called at session end).
    pub fn clear_session_entries(&mut self) {
        self.entries.retain(|e| e.scope != "session");
    }
}

/// Simple glob pattern matching.
/// Supports `*` (any substring) and `?` (any single character).
fn glob_matches(pattern: &str, input: &str) -> bool {
    glob_match_recursive(pattern.as_bytes(), input.as_bytes())
}

fn glob_match_recursive(pattern: &[u8], input: &[u8]) -> bool {
    match (pattern.first(), input.first()) {
        (None, None) => true,
        (Some(b'*'), _) => {
            // Try consuming zero or more characters.
            glob_match_recursive(&pattern[1..], input)
                || (!input.is_empty() && glob_match_recursive(pattern, &input[1..]))
        }
        (Some(b'?'), Some(_)) => glob_match_recursive(&pattern[1..], &input[1..]),
        (Some(p), Some(i)) if p.to_ascii_lowercase() == i.to_ascii_lowercase() => {
            glob_match_recursive(&pattern[1..], &input[1..])
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_allow() {
        let mut list = ExecAllowlist::default();
        list.upsert(AllowlistEntry {
            pattern: "ls *".to_string(),
            level: ApprovalLevel::Allow,
            reason: None,
            added_at: None,
            scope: "persistent".to_string(),
        });
        assert_eq!(list.evaluate("ls /tmp"), ApprovalLevel::Allow);
    }

    #[test]
    fn evaluates_deny() {
        let mut list = ExecAllowlist::default();
        list.upsert(AllowlistEntry {
            pattern: "rm -rf*".to_string(),
            level: ApprovalLevel::Deny,
            reason: None,
            added_at: None,
            scope: "persistent".to_string(),
        });
        assert_eq!(list.evaluate("rm -rf /"), ApprovalLevel::Deny);
    }

    #[test]
    fn unknown_command_asks() {
        let list = ExecAllowlist::default();
        assert_eq!(list.evaluate("unknown-bin --args"), ApprovalLevel::Ask);
    }

    #[test]
    fn safe_defaults_allow_git_status() {
        let list = ExecAllowlist::with_safe_defaults();
        assert_eq!(list.evaluate("git status"), ApprovalLevel::Allow);
    }
}
