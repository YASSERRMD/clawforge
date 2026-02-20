/// Hook trait and registry.
///
/// Hooks are async functions that observe or transform pipeline data.
/// Multiple hooks can be registered per phase; they run sequentially in
/// registration order. The first hook to return `abort: true` halts the chain.
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::types::{HookPayload, HookPhase, HookResult};

// ---------------------------------------------------------------------------
// Hook trait
// ---------------------------------------------------------------------------

/// A hook that runs at a specific lifecycle phase.
#[async_trait]
pub trait Hook: Send + Sync {
    /// Human-readable name for logging.
    fn name(&self) -> &str;

    /// Run the hook. Return `HookResult::pass()` to continue normally.
    async fn run(&self, payload: &HookPayload) -> Result<HookResult>;
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

type HookBox = Arc<dyn Hook>;
type PhaseMap = HashMap<HookPhase, Vec<HookBox>>;

/// Thread-safe registry of hooks organized by phase.
#[derive(Default, Clone)]
pub struct HookRegistry {
    hooks: Arc<RwLock<PhaseMap>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a hook to run at a specific phase.
    pub async fn register(&self, phase: HookPhase, hook: Arc<dyn Hook>) {
        let mut map = self.hooks.write().await;
        map.entry(phase).or_default().push(hook);
    }

    /// Run all hooks registered for the phase in the given payload.
    /// Returns the merged `HookResult` after running the chain.
    pub async fn run(&self, payload: &HookPayload) -> HookResult {
        let phase = payload.phase();
        let map = self.hooks.read().await;
        let Some(chain) = map.get(&phase) else {
            return HookResult::pass();
        };

        let mut merged = HookResult::pass();
        for hook in chain.iter() {
            debug!("[Hooks] Running {} for phase {:?}", hook.name(), phase);
            match hook.run(payload).await {
                Ok(result) => {
                    // Propagate content transform
                    if let Some(content) = &result.modified_content {
                        merged.modified_content = Some(content.clone());
                    }
                    // Propagate model override
                    if let Some(model) = &result.model_override {
                        merged.model_override = Some(model.clone());
                    }
                    // Abort chain if requested
                    if result.abort {
                        merged.abort = true;
                        merged.reason = result.reason;
                        return merged;
                    }
                }
                Err(e) => {
                    warn!("[Hooks] {} returned error: {}", hook.name(), e);
                    // Errors in hooks are non-fatal by default
                }
            }
        }
        merged
    }
}
