use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::{mpsc, broadcast, RwLock};
use tracing::{debug, error, info, warn};

use uuid::Uuid;
use clawforge_core::types::{AgentSpec, RunState};
use clawforge_core::{Component, Event, EventKind, Message};

use crate::store::EventStore;

/// The Supervisor component logs all audit events, enforces budget policies,
/// and tracks run state.
pub struct Supervisor {
    event_store: EventStore,
    broadcast_tx: RwLock<Option<broadcast::Sender<Event>>>,
    run_states: RwLock<std::collections::HashMap<Uuid, RunState>>,
}

impl Supervisor {
    pub fn new(event_store: EventStore) -> Self {
        Self {
            event_store,
            broadcast_tx: RwLock::new(None),
            run_states: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub async fn set_broadcast_tx(&self, tx: broadcast::Sender<Event>) {
        let mut guard = self.broadcast_tx.write().await;
        *guard = Some(tx);
    }

    /// Check budget constraints after an event.
    fn check_budget(&self, event: &Event) -> Option<EventKind> {
        // Phase 1: basic budget tracking — hard limits in Phase 3
        if let Some(tokens) = event.payload.get("tokens_used").and_then(|v| v.as_u64()) {
            if tokens > 100_000 {
                warn!(
                    run_id = %event.run_id,
                    tokens = tokens,
                    "Token budget warning"
                );
                return Some(EventKind::BudgetWarning);
            }
        }
        None
    }

    /// Get summarized run info from stored events.
    pub fn get_run_summary(&self, run_id: &uuid::Uuid) -> Result<serde_json::Value> {
        let events = self.event_store.get_run_events(run_id)?;
        let status = events
            .last()
            .map(|e| e.kind.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(serde_json::json!({
            "run_id": run_id.to_string(),
            "event_count": events.len(),
            "status": status,
            "events": events.iter().map(|e| serde_json::json!({
                "id": e.id.to_string(),
                "kind": e.kind.to_string(),
                "timestamp": e.timestamp.to_rfc3339(),
            })).collect::<Vec<_>>(),
        }))
    }

    /// Get recent runs summary for the API.
    pub fn get_recent_runs(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        let events = self.event_store.get_recent(limit)?;

        // Group by run_id
        let mut runs: std::collections::HashMap<String, Vec<&Event>> =
            std::collections::HashMap::new();
        for event in &events {
            runs.entry(event.run_id.to_string())
                .or_default()
                .push(event);
        }

        let summaries = runs
            .iter()
            .map(|(run_id, events)| {
                let status = events
                    .first()
                    .map(|e| e.kind.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                serde_json::json!({
                    "run_id": run_id,
                    "event_count": events.len(),
                    "status": status,
                })
            })
            .collect();

        Ok(summaries)
    }

    /// Save an agent spec.
    pub fn save_agent(&self, agent: &AgentSpec) -> Result<()> {
        self.event_store.save_agent(agent)
    }

    /// Get an agent by ID.
    pub fn get_agent(&self, id: &uuid::Uuid) -> Result<Option<AgentSpec>> {
        self.event_store.get_agent(id)
    }

    /// List all agents.
    pub fn list_agents(&self) -> Result<Vec<AgentSpec>> {
        self.event_store.list_agents()
    }
}

#[async_trait]
impl Component for Supervisor {
    fn name(&self) -> &str {
        "supervisor"
    }

    async fn start(&self, mut rx: mpsc::Receiver<Message>) -> Result<()> {
        info!("Supervisor started");

        while let Some(msg) = rx.recv().await {
            match msg {
                Message::AuditEvent(payload) => {
                    let event = &payload.event;

                    // Update local state tracking
                    let mut states = self.run_states.write().await;
                    match &event.kind {
                        EventKind::RunStarted => {
                            states.insert(event.run_id, RunState::Active);
                            info!(run_id = %event.run_id, agent_id = %event.agent_id, "Run started");
                        }
                        EventKind::RunCompleted => {
                            states.insert(event.run_id, RunState::Completed);
                            info!(run_id = %event.run_id, "Run completed");
                        }
                        EventKind::RunFailed => {
                            states.insert(event.run_id, RunState::Failed);
                            warn!(run_id = %event.run_id, "Run failed");
                        }
                        _ => {}
                    }
                    drop(states); // Release lock

                    debug!(
                        run_id = %event.run_id,
                        kind = %event.kind,
                        "Recording audit event"
                    );

                    // Persist the event
                    if let Err(e) = self.event_store.insert(event) {
                        error!(error = %e, "Failed to persist event");
                    } else {
                        // Broadcast event to subscribers (e.g. WebSocket)
                        let tx = self.broadcast_tx.read().await;
                        if let Some(tx) = &*tx {
                            // We don't care if there are no receivers
                            let _ = tx.send(event.clone()); 
                        }
                    }

                    // Check budget constraints
                    if let Some(warning_kind) = self.check_budget(event) {
                        info!(
                            run_id = %event.run_id,
                            warning = %warning_kind,
                            "Budget constraint triggered"
                        );
                    }
                }
                Message::CancelRun(run_id) => {
                    info!(%run_id, "Received cancellation request");
                    let mut states = self.run_states.write().await;
                    if let Some(state) = states.get_mut(&run_id) {
                        *state = RunState::Cancelled;
                        // In a real system, we'd need to signal the Executor to abort
                        // For now, setting state is the first step
                    }
                }
                Message::RequestInput { run_id, prompt } => {
                    info!(%run_id, %prompt, "Agent requested input");
                    let mut states = self.run_states.write().await;
                    states.insert(run_id, RunState::AwaitingInput(prompt));
                }
                Message::ProvideInput { run_id, input } => {
                     info!(%run_id, "Input provided");
                     let mut states = self.run_states.write().await;
                     // Resuming would involve sending a message back to the Planner/Executor
                     // For now, we just update the state back to Active
                     if states.contains_key(&run_id) {
                         states.insert(run_id, RunState::Active);
                     }
                }
                other => {
                    debug!(msg_type = ?other, "Supervisor ignoring non-audit message");
                }
            }
        }

        info!("Supervisor channel closed, shutting down");
        Ok(())
    }
}
