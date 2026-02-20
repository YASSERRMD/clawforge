/// Hook pipeline executor.
///
/// The pipeline is the public API surface for firing hooks in the agent run loop.
/// Callers:
/// 1. Agent receives a message → `pipeline.pre_message(...)` → continue or drop
/// 2. Agent calls a tool → `pipeline.pre_tool_call(...)` → approve or block
/// 3. Tool finishes → `pipeline.after_tool_call(...)`
/// 4. Compaction starts → `pipeline.pre_compaction(...)`
/// etc.
use tracing::debug;

use crate::registry::HookRegistry;
use crate::types::{
    CompactionPayload, HookPayload, HookResult, MessagePayload, ModelOverridePayload,
    SessionPayload, ToolCallPayload,
};

/// Top-level pipeline that wraps the registry with convenient method APIs.
#[derive(Clone)]
pub struct HookPipeline {
    pub registry: HookRegistry,
}

impl HookPipeline {
    pub fn new(registry: HookRegistry) -> Self {
        Self { registry }
    }

    pub async fn pre_message(&self, payload: MessagePayload) -> HookResult {
        debug!("[Pipeline] pre_message session={}", payload.session_id);
        self.registry.run(&HookPayload::PreMessage(payload)).await
    }

    pub async fn post_message(&self, payload: MessagePayload) -> HookResult {
        debug!("[Pipeline] post_message session={}", payload.session_id);
        self.registry.run(&HookPayload::PostMessage(payload)).await
    }

    pub async fn pre_tool_call(&self, payload: ToolCallPayload) -> HookResult {
        debug!(
            "[Pipeline] pre_tool_call tool={} session={}",
            payload.tool_name, payload.session_id
        );
        self.registry.run(&HookPayload::PreToolCall(payload)).await
    }

    pub async fn after_tool_call(&self, payload: ToolCallPayload) -> HookResult {
        debug!(
            "[Pipeline] after_tool_call tool={} session={}",
            payload.tool_name, payload.session_id
        );
        self.registry.run(&HookPayload::AfterToolCall(payload)).await
    }

    pub async fn pre_compaction(&self, payload: CompactionPayload) -> HookResult {
        debug!("[Pipeline] pre_compaction session={}", payload.session_id);
        self.registry.run(&HookPayload::PreCompaction(payload)).await
    }

    pub async fn post_compaction(&self, payload: CompactionPayload) -> HookResult {
        debug!("[Pipeline] post_compaction session={}", payload.session_id);
        self.registry.run(&HookPayload::PostCompaction(payload)).await
    }

    pub async fn session_start(&self, payload: SessionPayload) -> HookResult {
        debug!("[Pipeline] session_start session={}", payload.session_id);
        self.registry.run(&HookPayload::SessionStart(payload)).await
    }

    pub async fn session_end(&self, payload: SessionPayload) -> HookResult {
        debug!("[Pipeline] session_end session={}", payload.session_id);
        self.registry.run(&HookPayload::SessionEnd(payload)).await
    }

    pub async fn model_override(&self, payload: ModelOverridePayload) -> HookResult {
        debug!("[Pipeline] model_override session={}", payload.session_id);
        self.registry.run(&HookPayload::ModelOverride(payload)).await
    }
}
