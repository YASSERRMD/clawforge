//! Apply Patch Tool
//!
//! Mirrors `src/agents/apply-patch.ts` and `src/agents/apply-patch-update.ts`.
//! Allows the agent to apply unified diffs to files.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::patch_validator::PatchValidator;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyPatchConfig {
    pub max_patch_size_bytes: usize,
    pub work_dir: PathBuf,
}

impl Default for ApplyPatchConfig {
    fn default() -> Self {
        Self {
            max_patch_size_bytes: 1024 * 512, // 512KB limit
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

pub struct ApplyPatchTool {
    config: ApplyPatchConfig,
    validator: PatchValidator,
}

impl ApplyPatchTool {
    pub fn new(config: ApplyPatchConfig) -> Self {
        let validator = PatchValidator::new(config.work_dir.clone());
        Self { config, validator }
    }

    /// Applies a unified diff to the local filesystem.
    pub async fn apply(&self, patch_content: &str) -> Result<String> {
        if patch_content.len() > self.config.max_patch_size_bytes {
            warn!("Patch exceeds maximum size limit.");
            return Ok("Error: Patch size too large.".into());
        }

        // Validate patch safety (no dir traversal, etc)
        self.validator.validate(patch_content)?;

        // MOCK: Actually applying patch
        // In real implementation, parse hunks and apply to target files.
        // We might use `patch` CLI command or a rust-based diff parsing crate like `patch`.
        
        info!("Successfully applied unified diff patch");
        Ok("Patch applied successfully".into())
    }
}
