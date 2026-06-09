//! Agent marketplace domain model.
//!
//! The marketplace is a verified, internal catalogue of reusable agent
//! templates. Publishing puts an agent blueprint on the shelf; installing
//! stamps out a concrete agent into the [`registry`](crate::registry).

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{DataAccessLevel, RiskLevel};
use crate::registry::NewAgent;

/// Verification badge — has the listing been vetted by the platform team?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationBadge {
    /// Not yet reviewed.
    Unverified,
    /// Reviewed and verified by the platform team.
    Verified,
}

/// Compliance badge — where the listing sits in the compliance review process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceBadge {
    /// Compliance review pending.
    Pending,
    /// Passed compliance review.
    Compliant,
    /// Formally certified for regulated use.
    Certified,
}

/// A published marketplace listing wrapping a reusable [`AgentTemplate`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceAgent {
    /// Stable UUID.
    pub id: String,
    pub name: String,
    pub description: String,
    /// Functional category (e.g. `licensing`, `it-ops`, `customer-service`).
    pub category: String,
    /// Owning department.
    pub department: String,
    /// Average user rating (0.0–5.0).
    pub rating: f64,
    /// Number of times the listing has been installed.
    pub install_count: u64,
    /// Risk badge of the template.
    pub risk_level: RiskLevel,
    /// Verification status.
    pub verification: VerificationBadge,
    /// Compliance status.
    pub compliance: ComplianceBadge,
    /// The reusable blueprint.
    pub template: AgentTemplate,
    pub published_at: i64,
}

/// Input used to publish a new listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewListing {
    pub name: String,
    pub description: String,
    pub category: String,
    pub department: String,
    pub template: AgentTemplate,
}

impl MarketplaceAgent {
    /// Materialise a new listing: unverified, pending compliance, zero installs.
    pub fn from_new(input: NewListing) -> Self {
        MarketplaceAgent {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            description: input.description,
            category: input.category,
            department: input.department,
            rating: 0.0,
            install_count: 0,
            risk_level: input.template.risk_level,
            verification: VerificationBadge::Unverified,
            compliance: ComplianceBadge::Pending,
            template: input.template,
            published_at: Utc::now().timestamp(),
        }
    }

    /// Whether this listing is safe to surface as a trusted, install-ready
    /// option: verified and at least compliance-reviewed.
    pub fn is_trusted(&self) -> bool {
        self.verification == VerificationBadge::Verified
            && matches!(self.compliance, ComplianceBadge::Compliant | ComplianceBadge::Certified)
    }
}

/// The reusable blueprint behind a marketplace listing: everything needed to
/// instantiate a concrete agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    /// Framework the instantiated agent runs on.
    pub framework: String,
    /// Default model provider.
    pub model_provider: String,
    /// Default model name.
    pub model_name: String,
    /// Tools the template requires.
    #[serde(default)]
    pub required_tools: Vec<String>,
    /// MCP servers the template requires.
    #[serde(default)]
    pub required_mcp_servers: Vec<String>,
    /// Model providers the template is approved against.
    #[serde(default)]
    pub required_model_providers: Vec<String>,
    /// Data sensitivity the instantiated agent will access.
    pub data_access_level: DataAccessLevel,
    /// Risk level of the instantiated agent.
    pub risk_level: RiskLevel,
}

impl AgentTemplate {
    /// Produce a [`NewAgent`] from this template for the given owner/department.
    pub fn to_new_agent(
        &self,
        name: impl Into<String>,
        description: impl Into<String>,
        owner: impl Into<String>,
        department: impl Into<String>,
    ) -> NewAgent {
        NewAgent {
            name: name.into(),
            description: description.into(),
            owner: owner.into(),
            department: department.into(),
            framework: self.framework.clone(),
            model_provider: self.model_provider.clone(),
            model_name: self.model_name.clone(),
            tools_allowed: self.required_tools.clone(),
            mcp_servers_allowed: self.required_mcp_servers.clone(),
            data_access_level: self.data_access_level,
            risk_level: self.risk_level,
        }
    }
}
