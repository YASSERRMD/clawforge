/// QMD memory backend — wraps the `qmd` CLI subprocess for local vector search.
///
/// Mirrors `src/memory/qmd-manager.ts` from OpenClaw.
/// QMD is a local semantic search tool that indexes markdown/text files.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct QmdConfig {
    /// Path to the `qmd` binary.
    pub bin: String,
    /// Directories to add as collections.
    pub collections: Vec<QmdCollection>,
    /// Interval between automatic index updates (0 = disabled).
    pub update_interval_secs: u64,
    /// Maximum search results.
    pub max_results: usize,
    /// Search timeout.
    pub timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct QmdCollection {
    pub name: String,
    pub path: String,
    pub pattern: String,
}

impl Default for QmdConfig {
    fn default() -> Self {
        Self {
            bin: "qmd".to_string(),
            collections: vec![],
            update_interval_secs: 300,
            max_results: 10,
            timeout_secs: 30,
        }
    }
}

// ---------------------------------------------------------------------------
// Search result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmdSearchResult {
    pub path: String,
    pub score: f64,
    pub snippet: String,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct QmdMemoryManager {
    config: QmdConfig,
    agent_id: String,
    cache_dir: PathBuf,
}

impl QmdMemoryManager {
    pub fn new(agent_id: impl Into<String>, config: QmdConfig) -> Self {
        let agent_id = agent_id.into();
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let cache_dir = PathBuf::from(&home)
            .join(".cache")
            .join("clawforge")
            .join("agents")
            .join(&agent_id)
            .join("qmd");
        Self { config, agent_id, cache_dir }
    }

    async fn run_qmd(&self, args: &[&str]) -> Result<String> {
        let xdg_config = self.cache_dir.join("xdg-config");
        let xdg_cache = self.cache_dir.join("xdg-cache");
        let xdg_config_str = xdg_config.to_string_lossy().to_string();
        let xdg_cache_str = xdg_cache.to_string_lossy().to_string();

        let bin = self.config.bin.clone();
        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let timeout_secs = self.config.timeout_secs;

        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(timeout_secs),
            async move {
                let args_ref: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
                tokio::process::Command::new(&bin)
                    .args(&args_ref)
                    .env("XDG_CONFIG_HOME", &xdg_config_str)
                    .env("XDG_CACHE_HOME", &xdg_cache_str)
                    .env("NO_COLOR", "1")
                    .output()
                    .await
            },
        )
        .await;

        match result {
            Err(_) => bail!("qmd command timed out after {}s", self.config.timeout_secs),
            Ok(Err(e)) => bail!("qmd spawn error: {}", e),
            Ok(Ok(out)) => {
                if out.status.success() {
                    Ok(String::from_utf8_lossy(&out.stdout).to_string())
                } else {
                    bail!("qmd error: {}", String::from_utf8_lossy(&out.stderr))
                }
            }
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.cache_dir).await?;
        // Add all configured collections
        for col in &self.config.collections {
            let _ = self.run_qmd(&[
                "collection", "add", &col.path,
                "--name", &col.name,
                "--mask", &col.pattern,
            ]).await;
        }
        info!("[QMD] Initialized for agent {}", self.agent_id);
        Ok(())
    }

    pub async fn update(&self) -> Result<()> {
        info!("[QMD] Updating index for agent {}", self.agent_id);
        self.run_qmd(&["update"]).await?;
        self.run_qmd(&["embed"]).await.unwrap_or_else(|e| {
            warn!("[QMD] embed failed (non-fatal): {}", e);
            String::new()
        });
        Ok(())
    }

    pub async fn search(&self, query: &str) -> Result<Vec<QmdSearchResult>> {
        if query.trim().is_empty() { return Ok(vec![]); }
        let limit = self.config.max_results.to_string();
        let raw = self.run_qmd(&["query", query, "--limit", &limit, "--json"]).await?;
        let results = parse_qmd_json(&raw);
        info!("[QMD] Search '{}' → {} results", &query[..query.len().min(40)], results.len());
        Ok(results)
    }
}

fn parse_qmd_json(raw: &str) -> Vec<QmdSearchResult> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(raw) else {
        return vec![];
    };
    let Some(arr) = json.as_array() else { return vec![]; };
    arr.iter().filter_map(|entry| {
        let path = entry.get("docid").or_else(|| entry.get("path"))
            .and_then(|v| v.as_str())?.to_string();
        let score = entry.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let snippet = entry.get("snippet").and_then(|v| v.as_str()).unwrap_or("").to_string();
        Some(QmdSearchResult { path, score, snippet, start_line: None, end_line: None })
    }).collect()
}
