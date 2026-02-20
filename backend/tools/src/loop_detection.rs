/// Tool loop detection — prevents agents from calling the same tool
/// repeatedly in an infinite loop.
///
/// Mirrors `src/agents/tool-loop-detection.ts`.
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A single tool invocation record.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub input_hash: u64,
}

/// Detection state for a single agent run.
#[derive(Default)]
pub struct LoopDetector {
    /// (tool_name, input_hash) → call count
    call_counts: HashMap<(String, u64), usize>,
    /// Maximum allowed identical calls before a loop is declared.
    max_identical: usize,
}

impl LoopDetector {
    pub fn new(max_identical: usize) -> Self {
        Self {
            call_counts: HashMap::new(),
            max_identical,
        }
    }

    /// Record a tool call and return whether a loop was detected.
    pub fn record(&mut self, call: &ToolCall) -> bool {
        let key = (call.tool_name.clone(), call.input_hash);
        let count = self.call_counts.entry(key).or_insert(0);
        *count += 1;
        *count > self.max_identical
    }

    /// Reset the detector for a new run.
    pub fn reset(&mut self) {
        self.call_counts.clear();
    }
}

/// Compute a basic hash of the tool input string.
pub fn hash_input(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    input.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_detected_after_threshold() {
        let mut detector = LoopDetector::new(2);
        let call = ToolCall {
            tool_name: "bash".into(),
            input_hash: hash_input("ls -la"),
        };
        assert!(!detector.record(&call)); // 1st — ok
        assert!(!detector.record(&call)); // 2nd — ok
        assert!(detector.record(&call));  // 3rd — loop!
    }

    #[test]
    fn test_different_inputs_not_flagged() {
        let mut detector = LoopDetector::new(2);
        for i in 0..5 {
            let call = ToolCall {
                tool_name: "bash".into(),
                input_hash: hash_input(&format!("ls -{}", i)),
            };
            assert!(!detector.record(&call));
        }
    }
}
