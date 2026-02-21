//! Service inspector: inspect live daemon and agent processes.
//!
//! Provides runtime diagnostics: process info, memory usage, health checks.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::debug;

/// Information about a running service or process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub pid: Option<u32>,
    pub status: ServiceStatus,
    pub uptime_secs: Option<u64>,
    pub memory_kb: Option<u64>,
    pub cpu_percent: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Running,
    Stopped,
    Failed,
    Unknown,
}

/// Inspect the ClawForge daemon process itself.
pub fn inspect_self() -> ServiceInfo {
    let pid = std::process::id();
    let uptime = None; // Could track start time via static OnceCell

    // Read memory from /proc/self/status on Linux.
    let memory_kb = read_self_memory_kb();

    ServiceInfo {
        name: "clawforge-daemon".to_string(),
        pid: Some(pid),
        status: ServiceStatus::Running,
        uptime_secs: uptime,
        memory_kb,
        cpu_percent: None,
    }
}

/// Read the VmRSS (resident memory) of the current process in KB.
fn read_self_memory_kb() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let kb: u64 = line
                    .split_whitespace()
                    .nth(1)?
                    .parse()
                    .ok()?;
                return Some(kb);
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Check if a named system service is running (macOS launchd / Linux systemd).
pub async fn check_service(name: &str) -> ServiceStatus {
    debug!(service = %name, "Checking service status");

    #[cfg(target_os = "macos")]
    {
        let output = tokio::process::Command::new("launchctl")
            .args(["list", name])
            .output()
            .await;
        match output {
            Ok(o) if o.status.success() => ServiceStatus::Running,
            _ => ServiceStatus::Stopped,
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = tokio::process::Command::new("systemctl")
            .args(["is-active", "--quiet", name])
            .output()
            .await;
        match output {
            Ok(o) if o.status.success() => ServiceStatus::Running,
            Ok(_) => ServiceStatus::Stopped,
            Err(_) => ServiceStatus::Unknown,
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        ServiceStatus::Unknown
    }
}

/// Collect diagnostics for multiple services.
pub async fn inspect_services(names: &[&str]) -> Vec<ServiceInfo> {
    let mut infos = Vec::new();
    for name in names {
        let status = check_service(name).await;
        infos.push(ServiceInfo {
            name: name.to_string(),
            pid: None,
            status,
            uptime_secs: None,
            memory_kb: None,
            cpu_percent: None,
        });
    }
    infos
}
