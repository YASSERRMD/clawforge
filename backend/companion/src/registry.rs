/// Companion bot registry — selects the right companion for a run.
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::traits::{CompanionBot, Persona};
use crate::clawdbot::Clawdbot;
use crate::moltbot::Moltbot;

/// Registry of all available companion bots.
pub struct CompanionRegistry {
    bots: HashMap<String, Box<dyn CompanionBot>>,
}

impl CompanionRegistry {
    pub fn new() -> Self {
        let mut bots: HashMap<String, Box<dyn CompanionBot>> = HashMap::new();
        bots.insert("clawdbot".into(), Box::new(Clawdbot::new()));
        bots.insert("moltbot".into(), Box::new(Moltbot::new()));
        Self { bots }
    }

    /// Get a companion by ID, returning `None` if not found.
    pub fn get(&self, id: &str) -> Option<&dyn CompanionBot> {
        self.bots.get(id).map(|b| b.as_ref())
    }

    /// List all available companion IDs.
    pub fn ids(&self) -> Vec<&str> {
        self.bots.keys().map(|s| s.as_str()).collect()
    }

    /// Get the system prompt for a companion, falling back to a generic prompt.
    pub fn system_prompt_for(&self, id: &str) -> String {
        self.get(id)
            .map(|b| b.system_prompt().to_string())
            .unwrap_or_else(|| {
                "You are a helpful AI assistant.".to_string()
            })
    }
}

impl Default for CompanionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
