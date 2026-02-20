use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::info;

const CLAWHUB_BASE_URL: &str = "https://raw.githubusercontent.com/YASSERRMD/clawforge/main/tmp/openclaw/skills";

/// Locate and load a skill's SKILL.md content.
///
/// Looks in `~/.clawforge/workspace/skills/` -> `assets/skills/` -> Remote (ClawHub).
pub async fn load_skill(skill_name: &str) -> Result<String> {
    // 1. Check ~/.clawforge/workspace/skills/<name>/SKILL.md
    if let Some(mut home) = dirs::home_dir() {
        home.push(".clawforge");
        home.push("workspace");
        home.push("skills");
        home.push(skill_name);
        home.push("SKILL.md");

        if home.exists() {
            return std::fs::read_to_string(&home)
                .with_context(|| format!("Failed to read skill file: {}", home.display()));
        }
    }

    // 2. Check local assets/skills/<name>/SKILL.md
    let local_path = PathBuf::from("assets")
        .join("skills")
        .join(skill_name)
        .join("SKILL.md");

    if local_path.exists() {
        return std::fs::read_to_string(&local_path)
            .with_context(|| format!("Failed to read skill file: {}", local_path.display()));
    }

    // 3. Check Remote (ClawHub Integration)
    info!(%skill_name, "Skill not found locally, attempting to fetch from ClawHub");
    let remote_url = format!("{}/{}/SKILL.md", CLAWHUB_BASE_URL, skill_name);
    let response = reqwest::get(&remote_url)
        .await
        .with_context(|| format!("Failed to fetch skill from ClawHub: {}", remote_url))?;

    if response.status().is_success() {
        let content = response.text().await?;
        
        // Try to cache it in the workspace for next time
        if let Some(mut home) = dirs::home_dir() {
            home.push(".clawforge");
            home.push("workspace");
            home.push("skills");
            home.push(skill_name);
            
            if let Err(e) = std::fs::create_dir_all(&home) {
                tracing::warn!(error = %e, "Failed to create skill cache directory");
            } else {
                home.push("SKILL.md");
                let _ = std::fs::write(&home, &content);
                info!(%skill_name, "Cached downloaded skill to workspace");
            }
        }
        
        return Ok(content);
    }

    anyhow::bail!("Skill '{}' not found in workspace, assets, or ClawHub (HTTP {})", skill_name, response.status())
}
