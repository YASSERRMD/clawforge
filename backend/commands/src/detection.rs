/// Slash command detection — identify /commands in inbound messages.
///
/// Mirrors `src/auto-reply/command-detection.ts` from OpenClaw.
use crate::types::CommandInvocation;
use crate::registry::CommandRegistry;

/// Detect a slash command in the start of a message string.
/// Returns `Some(CommandInvocation)` if the message starts with a known alias.
/// Returns `None` if it's a normal message.
pub fn detect_command(text: &str, registry: &CommandRegistry) -> Option<CommandInvocation> {
    let trimmed = text.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    // Find the first whitespace to split alias from args
    let (alias_part, rest) = trimmed.split_once(|c: char| c.is_whitespace())
        .map(|(a, r)| (a, r.trim()))
        .unwrap_or((trimmed, ""));

    let def = registry.find_by_alias(alias_part)?;

    // Parse positional args (space-separated, respecting capture_remaining)
    let args = parse_args(rest, &def.args);

    Some(CommandInvocation {
        key: def.key.clone(),
        raw_alias: alias_part.to_string(),
        args,
        raw_args: rest.to_string(),
    })
}

fn parse_args(text: &str, arg_defs: &[crate::types::CommandArg]) -> Vec<String> {
    if text.is_empty() || arg_defs.is_empty() {
        return vec![];
    }

    let mut result = Vec::new();
    let mut remaining = text.trim();

    for (i, def) in arg_defs.iter().enumerate() {
        if remaining.is_empty() { break; }
        if def.capture_remaining || i == arg_defs.len() - 1 {
            result.push(remaining.to_string());
            break;
        }
        // Take one token
        let (token, rest) = remaining.split_once(|c: char| c.is_whitespace())
            .map(|(t, r)| (t.to_string(), r.trim()))
            .unwrap_or((remaining.to_string(), ""));
        result.push(token);
        remaining = rest;
    }
    result
}
