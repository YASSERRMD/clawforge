pub mod clawdbot;
pub mod moltbot;
pub mod node_host;
pub mod registry;
pub mod traits;

pub use clawdbot::Clawdbot;
pub use moltbot::Moltbot;
pub use node_host::{NodeHostRegistry, NodeInvocation, NodeInvocationResult, NodeRegistration, NodeStatus, NodeTransport};
pub use registry::CompanionRegistry;
pub use traits::{CompanionBot, Persona};
