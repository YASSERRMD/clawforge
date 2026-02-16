use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use cron::Schedule;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use clawforge_core::{
    AgentSpec, Component, Message, PlanRequest, TriggerSpec,
};

/// The Scheduler component evaluates agent triggers and dispatches PlanRequest messages.
pub struct Scheduler {
    agents: Vec<AgentSpec>,
    planner_tx: mpsc::Sender<Message>,
    supervisor_tx: mpsc::Sender<Message>,
}

impl Scheduler {
    pub fn new(
        agents: Vec<AgentSpec>,
        planner_tx: mpsc::Sender<Message>,
        supervisor_tx: mpsc::Sender<Message>,
    ) -> Self {
        Self {
            agents,
            planner_tx,
            supervisor_tx,
        }
    }
}

#[async_trait]
impl Component for Scheduler {
    fn name(&self) -> &str {
        "scheduler"
    }

    async fn start(&self, mut rx: mpsc::Receiver<Message>) -> Result<()> {
        info!(
            agent_count = self.agents.len(),
            "Scheduler started"
        );

        // Track next fire time for each cron/interval agent
        let mut next_fires: HashMap<Uuid, tokio::time::Instant> = HashMap::new();

        // Initialize interval agents
        for agent in &self.agents {
            match &agent.trigger {
                TriggerSpec::Interval { seconds } => {
                    let fire_at = tokio::time::Instant::now() + Duration::from_secs(*seconds);
                    next_fires.insert(agent.id, fire_at);
                    info!(
                        agent = %agent.name,
                        interval_secs = seconds,
                        "Registered interval trigger"
                    );
                }
                TriggerSpec::Cron { expression } => {
                    match Schedule::from_str(expression) {
                        Ok(schedule) => {
                            if let Some(next) = schedule.upcoming(Utc).next() {
                                let until = (next - Utc::now())
                                    .to_std()
                                    .unwrap_or(Duration::from_secs(60));
                                let fire_at = tokio::time::Instant::now() + until;
                                next_fires.insert(agent.id, fire_at);
                                info!(
                                    agent = %agent.name,
                                    next = %next,
                                    "Registered cron trigger"
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                agent = %agent.name,
                                error = %e,
                                "Invalid cron expression, skipping"
                            );
                        }
                    }
                }
                TriggerSpec::Webhook { path } => {
                    info!(agent = %agent.name, path = %path, "Registered webhook trigger");
                }
                TriggerSpec::Manual => {
                    debug!(agent = %agent.name, "Manual trigger — waiting for explicit invocation");
                }
            }
        }

        // Main scheduling loop
        let tick_interval = Duration::from_secs(1);
        let mut ticker = time::interval(tick_interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let now = tokio::time::Instant::now();
                    for agent in &self.agents {
                        if let Some(fire_at) = next_fires.get(&agent.id) {
                            if now >= *fire_at {
                                let run_id = Uuid::new_v4();
                                info!(
                                    agent = %agent.name,
                                    run_id = %run_id,
                                    "Trigger fired, dispatching plan request"
                                );

                                let plan_request = Message::PlanRequest(PlanRequest {
                                    run_id,
                                    agent: agent.clone(),
                                    context: serde_json::json!({
                                        "trigger": "scheduled",
                                        "timestamp": Utc::now().to_rfc3339(),
                                    }),
                                });

                                if let Err(e) = self.planner_tx.send(plan_request).await {
                                    error!(error = %e, "Failed to send plan request");
                                }

                                // Reschedule
                                match &agent.trigger {
                                    TriggerSpec::Interval { seconds } => {
                                        next_fires.insert(
                                            agent.id,
                                            now + Duration::from_secs(*seconds),
                                        );
                                    }
                                    TriggerSpec::Cron { expression } => {
                                        if let Ok(schedule) = Schedule::from_str(expression) {
                                            if let Some(next) = schedule.upcoming(Utc).next() {
                                                let until = (next - Utc::now())
                                                    .to_std()
                                                    .unwrap_or(Duration::from_secs(60));
                                                next_fires.insert(agent.id, now + until);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Some(Message::ScheduleJob(trigger)) => {
                            // Manual or webhook trigger received
                            if let Some(agent) = self.agents.iter().find(|a| a.id == trigger.agent_id) {
                                info!(
                                    agent = %agent.name,
                                    run_id = %trigger.run_id,
                                    reason = %trigger.trigger_reason,
                                    "Manual trigger received"
                                );
                                let plan_request = Message::PlanRequest(PlanRequest {
                                    run_id: trigger.run_id,
                                    agent: agent.clone(),
                                    context: serde_json::json!({
                                        "trigger": trigger.trigger_reason,
                                        "timestamp": Utc::now().to_rfc3339(),
                                    }),
                                });
                                if let Err(e) = self.planner_tx.send(plan_request).await {
                                    error!(error = %e, "Failed to send plan request");
                                }
                            } else {
                                warn!(
                                    agent_id = %trigger.agent_id,
                                    "Trigger received for unknown agent"
                                );
                            }
                        }
                        Some(other) => {
                            debug!(msg_type = ?other, "Scheduler ignoring non-schedule message");
                        }
                        None => {
                            info!("Scheduler channel closed, shutting down");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawforge_core::{Capabilities, JobTrigger, LlmPolicy};

    fn test_agent(trigger: TriggerSpec) -> AgentSpec {
        AgentSpec {
            id: Uuid::new_v4(),
            name: "test-agent".to_string(),
            description: "test".to_string(),
            trigger,
            capabilities: Capabilities::default(),
            llm_policy: LlmPolicy::default(),
            role: Default::default(),
            memory_config: None,
            workflow: vec![],
        }
    }

    #[tokio::test]
    async fn test_manual_trigger() {
        let (planner_tx, mut planner_rx) = mpsc::channel(16);
        let (supervisor_tx, _supervisor_rx) = mpsc::channel(16);
        let (scheduler_tx, scheduler_rx) = mpsc::channel(16);

        let agent = test_agent(TriggerSpec::Manual);
        let agent_id = agent.id;
        let scheduler = Scheduler::new(vec![agent], planner_tx, supervisor_tx);

        // Start scheduler in background
        let handle = tokio::spawn(async move {
            scheduler.start(scheduler_rx).await.unwrap();
        });

        // Send manual trigger
        let run_id = Uuid::new_v4();
        scheduler_tx
            .send(Message::ScheduleJob(JobTrigger {
                run_id,
                agent_id,
                trigger_reason: "manual".into(),
            }))
            .await
            .unwrap();

        // Should receive plan request
        let msg = tokio::time::timeout(Duration::from_secs(2), planner_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(msg.run_id(), run_id);

        // Cleanup
        drop(scheduler_tx);
        let _ = tokio::time::timeout(Duration::from_secs(1), handle).await;
    }
}
