//! Placeholder constructors for common integration categories.
//!
//! These are convenience builders that produce a [`NewIntegration`] with
//! sensible defaults for each category. They are *connection blueprints*, not
//! live clients — the actual wire protocol is implemented by the runtime; the
//! control plane only governs the connection.

use crate::constants::RiskLevel;

use super::model::{CredentialRef, IntegrationKind, IntegrationPermission, NewIntegration};

/// Placeholder for a database integration (Oracle, SQL Server, Postgres,
/// MongoDB, …). Defaults to read-only `Connect`+`Read` at high risk.
pub fn database(
    name: &str,
    owner: &str,
    department: &str,
    kind: IntegrationKind,
    endpoint: &str,
    credential: CredentialRef,
) -> NewIntegration {
    NewIntegration {
        name: name.into(),
        kind,
        description: "Database integration".into(),
        owner: owner.into(),
        department: department.into(),
        endpoint: endpoint.into(),
        credential,
        permissions: vec![IntegrationPermission::Connect, IntegrationPermission::Read],
        risk_level: RiskLevel::High,
    }
}

/// Placeholder for a GIS integration (ArcGIS and similar spatial services).
pub fn gis(name: &str, owner: &str, department: &str, endpoint: &str, credential: CredentialRef) -> NewIntegration {
    NewIntegration {
        name: name.into(),
        kind: IntegrationKind::ArcGis,
        description: "GIS / spatial data integration".into(),
        owner: owner.into(),
        department: department.into(),
        endpoint: endpoint.into(),
        credential,
        permissions: vec![IntegrationPermission::Connect, IntegrationPermission::Read],
        risk_level: RiskLevel::Medium,
    }
}

/// Placeholder for an SSO / identity-provider integration (OIDC/SAML, AD).
/// Identity integrations are high risk by default.
pub fn sso(name: &str, owner: &str, department: &str, endpoint: &str, credential: CredentialRef) -> NewIntegration {
    NewIntegration {
        name: name.into(),
        kind: IntegrationKind::Sso,
        description: "Single sign-on / identity provider integration".into(),
        owner: owner.into(),
        department: department.into(),
        endpoint: endpoint.into(),
        credential,
        permissions: vec![IntegrationPermission::Connect, IntegrationPermission::Read],
        risk_level: RiskLevel::High,
    }
}

/// Placeholder for an outbound webhook integration.
pub fn webhook(name: &str, owner: &str, department: &str, url: &str) -> NewIntegration {
    NewIntegration {
        name: name.into(),
        kind: IntegrationKind::Webhook,
        description: "Outbound webhook integration".into(),
        owner: owner.into(),
        department: department.into(),
        endpoint: url.into(),
        credential: CredentialRef::none(),
        permissions: vec![IntegrationPermission::Connect, IntegrationPermission::Write],
        risk_level: RiskLevel::Medium,
    }
}
