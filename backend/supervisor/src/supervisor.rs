use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use clawforge_core::{Component, Event, EventKind, Message};

use crate::store::EventStore;

/// The Supervisor component logs all audit events, enforces budget policies,
/// and tracks run state.
pub struct Supervisor {
    store: EventStore,
}

impl Supervisor {
    pub fn new(store: EventStore) -> Self {
        Self { store }
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
        let events = self.store.get_run_events(run_id)?;
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
        let events = self.store.get_recent(limit)?;

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

                    debug!(
                        run_id = %event.run_id,
                        kind = %event.kind,
                        "Recording audit event"
                    );

                    // Persist the event
                    if let Err(e) = self.store.insert(event) {
                        error!(error = %e, "Failed to persist event");
                    }

                    // Check budget constraints
                    if let Some(warning_kind) = self.check_budget(event) {
                        info!(
                            run_id = %event.run_id,
                            warning = %warning_kind,
                            "Budget constraint triggered"
                        );
                    }

                    // Log key lifecycle events
                    match &event.kind {
                        EventKind::RunStarted => {
                            info!(run_id = %event.run_id, agent_id = %event.agent_id, "Run started");
                        }
                        EventKind::RunCompleted => {
                            info!(run_id = %event.run_id, "Run completed");
                        }
                        EventKind::RunFailed => {
                            warn!(
                                run_id = %event.run_id,
                                payload = %event.payload,
                                "Run failed"
                            );
                        }
                        EventKind::ActionDenied => {
                            warn!(
                                run_id = %event.run_id,
                                payload = %event.payload,
                                "Action denied by capability check"
                            );
                        }
                        _ => {}
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
