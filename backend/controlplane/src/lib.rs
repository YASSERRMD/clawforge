//! # ClawForge Control Plane
//!
//! `clawforge-controlplane` is the enterprise/government control plane for AI agents.
//! Where the rest of the ClawForge workspace *runs* agents, this crate is responsible
//! for **managing, governing, securing, observing, and auditing** them.
//!
//! Positioning: *Kubernetes + ServiceNow + Splunk for AI Agents.*
//!
//! The crate is organised into self-contained domain modules, each added in its own
//! build phase. Phase 1 (foundation) establishes configuration, constants, structured
//! logging, and a unified error type. Later phases introduce `registry`, `governance`,
//! `observability`, `gateway`, `mcp`, `marketplace`, `integrations`, and `compliance`.

// Domain modules are wired in here as each phase lands.
#[macro_use]
pub mod logging;
pub mod config;
pub mod constants;
pub mod error;
pub mod governance;
pub mod registry;

pub use config::ControlPlaneConfig;
pub use constants::{DataAccessLevel, LifecycleStatus, RiskLevel};
pub use error::{ControlPlaneError, Result};
