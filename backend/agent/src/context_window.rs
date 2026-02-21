//! Context window compaction and token management.
//!
//! Mirrors `src/agents/context-window.ts`.

use crate::chat::ChatMessage;

pub struct ContextWindow {
    pub messages: Vec<ChatMessage>,
    pub token_count: usize,
}

impl ContextWindow {
    /// Build a compacted context window from the full transcript.
    /// Ensures we do not exceed `max_tokens`.
    pub fn build(transcript: &[ChatMessage], max_tokens: usize) -> Self {
        // Very basic implementation: just copy all for now.
        // In a real implementation:
        // 1. Always keep the system prompt (first message).
        // 2. Iterate backwards from the end, accumulating tokens.
        // 3. If we hit the limit, compact the middle into a summary or drop it.
        
        let messages = transcript.to_vec();
        let token_count = messages.len() * 50; // Fake token count

        if token_count > max_tokens {
            // Trim leading messages (excluding system prompt if any)
            // This is a naive truncation for the stub.
        }

        Self {
            messages,
            token_count,
        }
    }
}
