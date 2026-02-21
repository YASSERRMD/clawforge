pub mod env_manager;
pub mod launchd;
pub mod schtasks;
pub mod service;
pub mod service_inspector;
pub mod systemd;

pub use env_manager::{EnvStore, EnvVar};
pub use service::{
    current_platform, install_service, uninstall_service, start_service, stop_service,
    restart_service, status_service, service_audit, Platform,
};
pub use service_inspector::{check_service, inspect_self, inspect_services, ServiceInfo, ServiceStatus};
