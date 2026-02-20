/// Context compaction — summarizes long conversation histories to avoid
/// hitting LLM context window limits.
///
/// Mirrors `src/agents/compaction.ts`.
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A message turn in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub role: String,
    pub content: String,
}

/// Result of compaction: a short summary plus an indicator of how many tokens
/// were saved.
#[derive(Debug)]
pub struct CompactionResult {
    pub summary: String,
    pub original_turns: usize,
    pub retained_turns: usize,
}

/// Compact a conversation by keeping the last `keep_recent` turns and
/// replacing the rest with a summary.
///
/// In a real implementation the summary is generated with an LLM call.
/// Here we produce a deterministic extractive summary (concatenate first
/// sentence of each turn).
pub fn compact_history(turns: Vec<Turn>, keep_recent: usize) -> CompactionResult {
    let original_turns = turns.len();
    if turns.len() <= keep_recent {
        return CompactionResult {
            summary: String::new(),
            original_turns,
            retained_turns: original_turns,
        };
    }

    let split_at = original_turns.saturating_sub(keep_recent);
    let (old, _recent) = turns.split_at(split_at);

    // Extractive summary: first sentence of each old turn.
    let summary_lines: Vec<String> = old
        .iter()
        .map(|t| {
            let first = t
                .content
                .split(['.', '!', '?', '\n'])
                .next()
                .unwrap_or(&t.content)
                .trim()
                .to_string();
            format!("[{}] {}", t.role, first)
        })
        .collect();

    CompactionResult {
        summary: format!(
            "[Compacted {} turns]\n{}",
            split_at,
            summary_lines.join("\n")
        ),
        original_turns,
        retained_turns: keep_recent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_reduces_turns() {
        let turns: Vec<Turn> = (0..20)
            .map(|i| Turn {
                role: if i % 2 == 0 { "user".into() } else { "assistant".into() },
                content: format!("Message number {}. Extra text.", i),
            })
            .collect();

        let result = compact_history(turns, 5);
        assert_eq!(result.retained_turns, 5);
        assert!(result.summary.contains("[Compacted 15 turns]"));
    }
}
