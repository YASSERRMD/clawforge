use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::event::Event;
use crate::types::AgentSpec;

/// Messages exchanged between components via the ClawBus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// Scheduler → Bus: a trigger fired, schedule this job
    ScheduleJob(JobTrigger),
    /// Scheduler → Planner: plan the actions for this agent run
    PlanRequest(PlanRequest),
    /// Planner → Executor: execute this proposed action
    ExecuteAction(ActionProposal),
    /// Any → Supervisor: log an audit event
    AuditEvent(AuditEventPayload),
}

/// A trigger event from the scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobTrigger {
    pub run_id: Uuid,
    pub agent_id: Uuid,
    pub trigger_reason: String,
}

/// Request to the planner to generate an action plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRequest {
    pub run_id: Uuid,
    pub agent: AgentSpec,
    pub context: serde_json::Value,
}

/// A proposed action from the planner to the executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionProposal {
    pub run_id: Uuid,
    pub agent_id: Uuid,
    pub step_index: usize,
    pub action: ProposedAction,
}

/// The specific action to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProposedAction {
    ShellCommand {
        command: String,
        args: Vec<String>,
        working_dir: Option<String>,
    },
    HttpRequest {
        method: String,
        url: String,
        headers: std::collections::HashMap<String, String>,
        body: Option<String>,
    },
    LlmResponse {
        content: String,
        provider: String,
        model: String,
        tokens_used: u64,
    },
}

/// Audit event payload for the supervisor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventPayload {
    pub event: Event,
}

impl Message {
    pub fn run_id(&self) -> Uuid {
        match self {
            Message::ScheduleJob(j) => j.run_id,
            Message::PlanRequest(p) => p.run_id,
            Message::ExecuteAction(a) => a.run_id,
            Message::AuditEvent(e) => e.event.run_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization_roundtrip() {
        let msg = Message::ScheduleJob(JobTrigger {
            run_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            trigger_reason: "cron fired".to_string(),
        });
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.run_id(), deserialized.run_id());
    }

    #[test]
    fn test_run_id_extraction() {
        let run_id = Uuid::new_v4();
        let msg = Message::ScheduleJob(JobTrigger {
            run_id,
            agent_id: Uuid::new_v4(),
            trigger_reason: "test".to_string(),
        });
        assert_eq!(msg.run_id(), run_id);
    }
}
