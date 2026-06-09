//! Agent Marketplace — a verified internal catalogue of reusable agent templates.

pub mod model;
pub mod seed;
pub mod store;

pub use model::{AgentTemplate, ComplianceBadge, MarketplaceAgent, NewListing, VerificationBadge};
pub use seed::{sample_listings, seed};
pub use store::Marketplace;
