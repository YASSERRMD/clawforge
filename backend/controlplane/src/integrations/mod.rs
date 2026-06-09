//! Enterprise Integrations — governed connectors to enterprise & government systems.

pub mod model;

pub use model::{
    CredentialRef, IntegrationKind, IntegrationPermission, IntegrationProvider, NewIntegration,
};
