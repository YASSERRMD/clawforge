//! Agent Marketplace — a verified internal catalogue of reusable agent templates.

pub mod model;
pub mod store;

pub use model::{AgentTemplate, ComplianceBadge, MarketplaceAgent, NewListing, VerificationBadge};
pub use store::Marketplace;
