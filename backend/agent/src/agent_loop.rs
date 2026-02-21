//! Core agent execution loop.
//!
//! Mirrors `src/agents/runtime.ts` and `src/agents/agent-loop.ts`.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};

use crate::chat::{ChatMessage, ToolCallRequest};
use crate::context_window::ContextWindow;
use crate::session_state::SessionState;
use crate::system_prompt::PromptBuilder;
use crate::tool_dispatcher::ToolDispatcher;

/// Result of a single agent step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepResult {
    /// Agent produced a final response to the user.
    Response(ChatMessage),
    /// Agent called one or more tools; we should execute them and loop.
    ToolCalls(Vec<ToolCallRequest>),
    /// Agent stopped (e.g., hit token limit or finished).
    Stop,
    /// An error occurred during the step.
    Error(String),
}

/// The core agent runner that manages the conversation loop.
pub struct AgentRunner {
    pub session: Arc<tokio::sync::RwLock<SessionState>>,
    pub tool_dispatcher: Arc<ToolDispatcher>,
    pub max_steps: usize,
}

impl AgentRunner {
    pub fn new(
        session: Arc<tokio::sync::RwLock<SessionState>>,
        tool_dispatcher: Arc<ToolDispatcher>,
    ) -> Self {
        Self {
            session,
            tool_dispatcher,
            max_steps: 10, // Max chain length prevent infinite loops
        }
    }

    /// Run the agent loop until it produces a final response or hits the max steps limit.
    #[instrument(skip(self), fields(session_id = %self.session.read().await.session_id))]
    pub async fn run_loop(&self) -> Result<()> {
        info!("Starting agent loop");

        let mut step_count = 0;
        loop {
            if step_count >= self.max_steps {
                warn!("Max steps ({}) reached, stopping loop", self.max_steps);
                break;
            }

            step_count += 1;
            debug!("Agent loop step {}", step_count);

            let step_result = self.execute_single_step().await?;

            match step_result {
                StepResult::Response(msg) => {
                    info!("Agent produced response: {:?}", msg);
                    // Add to transcript
                    let mut session = self.session.write().await;
                    session.transcript.push(msg.clone());
                    break;
                }
                StepResult::ToolCalls(calls) => {
                    info!("Agent invoked {} tools", calls.len());
                    // Execute tools concurrently
                    let results = self.tool_dispatcher.execute_all(calls.clone()).await;
                    
                    // Add calls and results to transcript
                    let mut session = self.session.write().await;
                    for (call, res) in calls.into_iter().zip(results) {
                        session.transcript.push(ChatMessage::tool_result(
                            call.id,
                            serde_json::to_string(&res).unwrap_or_else(|e| e.to_string()),
                        ));
                    }
                    // Loop naturally continues
                }
                StepResult::Stop => {
                    info!("Agent requested stop");
                    break;
                }
                StepResult::Error(err) => {
                    error!("Step error: {}", err);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Single interaction with the LLM.
    async fn execute_single_step(&self) -> Result<StepResult> {
        let session = self.session.read().await;
        
        // 1. Build context window
        let context = ContextWindow::build(&session.transcript, session.model_config.max_context_tokens);
        
        // 2. Build system prompt
        let _sys_prompt = PromptBuilder::build(&session);

        // 3. Call LLM (abstracted behind some interface, mock for now)
        // TODO: call actual LLM provider via `planner` or `providers` crate
        debug!("Calling LLM with {} history messages", context.messages.len());
        
        // Mock response for now
        Ok(StepResult::Stop)
    }
}
