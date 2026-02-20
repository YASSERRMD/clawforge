/// Exec approval safety — static analysis + safe-bin allowlist.
///
/// Mirrors `src/infra/exec-approvals*.ts` from OpenClaw.
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Safe-bin allowlist
// ---------------------------------------------------------------------------

/// Binaries that are considered safe to run without approval.
static SAFE_BINS: &[&str] = &[
    "echo", "cat", "ls", "pwd", "which", "whoami", "date", "uname",
    "grep", "sed", "awk", "sort", "uniq", "wc", "head", "tail",
    "find", "locate", "file", "stat", "du", "df", "ps",
    "git", "cargo", "npm", "yarn", "pnpm", "node", "python3",
    "make", "cmake", "gcc", "clang", "rustc",
];

/// Regex patterns for dangerous command patterns.
static DANGEROUS_PATTERNS: &[&str] = &[
    r"rm\s+(-r|-f|-rf|-fr)\s",
    r"sudo\s",
    r"chmod\s+[0-7]*7",      // world-writable
    r"curl\s.*\|\s*(?:bash|sh|zsh)",
    r"wget\s.*\|\s*(?:bash|sh|zsh)",
    r"\bdd\b.*of=",          // disk overwrite
    r"mkfs\.",               // format disk
    r":\(\)\{.*\}",         // fork bomb
    r"base64.*decode.*\|.*sh",
];

// ---------------------------------------------------------------------------
// Approval result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalVerdict {
    /// Command is safe to run without approval.
    Safe,
    /// Command requires human approval before execution.
    RequiresApproval { reason: String },
    /// Command is blocked outright.
    Blocked { reason: String },
}

// ---------------------------------------------------------------------------
// Analyzer
// ---------------------------------------------------------------------------

pub struct ExecApprovalAnalyzer {
    dangerous_regexes: Vec<Regex>,
}

impl ExecApprovalAnalyzer {
    pub fn new() -> Result<Self> {
        let dangerous_regexes = DANGEROUS_PATTERNS
            .iter()
            .map(|p| Regex::new(p).map_err(|e| anyhow::anyhow!("bad regex {}: {}", p, e)))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { dangerous_regexes })
    }

    /// Analyze a command string and return an approval verdict.
    pub fn analyze(&self, command: &str) -> ApprovalVerdict {
        // 1. Check dangerous patterns first (always block/flag)
        for re in &self.dangerous_regexes {
            if re.is_match(command) {
                warn!("[Sandbox] Dangerous pattern matched: {:?}", re.as_str());
                return ApprovalVerdict::Blocked {
                    reason: format!("Dangerous pattern detected: {}", re.as_str()),
                };
            }
        }

        // 2. Extract the base binary name
        let bin = command.split_whitespace().next().unwrap_or("");
        let bin_name = std::path::Path::new(bin)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // 3. Check safe-bin allowlist
        if SAFE_BINS.contains(&bin_name.as_str()) {
            info!("[Sandbox] Safe bin: {}", bin_name);
            return ApprovalVerdict::Safe;
        }

        // 4. Unknown binary — require approval
        ApprovalVerdict::RequiresApproval {
            reason: format!("Binary '{}' is not on the safe-bin allowlist", bin_name),
        }
    }
}

impl Default for ExecApprovalAnalyzer {
    fn default() -> Self {
        Self::new().expect("valid patterns")
    }
}
