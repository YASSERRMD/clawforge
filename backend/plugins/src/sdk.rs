//! Plugin SDK
//!
//! Provides the core data structures and boundary interfaces exposed to plugin developers.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub session_id: String,
    pub channel_id: String,
    pub agent_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookResult {
    Continue,
    Abort(String),
    ModifyContext(PluginContext),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub payload: String,
    pub content_type: String,
}

pub trait ClawForgePlugin {
    fn on_load(&self) -> Result<(), String>;
    fn on_message(&self, ctx: &PluginContext, message: &str) -> HookResult;
    fn execute_tool(&self, ctx: &PluginContext, tool_name: &str, args: &str) -> ToolResult;
}
