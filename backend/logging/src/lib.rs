//! Telemetry and structured logging components for ClawForge.
//!
//! Handles log redaction, JSON output generation, file rotation, and specialized Agent event logging.

pub mod event_logger;
pub mod logger;
pub mod redact;

pub use event_logger::{AgentEvent, EventLogEntry, EventLogger};
pub use logger::init_logger;
pub use redact::redact_sensitive_data;
