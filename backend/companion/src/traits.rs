/// Companion bot personality trait.
///
/// Each companion bot has a unique persona, system prompt, and set of
/// behavioral rules. The trait provides the interface for integrating
/// a companion into the agent pipeline.
use serde::{Deserialize, Serialize};

/// The persona definition for a companion bot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    /// Short identifier (e.g. "clawdbot", "moltbot")
    pub id: String,
    /// Display name shown to users
    pub display_name: String,
    /// System prompt defining the bot's character and rules
    pub system_prompt: String,
    /// Optional emoji/avatar identifier
    pub avatar: Option<String>,
    /// Tone descriptor for logging/debugging
    pub tone: String,
}

/// Returns the full system prompt to prepend to any agent run using this companion.
pub trait CompanionBot: Send + Sync {
    fn persona(&self) -> &Persona;
    fn system_prompt(&self) -> &str {
        &self.persona().system_prompt
    }
    fn name(&self) -> &str {
        &self.persona().id
    }
}
