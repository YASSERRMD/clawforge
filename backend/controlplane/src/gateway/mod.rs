//! Security Gateway — pre-execution checks on every agent action.

pub mod policy;
pub mod request;

pub use policy::SecurityPolicy;
pub use request::ActionRequest;
