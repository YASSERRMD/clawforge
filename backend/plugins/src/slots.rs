/// Plugin tool slots — inject plugin tools into the agent tool registry.
///
/// Mirrors `src/plugins/slots.ts` from OpenClaw.
use serde::{Deserialize, Serialize};

use crate::manifest::PluginToolSlot;

/// A resolved tool entry from a plugin, ready for injection into the tool registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPluginTool {
    pub plugin_id: String,
    pub tool_name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Extract all enabled tool slots from a set of plugin manifests.
pub fn collect_plugin_tools(
    plugins: impl Iterator<Item = (String, Vec<PluginToolSlot>)>,
) -> Vec<ResolvedPluginTool> {
    plugins
        .flat_map(|(plugin_id, slots)| {
            slots.into_iter().map(move |slot| ResolvedPluginTool {
                plugin_id: plugin_id.clone(),
                tool_name: slot.name,
                description: slot.description,
                input_schema: slot.input_schema,
            })
        })
        .collect()
}
