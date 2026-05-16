//! Hot configuration reload.
//!
//! Watches a YAML config file for modifications and applies changes to the
//! shared `GatewayConfig` without restarting the process.

use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

/// Runtime-adjustable gateway settings loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Maximum WebSocket connections allowed simultaneously.
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// Rate-limit window in seconds.
    #[serde(default = "default_rate_window_secs")]
    pub rate_window_secs: u64,
    /// Maximum requests per rate-limit window.
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u32,
}

fn default_max_connections() -> usize { 1000 }
fn default_rate_window_secs() -> u64 { 60 }
fn default_rate_limit() -> u32 { 100 }

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            rate_window_secs: default_rate_window_secs(),
            rate_limit: default_rate_limit(),
        }
    }
}

pub struct ConfigReloader {
    config: Arc<RwLock<GatewayConfig>>,
}

impl ConfigReloader {
    pub fn new(config: Arc<RwLock<GatewayConfig>>) -> Self {
        Self { config }
    }

    /// Watch the specified YAML configuration file for changes and reload on modify.
    pub async fn watch<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);

        let mut watcher = notify::recommended_watcher(move |res| {
            if let Err(e) = tx.blocking_send(res) {
                error!("Failed to send file event: {:?}", e);
            }
        })?;

        watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;
        info!("Watching configuration file for changes: {:?}", path.as_ref());

        let config = Arc::clone(&self.config);
        let path_owned = path.as_ref().to_path_buf();

        tokio::spawn(async move {
            // keep watcher alive for the lifetime of the task
            let _w = watcher;
            while let Some(res) = rx.recv().await {
                match res {
                    Ok(event) if event.kind.is_modify() => {
                        info!("Config file modified — reloading {:?}", path_owned);
                        match std::fs::read_to_string(&path_owned) {
                            Err(e) => warn!("Could not read config file: {}", e),
                            Ok(contents) => match serde_yaml::from_str::<GatewayConfig>(&contents) {
                                Err(e) => warn!("Config parse error — keeping old config: {}", e),
                                Ok(new_cfg) => {
                                    *config.write().await = new_cfg;
                                    info!("Gateway config reloaded successfully");
                                }
                            },
                        }
                    }
                    Ok(_) => {}
                    Err(e) => warn!("Watch error: {:?}", e),
                }
            }
        });

        Ok(())
    }
}
