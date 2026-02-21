pub mod builtin;
pub mod evaluator;
pub mod pipeline;
pub mod registry;
pub mod types;

pub use builtin::{ChannelModelOverrideHook, ContentFilterHook, LoggingHook, ToolPolicyHook};
pub use pipeline::HookPipeline;
pub use registry::{Hook, HookRegistry};
pub use evaluator::should_fire;
pub use types::{
    CompactionPayload, HookCondition, HookContext, HookPayload, HookPhase, HookResult, HookTrigger,
    MessagePayload, ModelOverridePayload, SessionPayload, ToolCallPayload,
};
