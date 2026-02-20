/// Linux systemd service management.
///
/// Mirrors `src/daemon/systemd.ts` from OpenClaw.
use anyhow::{bail, Result};
use std::path::PathBuf;
use tracing::info;

fn systemd_unit_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".config").join("systemd").join("user")
}

pub fn unit_name(profile: Option<&str>) -> String {
    match profile {
        Some(p) if !p.is_empty() => format!("clawforge-gateway-{}.service", p),
        _ => "clawforge-gateway.service".to_string(),
    }
}

pub fn unit_path(profile: Option<&str>) -> PathBuf {
    systemd_unit_dir().join(unit_name(profile))
}

pub fn build_unit(
    description: &str,
    exec_start: &str,
    working_dir: Option<&str>,
    env: Option<&[(String, String)]>,
) -> String {
    let wd = working_dir
        .map(|d| format!("WorkingDirectory={}\n", d))
        .unwrap_or_default();
    let env_lines = env
        .iter()
        .flat_map(|pairs| pairs.iter())
        .map(|(k, v)| format!("Environment=\"{}={}\"\n", k, v))
        .collect::<String>();

    format!(
        r#"[Unit]
Description={desc}
After=network.target

[Service]
Type=simple
ExecStart={exec}
{wd}{env}Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
"#,
        desc = description,
        exec = exec_start,
        wd = wd,
        env = env_lines,
    )
}

async fn systemctl(args: &[&str]) -> Result<(String, i32)> {
    let out = tokio::process::Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let combined = if stdout.is_empty() { stderr } else { stdout };
    Ok((combined, out.status.code().unwrap_or(-1)))
}

pub async fn install_unit(
    profile: Option<&str>,
    exec_start: &str,
    working_dir: Option<&str>,
    env: Option<&[(String, String)]>,
) -> Result<PathBuf> {
    let path = unit_path(profile);
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    let content = build_unit("ClawForge Gateway", exec_start, working_dir, env);
    tokio::fs::write(&path, &content).await?;
    systemctl(&["daemon-reload"]).await.ok();
    let name = unit_name(profile);
    let (out, code) = systemctl(&["enable", "--now", &name]).await?;
    if code != 0 { bail!("systemctl enable failed: {}", out.trim()); }
    info!("[Daemon/systemd] Installed {}", name);
    Ok(path)
}

pub async fn uninstall_unit(profile: Option<&str>) -> Result<()> {
    let name = unit_name(profile);
    let path = unit_path(profile);
    systemctl(&["disable", "--now", &name]).await.ok();
    if path.exists() { tokio::fs::remove_file(&path).await?; }
    systemctl(&["daemon-reload"]).await.ok();
    Ok(())
}

pub async fn start_unit(profile: Option<&str>) -> Result<()> {
    let (out, code) = systemctl(&["start", &unit_name(profile)]).await?;
    if code != 0 { bail!("systemctl start failed: {}", out.trim()); }
    Ok(())
}

pub async fn stop_unit(profile: Option<&str>) -> Result<()> {
    systemctl(&["stop", &unit_name(profile)]).await.ok();
    Ok(())
}

pub async fn restart_unit(profile: Option<&str>) -> Result<()> {
    let (out, code) = systemctl(&["restart", &unit_name(profile)]).await?;
    if code != 0 { bail!("systemctl restart failed: {}", out.trim()); }
    Ok(())
}

pub async fn status_unit(profile: Option<&str>) -> Result<String> {
    let (out, _) = systemctl(&["status", &unit_name(profile)]).await?;
    Ok(out)
}
