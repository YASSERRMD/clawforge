//! Hot configuration relolad.
//!
//! Mirrors `src/gateway/config-reload.ts`.

use anyhow::Result;
use notify::{Watcher, RecursiveMode, RecommendedWatcher};
use std::path::Path;
use tokio::sync::mpsc;
use tracing::{info, warn, error};

pub struct ConfigReloader {
    // In a real implementation this holds an Arc to the parsed config that it updates.
}

impl ConfigReloader {
    pub fn new() -> Self {
        Self {}
    }

    /// Watch the specified configuration file for changes and reload.
    pub async fn watch<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);

        let mut watcher = notify::recommended_watcher(move |res| {
            if let Err(e) = tx.blocking_send(res) {
                error!("Failed to send file event: {:?}", e);
            }
        })?;

        watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;
        info!("Watching configuration file for changes: {:?}", path.as_ref());

        // Spawn a task to handle the events
        tokio::spawn(async move {
            // keep watcher alive
            let _w = watcher;
            while let Some(res) = rx.recv().await {
                match res {
                    Ok(event) => {
                        if event.kind.is_modify() {
                            info!("Config file modified. Triggering soft reload...");
                            // TODO: Read file, parse config, update shared state
                        }
                    }
                    Err(e) => warn!("Watch error: {:?}", e),
                }
            }
        });

        Ok(())
    }
}
