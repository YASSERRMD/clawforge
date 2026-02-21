//! Plugin lifecycle hooks: before_load, after_load, before_unload, after_unload.
//!
//! Mirrors `src/plugins/lifecycle.ts`.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Current state of a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginState {
    Unloaded,
    Loading,
    Active,
    Unloading,
    Failed,
}

/// Context passed to lifecycle hooks.
#[derive(Debug, Clone)]
pub struct PluginLifecycleContext {
    pub plugin_id: String,
    pub plugin_version: String,
    pub config: serde_json::Value,
}

/// Lifecycle event trait implemented by plugins.
#[async_trait]
pub trait PluginLifecycle: Send + Sync {
    /// Called before the plugin is loaded. Can cancel the load by returning Err.
    async fn before_load(&self, ctx: &PluginLifecycleContext) -> Result<()>;
    /// Called after the plugin is successfully loaded.
    async fn after_load(&self, ctx: &PluginLifecycleContext) -> Result<()>;
    /// Called before the plugin is unloaded. Should do cleanup.
    async fn before_unload(&self, ctx: &PluginLifecycleContext) -> Result<()>;
    /// Called after the plugin is unloaded.
    async fn after_unload(&self, ctx: &PluginLifecycleContext);
}

/// Default no-op lifecycle implementation.
pub struct DefaultPluginLifecycle;

#[async_trait]
impl PluginLifecycle for DefaultPluginLifecycle {
    async fn before_load(&self, ctx: &PluginLifecycleContext) -> Result<()> {
        debug!(plugin = %ctx.plugin_id, "before_load");
        Ok(())
    }

    async fn after_load(&self, ctx: &PluginLifecycleContext) -> Result<()> {
        info!(plugin = %ctx.plugin_id, version = %ctx.plugin_version, "Plugin loaded");
        Ok(())
    }

    async fn before_unload(&self, ctx: &PluginLifecycleContext) -> Result<()> {
        debug!(plugin = %ctx.plugin_id, "before_unload");
        Ok(())
    }

    async fn after_unload(&self, ctx: &PluginLifecycleContext) {
        info!(plugin = %ctx.plugin_id, "Plugin unloaded");
    }
}

/// Run the full load sequence for a plugin.
pub async fn run_load_sequence(
    lifecycle: &dyn PluginLifecycle,
    ctx: &PluginLifecycleContext,
) -> Result<PluginState> {
    debug!(plugin = %ctx.plugin_id, "Running load sequence");
    lifecycle.before_load(ctx).await?;
    if let Err(e) = lifecycle.after_load(ctx).await {
        warn!(plugin = %ctx.plugin_id, error = %e, "after_load failed");
        return Ok(PluginState::Failed);
    }
    Ok(PluginState::Active)
}

/// Run the full unload sequence for a plugin.
pub async fn run_unload_sequence(
    lifecycle: &dyn PluginLifecycle,
    ctx: &PluginLifecycleContext,
) -> PluginState {
    debug!(plugin = %ctx.plugin_id, "Running unload sequence");
    if let Err(e) = lifecycle.before_unload(ctx).await {
        warn!(plugin = %ctx.plugin_id, error = %e, "before_unload failed");
    }
    lifecycle.after_unload(ctx).await;
    PluginState::Unloaded
}
