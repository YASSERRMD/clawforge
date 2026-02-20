use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use clawforge_core::{
    ActionProposal, AuditEventPayload, ClawError, Component, Event, EventKind,
    LlmRequest, Message, PlanRequest, ProposedAction,
    message::MemoryQueryRequest, // Add this
};

use crate::providers::ProviderRegistry;

/// The Planner component receives PlanRequests and races multiple LLM providers
/// to generate action proposals.
pub struct LlmPlanner {
    registry: Arc<ProviderRegistry>,
    executor_tx: mpsc::Sender<Message>,
    supervisor_tx: mpsc::Sender<Message>,
    memory_tx: Option<mpsc::Sender<Message>>,
    // We will inject tool definitions into the prompt, but the Executor actually runs them.
    // The planner needs to know ABOUT them.
}

impl LlmPlanner {
    pub fn new(
        registry: Arc<ProviderRegistry>,
        executor_tx: mpsc::Sender<Message>,
        supervisor_tx: mpsc::Sender<Message>,
        memory_tx: Option<mpsc::Sender<Message>>,
    ) -> Self {
        Self {
            registry,
            executor_tx,
            supervisor_tx,
            memory_tx,
        }
    }

    /// Race all configured providers and return the first successful response.
    async fn parallel_plan(&self, request: &PlanRequest) -> Result<ProposedAction, ClawError> {
        let providers = self.registry.get_providers(&request.agent.llm_policy.providers);

        if providers.is_empty() {
            return Err(ClawError::AllProvidersFailed);
        }

        // Inject tool context if agent has tools
        let mut system_prompt = request.agent.llm_policy.system_prompt.clone();
        if !request.agent.allowed_tools.is_empty() {
             system_prompt.push_str("\n\nYou have access to the following tools:\n");
             for tool in &request.agent.allowed_tools {
                 system_prompt.push_str(&format!("- {}\n", tool));
                 // In a real implementation, we would look up the tool definition and inject schema here
             }
             system_prompt.push_str("\nTo use a tool, reply in the format:\nAction: ToolName(arg1=\"value\", arg2=\"value\")\n");
        }

        // Inject skills context if agent has skills
        if !request.agent.allowed_skills.is_empty() {
             system_prompt.push_str("\n\n=== AVAILABLE SKILLS ===\n");
             for skill in &request.agent.allowed_skills {
                 match crate::skills::load_skill(skill).await {
                     Ok(content) => {
                         system_prompt.push_str(&format!("\n--- SKILL: {} ---\n{}\n-------------------\n", skill, content));
                     }
                     Err(e) => {
                         warn!(%skill, error = %e, "Failed to load skill for agent prompt");
                     }
                 }
             }
             system_prompt.push_str("========================\n");
        }

        let llm_request = LlmRequest {
            model: request.agent.llm_policy.model.clone(),
            system_prompt,
            user_prompt: serde_json::to_string_pretty(&request.context)
                .unwrap_or_else(|_| request.context.to_string()),
            max_tokens: request.agent.llm_policy.max_tokens,
            temperature: request.agent.llm_policy.temperature,
        };

        info!(
            provider_count = providers.len(),
            model = %llm_request.model,
            "Racing LLM providers"
        );

        let start = Instant::now();

        // Create futures for all providers
        let futures: Vec<_> = providers
            .iter()
            .map(|provider| {
                let req = llm_request.clone();
                let provider = Arc::clone(provider);
                async move {
                    let name = provider.name().to_string();
                    debug!(provider = %name, "Calling provider");
                    match provider.complete(&req).await {
                        Ok(response) => {
                            info!(
                                provider = %name,
                                tokens = response.tokens_used,
                                latency_ms = response.latency_ms,
                                "Provider responded"
                            );
                            Ok(response)
                        }
                        Err(e) => {
                            warn!(provider = %name, error = %e, "Provider failed");
                            Err(e)
                        }
                    }
                }
            })
            .collect();

        // Race all providers — first success wins
        let mut last_error = None;
        let mut join_set = tokio::task::JoinSet::new();
        for fut in futures {
            join_set.spawn(fut);
        }

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(response)) => {
                    let elapsed = start.elapsed();
                    info!(
                        provider = %response.provider,
                        total_latency_ms = elapsed.as_millis(),
                        "Plan generated"
                    );

                    info!(
                        provider = %response.provider,
                        total_latency_ms = elapsed.as_millis(),
                        "Plan generated"
                    );

                    // Simple parser for "Action: ToolName(json_args)"
                    // Example: Action: file_write({"path": "foo.txt", "content": "bar"})
                    if let Some(action_line) = response.content.lines().find(|l| l.starts_with("Action: ")) {
                        let content = action_line.trim_start_matches("Action: ").trim();
                        // simplistic parsing: Name(Args)
                        if let Some(idx) = content.find('(') {
                             if content.ends_with(')') {
                                 let tool_name = &content[..idx];
                                 let args_str = &content[idx+1..content.len()-1];
                                 
                                 // Try to parse args as JSON (assuming the LLM output valid JSON inside parens)
                                 // Or lenient parsing could go here.
                                 if let Ok(args) = serde_json::from_str::<serde_json::Value>(args_str) {
                                     info!(tool = %tool_name, "Parsed tool call");
                                     return Ok(ProposedAction::ToolCall {
                                         name: tool_name.to_string(),
                                         args,
                                     });
                                 } else {
                                     warn!("Failed to parse tool args as JSON: {}", args_str);
                                 }
                             }
                        }
                    } else if let Some(code_block) = response.content.strip_prefix("```json") {
                        // Support JSON output for tools too
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(code_block.trim_end_matches("```").trim()) {
                             if let Some(tool) = val.get("tool").and_then(|t| t.as_str()) {
                                 if let Some(args) = val.get("args") {
                                      return Ok(ProposedAction::ToolCall {
                                         name: tool.to_string(),
                                         args: args.clone(),
                                     });
                                 }
                             }
                        }
                    }

                    // Fallback to text response
                    return Ok(ProposedAction::LlmResponse {
                        content: response.content,
                        provider: response.provider,
                        model: response.model,
                        tokens_used: response.tokens_used,
                    });
                }
                Ok(Err(e)) => {
                    last_error = Some(e);
                }
                Err(e) => {
                    error!(error = %e, "Provider task panicked");
                }
            }
        }

        if let Some(e) = last_error {
            Err(ClawError::LlmError {
                provider: "all".to_string(),
                message: e.to_string(),
            })
        } else {
            Err(ClawError::AllProvidersFailed)
        }
    }
}

#[async_trait]
impl Component for LlmPlanner {
    fn name(&self) -> &str {
        "planner"
    }

    async fn start(&self, mut rx: mpsc::Receiver<Message>) -> Result<()> {
        info!("Planner started");
        
        // Track pending plan requests waiting for memory: run_id -> PlanRequest
        let mut pending_plans: std::collections::HashMap<uuid::Uuid, PlanRequest> = std::collections::HashMap::new();

        while let Some(msg) = rx.recv().await {
            match msg {
                Message::PlanRequest(request) => {
                    let run_id = request.run_id;
                    let agent_id = request.agent.id;

                    info!(
                        run_id = %run_id,
                        agent = %request.agent.name,
                        "Processing plan request"
                    );

                    // 1. Emit start event
                    let _ = self.supervisor_tx.send(Message::AuditEvent(AuditEventPayload {
                        event: Event::new(run_id, agent_id, EventKind::RunStarted, serde_json::json!({"source": "planner"}))
                    })).await;

                    // 2. Check memory config
                    if let Some(_mem_config) = &request.agent.memory_config {
                         // Only query if we have a memory channel
                         if let Some(mem_tx) = &self.memory_tx {
                            info!(run_id = %run_id, "Querying memory for context");
                            
                            // Construct query vector (mock for now, ideally embed user prompt)
                            // In a real system, we'd embed request.context
                            let mock_query = vec![0.0; 1536]; 

                            let query = MemoryQueryRequest {
                                run_id,
                                agent_id,
                                query_vector: mock_query,
                                min_score: 0.7,
                                limit: 3,
                            };
                            
                            // Store pending request
                            pending_plans.insert(run_id, request);
                            
                            if let Err(e) = mem_tx.send(Message::MemoryQuery(query)).await {
                                error!(error = %e, "Failed to send memory query");
                                // Fallback: plan without memory
                                // Retrieve request back (clone needed if we didn't insert, but we did)
                                // Ideally we recover. For now, just log.
                            }
                            continue;
                         }
                    }

                    // 3. No memory or no config -> Plan immediately
                    self.execute_planning(request).await;
                }
                Message::MemoryResponse(response) => {
                    if let Some(mut request) = pending_plans.remove(&response.run_id) {
                         info!(run_id = %response.run_id, results = response.results.len(), "Received memory context");
                         
                         // Enrich context with memory results
                         if let serde_json::Value::Object(ref mut map) = request.context {
                             map.insert("memory_context".to_string(), serde_json::json!(response.results));
                         }

                         self.execute_planning(request).await;
                    } else {
                        warn!(run_id = %response.run_id, "Received memory response for unknown run");
                    }
                }
                other => {
                    debug!(msg_type = ?other, "Planner ignoring non-plan message");
                }
            }
        }

        info!("Planner channel closed, shutting down");
        Ok(())
    }
}

impl LlmPlanner {
    /// Run the planning logic and dispatch to executor.
    async fn execute_planning(&self, request: PlanRequest) {
        let run_id = request.run_id;
        let agent_id = request.agent.id;

        match self.parallel_plan(&request).await {
            Ok(action) => {
                info!(run_id = %run_id, "Plan generated, sending to executor");
                let _ = self.supervisor_tx.send(Message::AuditEvent(AuditEventPayload {
                    event: Event::new(run_id, agent_id, EventKind::PlanGenerated, serde_json::json!({"action_type": "llm_response"})),
                })).await;

                let proposal = Message::ExecuteAction(ActionProposal {
                    run_id,
                    agent_id,
                    step_index: 0,
                    action,
                });

                if let Err(e) = self.executor_tx.send(proposal).await {
                    error!(error = %e, "Failed to send action to executor");
                }
            }
            Err(e) => {
                error!(run_id = %run_id, error = %e, "Planning failed");
                let _ = self.supervisor_tx.send(Message::AuditEvent(AuditEventPayload {
                    event: Event::new(run_id, agent_id, EventKind::RunFailed, serde_json::json!({"error": e.to_string()})),
                })).await;
            }
        }
    }
}
