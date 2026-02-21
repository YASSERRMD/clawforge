//! Patch Validator
//!
//! Validates unified diff patches to ensure they don't modify files outside the
//! intended workspace sandbox (i.e. directory traversal protection).

use anyhow::{bail, Result};
use std::path::{Component, Path, PathBuf};
use tracing::debug;

pub struct PatchValidator {
    work_dir: PathBuf,
}

impl PatchValidator {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// Validates a single file path for safety
    pub fn is_safe_path(&self, target_path: &str) -> bool {
        let path = Path::new(target_path);
        
        // Block absolute paths
        if path.is_absolute() {
            return false;
        }

        // Block directory traversal (..)
        for component in path.components() {
            if matches!(component, Component::ParentDir) {
                return false;
            }
        }

        true
    }

    /// Scans a unified diff string to ensure all referenced files are safe.
    pub fn validate(&self, patch_content: &str) -> Result<()> {
        for line in patch_content.lines() {
            if line.starts_with("--- a/") || line.starts_with("+++ b/") {
                let file_path = line[6..].trim();
                if !self.is_safe_path(file_path) {
                    bail!("Unsafe path detected in patch: {}", file_path);
                }
                debug!("Validated patch target path: {}", file_path);
            }
        }

        Ok(())
    }
}
