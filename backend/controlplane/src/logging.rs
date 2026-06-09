//! Thin structured-logging helpers over `tracing`.
//!
//! The control plane does not own the global subscriber (the CLI / daemon does),
//! but it standardises *how* control-plane events are emitted so that audit and
//! observability tooling can rely on a consistent `target` and field set.

/// `tracing` target used by all control-plane emissions.
pub const LOG_TARGET: &str = "clawforge::controlplane";

/// Log an informational control-plane event with a stable `action` field.
#[macro_export]
macro_rules! cp_info {
    ($action:expr, $($arg:tt)*) => {
        tracing::info!(target: $crate::logging::LOG_TARGET, action = $action, $($arg)*)
    };
}

/// Log a control-plane warning (e.g. policy violation, degraded health).
#[macro_export]
macro_rules! cp_warn {
    ($action:expr, $($arg:tt)*) => {
        tracing::warn!(target: $crate::logging::LOG_TARGET, action = $action, $($arg)*)
    };
}

/// Log a control-plane denial / blocked action at error level.
#[macro_export]
macro_rules! cp_blocked {
    ($action:expr, $($arg:tt)*) => {
        tracing::error!(target: $crate::logging::LOG_TARGET, action = $action, blocked = true, $($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_is_namespaced() {
        assert!(LOG_TARGET.starts_with("clawforge::"));
    }

    #[test]
    fn macros_expand_without_panicking() {
        cp_info!("test.action", detail = "ok");
        cp_warn!("test.action", detail = "degraded");
        cp_blocked!("test.action", reason = "denied");
    }
}
