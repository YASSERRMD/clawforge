/// Platform-dispatching service controller.
///
/// Mirrors `src/daemon/service.ts` from OpenClaw.
/// Dispatches to launchd (macOS), systemd (Linux), or schtasks (Windows).
use anyhow::Result;
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Linux,
    Windows,
}

pub fn current_platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::MacOs
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

/// Install ClawForge as a system service.
pub async fn install_service(
    profile: Option<&str>,
    program: &str,
    program_args: &[String],
    working_dir: Option<&str>,
    env: Option<&[(String, String)]>,
) -> Result<String> {
    match current_platform() {
        Platform::MacOs => {
            let mut args = vec![program.to_string()];
            args.extend_from_slice(program_args);
            let path = crate::launchd::install_launch_agent(profile, &args, working_dir, env).await?;
            Ok(format!("Installed LaunchAgent: {}", path.display()))
        }
        Platform::Linux => {
            let exec = format!("{} {}", program, program_args.join(" "));
            let path = crate::systemd::install_unit(profile, &exec, working_dir, env).await?;
            Ok(format!("Installed systemd unit: {}", path.display()))
        }
        Platform::Windows => {
            crate::schtasks::install_task(profile, program, program_args).await?;
            Ok(format!("Installed Task Scheduler task: {}", crate::schtasks::task_name(profile)))
        }
    }
}

pub async fn uninstall_service(profile: Option<&str>) -> Result<()> {
    match current_platform() {
        Platform::MacOs => crate::launchd::uninstall_launch_agent(profile).await,
        Platform::Linux => crate::systemd::uninstall_unit(profile).await,
        Platform::Windows => crate::schtasks::uninstall_task(profile).await,
    }
}

pub async fn start_service(profile: Option<&str>) -> Result<()> {
    match current_platform() {
        Platform::MacOs => crate::launchd::start_launch_agent(profile).await,
        Platform::Linux => crate::systemd::start_unit(profile).await,
        Platform::Windows => crate::schtasks::start_task(profile).await,
    }
}

pub async fn stop_service(profile: Option<&str>) -> Result<()> {
    match current_platform() {
        Platform::MacOs => crate::launchd::stop_launch_agent(profile).await,
        Platform::Linux => crate::systemd::stop_unit(profile).await,
        Platform::Windows => crate::schtasks::stop_task(profile).await,
    }
}

pub async fn restart_service(profile: Option<&str>) -> Result<()> {
    match current_platform() {
        Platform::MacOs => crate::launchd::restart_launch_agent(profile).await,
        Platform::Linux => crate::systemd::restart_unit(profile).await,
        Platform::Windows => {
            crate::schtasks::stop_task(profile).await.ok();
            crate::schtasks::start_task(profile).await
        }
    }
}

pub async fn status_service(profile: Option<&str>) -> Result<String> {
    match current_platform() {
        Platform::MacOs => {
            let s = crate::launchd::status_launch_agent(profile).await?;
            let state = if s.running { "running" } else { "stopped" };
            let pid_str = s.pid.map(|p| format!(", pid={}", p)).unwrap_or_default();
            Ok(format!("Status: {}{}", state, pid_str))
        }
        Platform::Linux => crate::systemd::status_unit(profile).await,
        Platform::Windows => crate::schtasks::status_task(profile).await,
    }
}

pub async fn service_audit(profile: Option<&str>) -> Result<Vec<String>> {
    let mut report = vec![];
    let status = status_service(profile).await.unwrap_or_else(|e| format!("error: {e}"));
    report.push(format!("Platform: {:?}", current_platform()));
    report.push(format!("Service status: {}", status));
    Ok(report)
}
