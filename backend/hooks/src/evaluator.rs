//! Hook evaluator: evaluates conditions for hook triggering.
//!
//! Determines whether a hook's trigger conditions match the current context.
//! Mirrors `src/hooks/evaluator.ts`.

use crate::types::{HookCondition, HookContext, HookTrigger};
use serde_json::Value;
use tracing::debug;

/// Evaluate whether a hook should fire given the current context.
pub fn should_fire(trigger: &HookTrigger, ctx: &HookContext) -> bool {
    match trigger {
        HookTrigger::Always => true,

        HookTrigger::OnEvent { event_name } => {
            ctx.event_name.as_deref() == Some(event_name.as_str())
        }

        HookTrigger::OnCondition { condition } => evaluate_condition(condition, ctx),

        HookTrigger::OnPattern { pattern } => {
            ctx.message_text
                .as_deref()
                .map(|text| pattern_matches(pattern, text))
                .unwrap_or(false)
        }

        HookTrigger::OnSchedule { .. } => {
            // Schedule hooks are driven by the cron engine, not evaluated here.
            false
        }
    }
}

/// Evaluate a structured condition against the hook context.
fn evaluate_condition(cond: &HookCondition, ctx: &HookContext) -> bool {
    match cond {
        HookCondition::FieldEquals { field, value } => {
            extract_field(ctx, field)
                .map(|v| &v == value)
                .unwrap_or(false)
        }

        HookCondition::FieldContains { field, substring } => {
            extract_field(ctx, field)
                .and_then(|v| v.as_str().map(|s| s.contains(substring.as_str())))
                .unwrap_or(false)
        }

        HookCondition::FieldMatches { field, regex } => {
            extract_field(ctx, field)
                .and_then(|v| v.as_str().map(String::from))
                .map(|text| {
                    regex::Regex::new(regex)
                        .map(|re| re.is_match(&text))
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        }

        HookCondition::And { conditions } => {
            conditions.iter().all(|c| evaluate_condition(c, ctx))
        }

        HookCondition::Or { conditions } => {
            conditions.iter().any(|c| evaluate_condition(c, ctx))
        }

        HookCondition::Not { condition } => !evaluate_condition(condition, ctx),
    }
}

/// Extract a field from the hook context by dotted path (e.g., "message.text").
fn extract_field(ctx: &HookContext, field: &str) -> Option<Value> {
    let json = serde_json::to_value(ctx).ok()?;
    let mut current = &json;
    for part in field.split('.') {
        current = current.get(part)?;
    }
    Some(current.clone())
}

/// Simple glob-like pattern matching (supports `*` wildcard).
fn pattern_matches(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Ok(re) = regex::Regex::new(&glob_to_regex(pattern)) {
        return re.is_match(text);
    }
    text.contains(pattern)
}

fn glob_to_regex(pattern: &str) -> String {
    let mut regex = String::from("(?i)^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }
    regex.push('$');
    regex
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(event: &str) -> HookContext {
        HookContext {
            event_name: Some(event.to_string()),
            message_text: None,
            extra: Default::default(),
        }
    }

    #[test]
    fn always_fires() {
        assert!(should_fire(&HookTrigger::Always, &ctx("anything")));
    }

    #[test]
    fn on_event_matches() {
        let trigger = HookTrigger::OnEvent { event_name: "message.received".into() };
        assert!(should_fire(&trigger, &ctx("message.received")));
        assert!(!should_fire(&trigger, &ctx("message.sent")));
    }

    #[test]
    fn pattern_wildcard() {
        assert!(pattern_matches("hello *", "hello world"));
        assert!(!pattern_matches("hello *", "goodbye world"));
    }
}
