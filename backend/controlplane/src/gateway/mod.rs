//! Security Gateway — pre-execution checks on every agent action.

pub mod decision;
pub mod engine;
pub mod policy;
pub mod request;

pub use decision::SecurityDecision;
pub use engine::SecurityGateway;
pub use policy::SecurityPolicy;
pub use request::ActionRequest;
