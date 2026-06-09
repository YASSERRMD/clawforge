//! Sample marketplace listings for demos and local development.

use crate::constants::{DataAccessLevel, RiskLevel};
use crate::error::Result;

use super::model::{AgentTemplate, ComplianceBadge, MarketplaceAgent, NewListing, VerificationBadge};
use super::store::Marketplace;

/// The built-in catalogue of example listings.
pub fn sample_listings() -> Vec<NewListing> {
    vec![
        NewListing {
            name: "Permit Intake Assistant".into(),
            description: "Triages building-permit applications and routes them.".into(),
            category: "licensing".into(),
            department: "Licensing".into(),
            template: AgentTemplate {
                framework: "openclaw".into(),
                model_provider: "anthropic".into(),
                model_name: "claude-opus-4-8".into(),
                required_tools: vec!["search".into(), "document.read".into()],
                required_mcp_servers: vec!["records-mcp".into()],
                required_model_providers: vec!["anthropic".into()],
                data_access_level: DataAccessLevel::Internal,
                risk_level: RiskLevel::Medium,
            },
        },
        NewListing {
            name: "Service Desk Responder".into(),
            description: "Answers common citizen service-desk questions.".into(),
            category: "customer-service".into(),
            department: "Customer Happiness".into(),
            template: AgentTemplate {
                framework: "openclaw".into(),
                model_provider: "anthropic".into(),
                model_name: "claude-sonnet-4-6".into(),
                required_tools: vec!["search".into()],
                required_mcp_servers: vec![],
                required_model_providers: vec!["anthropic".into()],
                data_access_level: DataAccessLevel::Public,
                risk_level: RiskLevel::Low,
            },
        },
    ]
}

/// Publish the sample listings, marking them verified and compliant so the
/// "trusted" catalogue has content out of the box.
pub fn seed(mkt: &Marketplace) -> Result<Vec<MarketplaceAgent>> {
    let mut out = Vec::new();
    for input in sample_listings() {
        let listing = mkt.publish(input)?;
        mkt.set_verification(&listing.id, VerificationBadge::Verified)?;
        let listing = mkt.set_compliance(&listing.id, ComplianceBadge::Compliant)?;
        out.push(listing);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_produces_trusted_listings() {
        let mkt = Marketplace::in_memory().unwrap();
        let listings = seed(&mkt).unwrap();
        assert_eq!(listings.len(), 2);
        assert!(listings.iter().all(|l| l.is_trusted()));
    }
}
