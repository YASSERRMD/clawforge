//! Dynamic system prompt builder.
//!
//! Mirrors `src/agents/agent-prompt.ts` and `src/agents/assistant-identity.ts`.

use crate::assistant_identity::AssistantIdentity;
use crate::chat::ChatMessage;
use crate::prompt_cache::PromptCache;
use crate::session_state::SessionState;
use std::sync::Arc;

pub struct PromptBuilder {
    cache: Arc<PromptCache>,
}

impl PromptBuilder {
    pub fn new(cache: Arc<PromptCache>) -> Self {
        Self { cache }
    }

    /// Builds the monolithic system prompt that configures the agent's behavior.
    pub fn build(&self, session: &SessionState, identity: &AssistantIdentity) -> ChatMessage {
        let cache_key = format!("{}:{}", session.session_id, session.agent_id);

        if let Some(cached) = self.cache.get(&cache_key) {
            return cached;
        }

        // Collect core identity
        let persona = identity.compile();

        // Collect memory snippets (mocked here)
        let memory = "No additional memory.";

        // Collect available tools description
        let tools = "Tools available: []";

        // Assemble into a single system message
        let content = format!(
            "{}\n\n{}\n\nRULES:\n1. Be helpful.\n2. Do NOT use fake tool calls.\n\n{}",
            persona, memory, tools
        );

        let msg = ChatMessage::system(content);
        self.cache.insert(cache_key, msg.clone());
        msg
    }
}
