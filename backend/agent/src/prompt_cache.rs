//! System prompt caching.
//!
//! Mirrors `src/agents/prompt-cache.ts` in intent.

use moka::sync::Cache;
use std::time::Duration;
use crate::chat::ChatMessage;

pub struct PromptCache {
    /// Caches the system prompt Message by session_id/agent_id hash
    cache: Cache<String, ChatMessage>,
}

impl PromptCache {
    pub fn new() -> Self {
        Self {
            // Cache system prompts for 10 minutes to avoid rebuilding mostly static strings
            cache: Cache::builder()
                .time_to_idle(Duration::from_secs(600))
                .build(),
        }
    }

    pub fn get(&self, key: &str) -> Option<ChatMessage> {
        self.cache.get(key)
    }

    pub fn insert(&self, key: String, prompt: ChatMessage) {
        self.cache.insert(key, prompt);
    }
    
    pub fn invalidate(&self, key: &str) {
        self.cache.invalidate(key);
    }
}
