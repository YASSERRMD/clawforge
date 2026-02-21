//! Agent identity management.
//!
//! Mirrors `src/agents/assistant-identity.ts`.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantIdentity {
    pub name: String,
    pub primary_persona: String,
    pub avatar_url: Option<String>,
    pub style_overrides: HashMap<String, String>,
}

impl Default for AssistantIdentity {
    fn default() -> Self {
        Self {
            name: "ClawForge Assistant".into(),
            primary_persona: "You are a helpful, concise AI assistant.".into(),
            avatar_url: None,
            style_overrides: HashMap::new(),
        }
    }
}

impl AssistantIdentity {
    pub fn new(name: impl Into<String>, persona: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            primary_persona: persona.into(),
            ..Default::default()
        }
    }

    /// Compile the identity into a system prompt paragraph.
    pub fn compile(&self) -> String {
        let mut out = format!("IDENTITY: {}\n{}", self.name, self.primary_persona);
        
        if !self.style_overrides.is_empty() {
            out.push_str("\n\nSTYLE GUIDELINES:\n");
            for (k, v) in &self.style_overrides {
                out.push_str(&format!("- {}: {}\n", k, v));
            }
        }
        
        out
    }
}
