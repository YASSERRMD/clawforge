pub mod client;
pub mod registry;
pub mod server;
pub mod telemetry;
pub mod types;

pub use client::AcpClient;
pub use registry::{SubAgentRegistry, MAX_SUBAGENT_DEPTH};
pub use server::{build_acp_router, AcpServerState};
pub use telemetry::{AcpRequestRecord, AcpTelemetry, AcpTimer, MethodStats};
pub use types::{
    PermissionRequest, PermissionResponse, SpawnRequest, SubAgentAnnouncement, SubAgentSession,
    SubAgentStatus,
};
