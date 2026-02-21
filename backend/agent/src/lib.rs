//! ClawForge Agent Runner
//!
//! The core execution loop for agents, including state management, context window handling,
//! prompt building, and tool dispatching.

pub mod agent_loop;
pub mod assistant_identity;
pub mod chat;
pub mod context_window;
pub mod prompt_cache;
pub mod session_state;
pub mod system_prompt;
pub mod tool_dispatcher;

pub use agent_loop::{AgentRunner, StepResult};
pub use context_window::ContextWindow;
pub use session_state::{SessionState, ModelConfig};
pub use system_prompt::PromptBuilder;
pub use tool_dispatcher::{ToolDispatcher, ToolResult};
