/// Dangerous tool blocklist — tools that must always require explicit HITL approval.
///
/// Mirrors `src/security/dangerous-tools.ts` from the OpenClaw reference.
use std::collections::HashSet;

/// Returns the set of tool names that are considered dangerous and must never
/// be auto-approved. These always require explicit human approval.
pub fn dangerous_tools() -> HashSet<&'static str> {
    [
        // Destructive filesystem operations
        "rm",
        "delete",
        "file_delete",
        "file_write",
        "patch_file",
        // Arbitrary code execution
        "bash",
        "shell",
        "exec",
        "run_command",
        // Network + exfiltration risk
        "http_post",
        "http_put",
        "http_patch",
        "http_delete",
        // Database mutation
        "db_write",
        "db_execute",
        // Credential / secret access
        "secret_read",
        "keychain_get",
    ]
    .into_iter()
    .collect()
}

/// Returns true if a tool requires a human approval step.
pub fn is_dangerous(tool_name: &str) -> bool {
    dangerous_tools().contains(tool_name.to_lowercase().as_str())
}

/// Auto-approvable tool kinds (safe to run without human prompt).
pub fn safe_kinds() -> HashSet<&'static str> {
    ["read", "search", "list", "status"].into_iter().collect()
}

/// Returns true if the given tool kind is generally safe to auto-approve.
pub fn is_safe_kind(kind: &str) -> bool {
    safe_kinds().contains(kind.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_is_dangerous() {
        assert!(is_dangerous("bash"));
        assert!(is_dangerous("BASH")); // case insensitive
    }

    #[test]
    fn test_read_is_safe() {
        assert!(is_safe_kind("read"));
    }
}
