//! Memory sync pipeline: watches workspace files and re-indexes changed content.
//!
//! Mirrors OpenClaw's `manager-sync-ops.ts` (38 KB).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// A file that needs to be re-indexed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
}

/// Index metadata stored per file (for change detection without re-hashing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndexMeta {
    pub path: PathBuf,
    pub mtime_secs: u64,
    pub size_bytes: u64,
    pub chunk_ids: Vec<String>,
}

/// Sync state: tracks what's been indexed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncState {
    pub indexed: HashMap<PathBuf, FileIndexMeta>,
    pub last_sync: Option<u64>,
}

impl SyncState {
    /// Load sync state from disk (JSON).
    pub async fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    /// Save sync state to disk.
    pub async fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json.as_bytes()).await?;
        fs::rename(&tmp, path).await?;
        Ok(())
    }

    /// Record a file as indexed.
    pub fn mark_indexed(&mut self, meta: FileIndexMeta) {
        self.indexed.insert(meta.path.clone(), meta);
    }

    /// Remove a deleted file from the index.
    pub fn mark_deleted(&mut self, path: &Path) {
        self.indexed.remove(path);
    }

    /// Get current Unix timestamp in seconds.
    pub fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs()
    }
}

/// Supported file extensions for indexing.
pub const INDEXABLE_EXTENSIONS: &[&str] = &[
    "md", "txt", "rs", "py", "ts", "js", "go", "java", "c", "cpp",
    "h", "css", "html", "json", "yaml", "yml", "toml", "sh",
];

/// Scan a directory tree and detect changed/new/deleted files vs sync state.
pub async fn detect_changes(
    root: &Path,
    state: &SyncState,
    max_file_size: u64,
) -> Result<Vec<FileChange>> {
    let mut changes = Vec::new();
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();

    // Walk directory tree.
    scan_dir(root, &mut changes, &mut seen_paths, state, max_file_size).await?;

    // Find deleted files (in state but no longer on disk).
    for indexed_path in state.indexed.keys() {
        if !seen_paths.contains(indexed_path) {
            debug!(path = %indexed_path.display(), "Detected deleted file");
            changes.push(FileChange {
                path: indexed_path.clone(),
                kind: ChangeKind::Deleted,
            });
        }
    }

    Ok(changes)
}

fn is_indexable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| INDEXABLE_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

#[async_recursion::async_recursion]
async fn scan_dir(
    dir: &Path,
    changes: &mut Vec<FileChange>,
    seen: &mut HashSet<PathBuf>,
    state: &SyncState,
    max_file_size: u64,
) -> Result<()> {
    let mut entries = match fs::read_dir(dir).await {
        Ok(e) => e,
        Err(e) => {
            warn!("Cannot read dir {}: {}", dir.display(), e);
            return Ok(());
        }
    };

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden dirs and common ignore patterns.
        if file_name.starts_with('.') || file_name == "node_modules" || file_name == "target" {
            continue;
        }

        let meta = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.is_dir() {
            scan_dir(&path, changes, seen, state, max_file_size).await?;
        } else if meta.is_file() && is_indexable(&path) {
            let size = meta.len();
            if size > max_file_size {
                debug!(path = %path.display(), size, "Skipping large file");
                continue;
            }

            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            seen.insert(path.clone());

            let change_kind = if let Some(indexed) = state.indexed.get(&path) {
                if indexed.mtime_secs != mtime || indexed.size_bytes != size {
                    Some(ChangeKind::Modified)
                } else {
                    None // Unchanged.
                }
            } else {
                Some(ChangeKind::Created)
            };

            if let Some(kind) = change_kind {
                debug!(path = %path.display(), ?kind, "Detected file change");
                changes.push(FileChange { path, kind });
            }
        }
    }
    Ok(())
}

/// Text chunker: splits content into overlapping chunks for embedding.
pub fn chunk_text(content: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if content.is_empty() {
        return vec![];
    }
    // Split by paragraph first, then by char limit.
    let paragraphs: Vec<&str> = content.split("\n\n").collect();
    let mut chunks = Vec::new();
    let mut current = String::new();

    for para in &paragraphs {
        if current.len() + para.len() > chunk_size && !current.is_empty() {
            chunks.push(current.trim().to_string());
            // Overlap: keep last `overlap` chars.
            current = if current.len() > overlap {
                current[current.len() - overlap..].to_string()
            } else {
                current
            };
        }
        current.push_str(para);
        current.push_str("\n\n");
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_text_by_paragraph() {
        let content = "Para one.\n\nPara two.\n\nPara three.";
        let chunks = chunk_text(content, 50, 10);
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|c| !c.is_empty()));
    }

    #[test]
    fn chunks_empty_content() {
        let chunks = chunk_text("", 50, 10);
        assert!(chunks.is_empty());
    }
}
