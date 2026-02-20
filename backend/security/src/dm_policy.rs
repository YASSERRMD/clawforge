/// DM policy — controls which users/addresses are allowed to invoke the agent
/// via direct message across channels.
///
/// Mirrors `src/security/dm-policy-shared.ts`.
use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tracing::warn;

/// Policy for who may DM the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmPolicy {
    /// Explicit allowlist. If `allow_all` is false, only these identifiers pass.
    pub allowlist: HashSet<String>,
    /// If true, all inbound DMs are accepted (no allowlist enforcement).
    pub allow_all: bool,
    /// If true, block all DMs entirely.
    pub block_all: bool,
}

impl Default for DmPolicy {
    fn default() -> Self {
        Self {
            allowlist: HashSet::new(),
            allow_all: true, // open by default (same as OpenClaw)
            block_all: false,
        }
    }
}

impl DmPolicy {
    /// Returns `true` if the given sender is permitted to invoke the agent.
    pub fn is_allowed(&self, sender: &str) -> bool {
        if self.block_all {
            warn!("[DmPolicy] Blocked DM from {}", sender);
            return false;
        }
        if self.allow_all {
            return true;
        }
        let allowed = self.allowlist.contains(sender);
        if !allowed {
            warn!("[DmPolicy] DM from {} not in allowlist", sender);
        }
        allowed
    }

    /// Add an identifier to the allowlist.
    pub fn allow(&mut self, sender: impl Into<String>) {
        self.allowlist.insert(sender.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_all_by_default() {
        let policy = DmPolicy::default();
        assert!(policy.is_allowed("anyone@example.com"));
    }

    #[test]
    fn test_allowlist_enforcement() {
        let mut policy = DmPolicy { allow_all: false, ..Default::default() };
        policy.allow("trusted@example.com");
        assert!(policy.is_allowed("trusted@example.com"));
        assert!(!policy.is_allowed("stranger@evil.com"));
    }

    #[test]
    fn test_block_all() {
        let policy = DmPolicy { block_all: true, ..Default::default() };
        assert!(!policy.is_allowed("anyone"));
    }
}
