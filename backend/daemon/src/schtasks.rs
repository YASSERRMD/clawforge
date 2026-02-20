/// Windows Task Scheduler (schtasks) service management.
///
/// Mirrors `src/daemon/schtasks.ts` from OpenClaw.
use anyhow::{bail, Result};
use tracing::info;

pub fn task_name(profile: Option<&str>) -> String {
    match profile {
        Some(p) if !p.is_empty() => format!("ClawForge\\Gateway_{}", p),
        _ => "ClawForge\\Gateway".to_string(),
    }
}

async fn schtasks(args: &[&str]) -> Result<(String, i32)> {
    let out = tokio::process::Command::new("schtasks")
        .args(args)
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let combined = if stdout.is_empty() { stderr } else { stdout };
    Ok((combined, out.status.code().unwrap_or(-1)))
}

pub async fn install_task(
    profile: Option<&str>,
    program: &str,
    args: &[String],
) -> Result<()> {
    let name = task_name(profile);
    let cmd = format!("{} {}", program, args.join(" "));
    // Delete existing task first (idempotent)
    schtasks(&["/Delete", "/TN", &name, "/F"]).await.ok();
    let (out, code) = schtasks(&[
        "/Create", "/TN", &name,
        "/TR", &cmd,
        "/SC", "ONLOGON",
        "/F",
        "/RL", "HIGHEST",
    ])
    .await?;
    if code != 0 { bail!("schtasks /Create failed: {}", out.trim()); }
    // Start immediately
    schtasks(&["/Run", "/TN", &name]).await.ok();
    info!("[Daemon/schtasks] Installed task: {}", name);
    Ok(())
}

pub async fn uninstall_task(profile: Option<&str>) -> Result<()> {
    let name = task_name(profile);
    schtasks(&["/Delete", "/TN", &name, "/F"]).await.ok();
    Ok(())
}

pub async fn start_task(profile: Option<&str>) -> Result<()> {
    let name = task_name(profile);
    let (out, code) = schtasks(&["/Run", "/TN", &name]).await?;
    if code != 0 { bail!("schtasks /Run failed: {}", out.trim()); }
    Ok(())
}

pub async fn stop_task(profile: Option<&str>) -> Result<()> {
    let name = task_name(profile);
    schtasks(&["/End", "/TN", &name]).await.ok();
    Ok(())
}

pub async fn status_task(profile: Option<&str>) -> Result<String> {
    let name = task_name(profile);
    let (out, _) = schtasks(&["/Query", "/TN", &name, "/FO", "LIST"]).await?;
    Ok(out)
}
