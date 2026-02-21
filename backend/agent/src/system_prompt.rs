//! Dynamic system prompt builder.
//!
//! Mirrors `src/agents/agent-prompt.ts` and `src/agents/assistant-identity.ts`.

use crate::session_state::SessionState;
use crate::chat::ChatMessage;

pub struct PromptBuilder;

impl PromptBuilder {
    /// Builds the monolithic system prompt that configures the agent's behavior.
    pub fn build(session: &SessionState) -> ChatMessage {
        // Collect core identity
        let persona = Self::build_identity(&session.agent_id);
        
        // Collect memory snippets (mocked here)
        let memory = "No additional memory.";

        // Collect available tools description
        let tools = "Tools available: []";

        // Assemble into a single system message
        let content = format!(
            "{}\n\n{}\n\nRULES:\n1. Be helpful.\n2. Do NOT use fake tool calls.\n\n{}",
            persona, memory, tools
        );

        ChatMessage::system(content)
    }

    fn build_identity(agent_id: &str) -> String {
        format!(
            "You are {}, an advanced AI agent powered by ClawForge. \
            Your goal is to assist the user proactively and securely.",
            agent_id
        )
    }
}
