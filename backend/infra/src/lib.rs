//! Infrastructure module for ClawForge.
//!
//! Provides operational support metrics, cost tracking, log analysis utilities,
//! and usage metrics required for auditing and dashboard representations.

pub mod channel_activity;
pub mod cost_tracker;
pub mod usage_scanner;

pub use channel_activity::{ChannelActivity, ChannelActivityMonitor};
pub use cost_tracker::{CostRecord, CostTracker, TokenUsage};
pub use usage_scanner::{UsageReport, UsageScanner};
