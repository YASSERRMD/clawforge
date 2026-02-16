use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An immutable event in the event-sourcing log.
/// Every action, decision, and state change is recorded as an Event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub run_id: Uuid,
    pub agent_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub kind: EventKind,
    pub payload: serde_json::Value,
}

/// Categories of events that can occur during a run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A run was started
    RunStarted,
    /// A trigger fired
    TriggerFired,
    /// The planner produced a plan
    PlanGenerated,
    /// An action was proposed for execution
    ActionProposed,
    /// An action was approved by capability check
    ActionApproved,
    /// An action was denied by capability check
    ActionDenied,
    /// An action was executed
    ActionExecuted,
    /// An action failed
    ActionFailed,
    /// A run completed successfully
    RunCompleted,
    /// A run failed
    RunFailed,
    /// Budget threshold was reached
    BudgetWarning,
    /// Budget limit was exceeded
    BudgetExceeded,
}

impl Event {
    pub fn new(run_id: Uuid, agent_id: Uuid, kind: EventKind, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            run_id,
            agent_id,
            timestamp: Utc::now(),
            kind,
            payload,
        }
    }
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", self));
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let run_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let event = Event::new(
            run_id,
            agent_id,
            EventKind::RunStarted,
            serde_json::json!({"reason": "manual"}),
        );
        assert_eq!(event.run_id, run_id);
        assert_eq!(event.agent_id, agent_id);
        assert_eq!(event.kind, EventKind::RunStarted);
    }

    #[test]
    fn test_event_serialization() {
        let event = Event::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            EventKind::ActionExecuted,
            serde_json::json!({"result": "ok"}),
        );
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.kind, EventKind::ActionExecuted);
    }

    #[test]
    fn test_event_kind_display() {
        assert_eq!(EventKind::RunStarted.to_string(), "run_started");
        assert_eq!(EventKind::ActionFailed.to_string(), "action_failed");
    }
}
