use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use clawforge_core::{
    ActionProposal, AuditEventPayload, ClawError, Component, Event, EventKind,
    LlmRequest, Message, PlanRequest, ProposedAction,
};

use crate::providers::ProviderRegistry;

/// The Planner component receives PlanRequests and races multiple LLM providers
/// to generate action proposals.
pub struct LlmPlanner {
    registry: Arc<ProviderRegistry>,
    executor_tx: mpsc::Sender<Message>,
    supervisor_tx: mpsc::Sender<Message>,
}

impl LlmPlanner {
    pub fn new(
        registry: Arc<ProviderRegistry>,
        executor_tx: mpsc::Sender<Message>,
        supervisor_tx: mpsc::Sender<Message>,
    ) -> Self {
        Self {
            registry,
            executor_tx,
            supervisor_tx,
        }
    }

    /// Race all configured providers and return the first successful response.
    async fn parallel_plan(&self, request: &PlanRequest) -> Result<ProposedAction, ClawError> {
        let providers = self.registry.get_providers(&request.agent.llm_policy.providers);

        if providers.is_empty() {
            return Err(ClawError::AllProvidersFailed);
        }

        let llm_request = LlmRequest {
            model: request.agent.llm_policy.model.clone(),
            system_prompt: request.agent.llm_policy.system_prompt.clone(),
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

                    // Emit plan-started event
                    let _ = self
                        .supervisor_tx
                        .send(Message::AuditEvent(AuditEventPayload {
                            event: Event::new(
                                run_id,
                                agent_id,
                                EventKind::RunStarted,
                                serde_json::json!({"source": "planner"}),
                            ),
                        }))
                        .await;

                    match self.parallel_plan(&request).await {
                        Ok(action) => {
                            info!(run_id = %run_id, "Plan generated, sending to executor");

                            // Emit plan-generated event
                            let _ = self
                                .supervisor_tx
                                .send(Message::AuditEvent(AuditEventPayload {
                                    event: Event::new(
                                        run_id,
                                        agent_id,
                                        EventKind::PlanGenerated,
                                        serde_json::json!({"action_type": "llm_response"}),
                                    ),
                                }))
                                .await;

                            // Send action to executor
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

                            let _ = self
                                .supervisor_tx
                                .send(Message::AuditEvent(AuditEventPayload {
                                    event: Event::new(
                                        run_id,
                                        agent_id,
                                        EventKind::RunFailed,
                                        serde_json::json!({"error": e.to_string()}),
                                    ),
                                }))
                                .await;
                        }
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
