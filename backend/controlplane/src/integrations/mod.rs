//! Enterprise Integrations — governed connectors to enterprise & government systems.

pub mod model;
pub mod placeholders;
pub mod store;

pub use model::{
    CredentialRef, IntegrationKind, IntegrationPermission, IntegrationProvider, NewIntegration,
};
pub use store::IntegrationRegistry;
