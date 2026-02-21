//! Session state management for the running agent.
//!
//! Mirrors `src/agents/runtime.ts` state holding aspect.

use std::collections::HashMap;
use crate::chat::ChatMessage;

/// Configuration for the model being used in the session.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub model_name: String,
    pub max_context_tokens: usize,
    pub temperature: f32,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_name: "gpt-4o".to_string(),
            max_context_tokens: 128000,
            temperature: 0.7,
        }
    }
}

/// Active state of a conversation session.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: String,
    pub agent_id: String,
    /// Full, un-compacted conversation transcript.
    pub transcript: Vec<ChatMessage>,
    pub model_config: ModelConfig,
    /// Variables and context scoped to this session.
    pub context_vars: HashMap<String, String>,
}

impl SessionState {
    pub fn new(session_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            agent_id: agent_id.into(),
            transcript: Vec::new(),
            model_config: ModelConfig::default(),
            context_vars: HashMap::new(),
        }
    }
}
