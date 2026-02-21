//! Discord Embed Builder
//!
//! Provides utilities to map standard Markdown AST or tool output reports into
//! rich, colorful Discord Embed blocks.

use anyhow::Result;

pub struct DiscordEmbeds;

impl DiscordEmbeds {
    /// Translates an agent's final answer string into a rich purple embed format.
    pub fn build_agent_response(content: &str) -> String {
        // MOCK: Generate JSON matching Discord Embed Schema
        let embed = format!(
            r#"{{ "embeds": [{{ "description": "{}", "color": 10181046, "footer": {{ "text": "ClawForge Backend" }} }}] }}"#, 
            content.replace('"', "\\\"")
        );
        embed
    }

    /// Wraps error strings into a standard red contextual alert embed.
    pub fn build_error_card(error: &str) -> String {
        format!(
            r#"{{ "embeds": [{{ "title": "Runtime Event Error", "description": "{}", "color": 15548997 }}] }}"#, 
            error.replace('"', "\\\"")
        )
    }
}
