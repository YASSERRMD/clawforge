//! Agent status lifecycle and allowed transitions.
//!
//! The registry models an agent's life as a small state machine. Transitions
//! that would skip governance (e.g. `Draft` straight to `Active`) are rejected
//! so that an agent can only become operational after passing through approval.
//!
//! ```text
//!   Draft ──submit──▶ PendingApproval ──approve──▶ Active
//!     │                    │                        │  ▲
//!     │                    └──reject──▶ Draft        │  │ resume
//!     │                                         suspend  │
//!     ▼                                              ▼  │
//!   Deactivated ◀──────────────────────────────── Suspended
//!     ▲                                              │
//!     └──────────────── Blocked ◀───────────────────┘
//! ```

use crate::constants::LifecycleStatus;

/// Whether an agent may move directly from `from` to `to`.
pub fn can_transition(from: LifecycleStatus, to: LifecycleStatus) -> bool {
    use LifecycleStatus::*;
    // Deactivation and blocking are always permitted as administrative overrides.
    if matches!(to, Deactivated | Blocked) {
        return from != Deactivated;
    }
    match (from, to) {
        (Draft, PendingApproval) => true,
        (PendingApproval, Active) => true,
        (PendingApproval, Draft) => true, // rejected back to draft
        (Active, Suspended) => true,
        (Suspended, Active) => true,
        (Blocked, Active) => true, // unblock
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LifecycleStatus::*;

    #[test]
    fn draft_cannot_jump_to_active() {
        assert!(!can_transition(Draft, Active));
        assert!(can_transition(Draft, PendingApproval));
    }

    #[test]
    fn approval_gate_enables_active() {
        assert!(can_transition(PendingApproval, Active));
        assert!(can_transition(PendingApproval, Draft));
    }

    #[test]
    fn deactivation_is_terminal() {
        assert!(can_transition(Active, Deactivated));
        assert!(!can_transition(Deactivated, Active));
    }

    #[test]
    fn blocking_allowed_from_active() {
        assert!(can_transition(Active, Blocked));
        assert!(can_transition(Blocked, Active));
    }
}
