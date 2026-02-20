pub mod launchd;
pub mod schtasks;
pub mod service;
pub mod systemd;

pub use service::{
    current_platform, install_service, uninstall_service, start_service, stop_service,
    restart_service, status_service, service_audit, Platform,
};
