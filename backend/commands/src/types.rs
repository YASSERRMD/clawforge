/// Slash command types.
///
/// Mirrors `src/auto-reply/commands-registry.types.ts` from OpenClaw.
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Scope
// ---------------------------------------------------------------------------

/// Where a command can be invoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandScope {
    /// Only via native channel button/menu (not by typing /slash).
    Native,
    /// Only via typed /slash text.
    Text,
    /// Both native and text.
    Both,
}

// ---------------------------------------------------------------------------
// Category
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandCategory {
    Status,
    Session,
    Options,
    Management,
    Tools,
    Media,
    Docks,
}

// ---------------------------------------------------------------------------
// Arg
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandArg {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub arg_type: ArgType,
    pub required: bool,
    /// If true, consumes all remaining text.
    pub capture_remaining: bool,
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgType {
    String,
    Number,
}

// ---------------------------------------------------------------------------
// Command definition
// ---------------------------------------------------------------------------

/// A fully-defined slash command entry in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDef {
    /// Unique key (e.g. "think", "stop", "model").
    pub key: String,
    /// Native channel button name (e.g. "think" → maps to a button).
    pub native_name: Option<String>,
    pub description: String,
    pub scope: CommandScope,
    pub category: CommandCategory,
    /// Primary slash aliases (must start with '/').
    pub text_aliases: Vec<String>,
    pub args: Vec<CommandArg>,
    pub accepts_args: bool,
}

impl CommandDef {
    /// Primary alias (first in list), or key if none.
    pub fn primary_alias(&self) -> &str {
        self.text_aliases.first().map(|s| s.as_str()).unwrap_or(&self.key)
    }
}

// ---------------------------------------------------------------------------
// Parsed invocation
// ---------------------------------------------------------------------------

/// A detected and parsed slash-command invocation.
#[derive(Debug, Clone)]
pub struct CommandInvocation {
    pub key: String,
    pub raw_alias: String,
    /// Positional arguments parsed from remaining text.
    pub args: Vec<String>,
    /// Full remaining text after the command name.
    pub raw_args: String,
}
