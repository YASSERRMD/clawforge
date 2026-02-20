/// macOS launchd service management.
///
/// Mirrors `src/daemon/launchd.ts` from OpenClaw.
use anyhow::{bail, Result};
use std::path::PathBuf;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

fn home_dir() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"))
}

pub fn launch_agent_label(profile: Option<&str>) -> String {
    match profile {
        Some(p) if !p.is_empty() => format!("ai.clawforge.gateway.{}", p),
        _ => "ai.clawforge.gateway".to_string(),
    }
}

pub fn launch_agent_plist_path(profile: Option<&str>) -> PathBuf {
    let label = launch_agent_label(profile);
    home_dir().join("Library").join("LaunchAgents").join(format!("{}.plist", label))
}

pub fn gateway_log_paths() -> (PathBuf, PathBuf) {
    let log_dir = home_dir().join(".local").join("share").join("clawforge").join("logs");
    let stdout = log_dir.join("gateway.log");
    let stderr = log_dir.join("gateway.err.log");
    (stdout, stderr)
}

// ---------------------------------------------------------------------------
// Plist builder
// ---------------------------------------------------------------------------

pub fn build_launchd_plist(
    label: &str,
    program_args: &[String],
    working_dir: Option<&str>,
    stdout_path: &str,
    stderr_path: &str,
    env: Option<&[(String, String)]>,
) -> String {
    let args_xml = program_args
        .iter()
        .map(|a| format!("        <string>{}</string>", xml_escape(a)))
        .collect::<Vec<_>>()
        .join("\n");

    let env_xml = if let Some(pairs) = env {
        let entries = pairs
            .iter()
            .map(|(k, v)| format!("        <key>{}</key><string>{}</string>", xml_escape(k), xml_escape(v)))
            .collect::<Vec<_>>()
            .join("\n");
        format!("    <key>EnvironmentVariables</key>\n    <dict>\n{}\n    </dict>\n", entries)
    } else {
        String::new()
    };

    let wd_xml = working_dir
        .map(|d| format!("    <key>WorkingDirectory</key>\n    <string>{}</string>\n", xml_escape(d)))
        .unwrap_or_default();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
{args}
    </array>
{wd}    <key>StandardOutPath</key>
    <string>{stdout}</string>
    <key>StandardErrorPath</key>
    <string>{stderr}</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
{env}</dict>
</plist>
"#,
        label = xml_escape(label),
        args = args_xml,
        wd = wd_xml,
        stdout = xml_escape(stdout_path),
        stderr = xml_escape(stderr_path),
        env = env_xml,
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// launchctl helpers
// ---------------------------------------------------------------------------

async fn launchctl(args: &[&str]) -> Result<(String, i32)> {
    let out = tokio::process::Command::new("launchctl")
        .args(args)
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let code = out.status.code().unwrap_or(-1);
    let combined = if stdout.is_empty() { stderr } else { stdout };
    Ok((combined, code))
}

fn gui_domain() -> String {
    // On macOS, parse gui/UID from `id -u`
    #[cfg(unix)]
    {
        let uid = std::process::Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(501);
        format!("gui/{}", uid)
    }
    #[cfg(not(unix))]
    "gui/501".to_string()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub async fn install_launch_agent(
    profile: Option<&str>,
    program_args: &[String],
    working_dir: Option<&str>,
    env: Option<&[(String, String)]>,
) -> Result<PathBuf> {
    let label = launch_agent_label(profile);
    let plist_path = launch_agent_plist_path(profile);
    let (stdout_path, stderr_path) = gateway_log_paths();

    // Ensure log dir exists
    if let Some(log_dir) = stdout_path.parent() {
        tokio::fs::create_dir_all(log_dir).await?;
    }
    tokio::fs::create_dir_all(plist_path.parent().unwrap()).await?;

    let plist = build_launchd_plist(
        &label,
        program_args,
        working_dir,
        stdout_path.to_str().unwrap_or(""),
        stderr_path.to_str().unwrap_or(""),
        env,
    );
    tokio::fs::write(&plist_path, &plist).await?;
    info!("[Daemon/launchd] Wrote plist: {}", plist_path.display());

    let domain = gui_domain();
    let path_str = plist_path.to_str().unwrap_or("");
    launchctl(&["bootout", &domain, path_str]).await.ok();
    launchctl(&["unload", path_str]).await.ok();
    launchctl(&["enable", &format!("{}/{}", domain, label)]).await.ok();

    let (out, code) = launchctl(&["bootstrap", &domain, path_str]).await?;
    if code != 0 {
        bail!("launchctl bootstrap failed ({}): {}", code, out.trim());
    }
    launchctl(&["kickstart", "-k", &format!("{}/{}", domain, label)]).await.ok();
    info!("[Daemon/launchd] Installed and started {}", label);
    Ok(plist_path)
}

pub async fn uninstall_launch_agent(profile: Option<&str>) -> Result<()> {
    let label = launch_agent_label(profile);
    let plist_path = launch_agent_plist_path(profile);
    let domain = gui_domain();
    let path_str = plist_path.to_str().unwrap_or("");
    launchctl(&["bootout", &domain, path_str]).await.ok();
    launchctl(&["unload", path_str]).await.ok();
    if plist_path.exists() {
        tokio::fs::remove_file(&plist_path).await?;
        info!("[Daemon/launchd] Removed plist: {}", plist_path.display());
    }
    Ok(())
}

pub async fn start_launch_agent(profile: Option<&str>) -> Result<()> {
    let label = launch_agent_label(profile);
    let plist_path = launch_agent_plist_path(profile);
    let domain = gui_domain();
    let (out, code) = launchctl(&["kickstart", "-k", &format!("{}/{}", domain, label)]).await?;
    if code != 0 { bail!("launchctl kickstart failed: {}", out.trim()); }
    Ok(())
}

pub async fn stop_launch_agent(profile: Option<&str>) -> Result<()> {
    let label = launch_agent_label(profile);
    let domain = gui_domain();
    launchctl(&["bootout", &format!("{}/{}", domain, label)]).await.ok();
    Ok(())
}

pub async fn restart_launch_agent(profile: Option<&str>) -> Result<()> {
    stop_launch_agent(profile).await.ok();
    let label = launch_agent_label(profile);
    let plist_path = launch_agent_plist_path(profile);
    let domain = gui_domain();
    let path_str = plist_path.to_str().unwrap_or("");
    let (out, code) = launchctl(&["bootstrap", &domain, path_str]).await?;
    if code != 0 { bail!("launchctl bootstrap failed: {}", out.trim()); }
    start_launch_agent(profile).await
}

pub async fn status_launch_agent(profile: Option<&str>) -> Result<ServiceStatus> {
    let label = launch_agent_label(profile);
    let domain = gui_domain();
    let (out, code) = launchctl(&["print", &format!("{}/{}", domain, label)]).await?;
    if code != 0 {
        return Ok(ServiceStatus { running: false, pid: None, detail: out.trim().to_string() });
    }
    // Parse "pid = N" from launchctl print output
    let pid = out.lines().find_map(|line| {
        let line = line.trim();
        if line.starts_with("pid") {
            line.split('=').nth(1).and_then(|s| s.trim().parse::<u32>().ok())
        } else {
            None
        }
    });
    Ok(ServiceStatus { running: pid.is_some(), pid, detail: String::new() })
}

#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub detail: String,
}
