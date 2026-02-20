/// Skill install pipeline — download and install skills from URLs or GitHub.
///
/// Mirrors `src/agents/skills-install.ts` + `skills-install-download.ts` from OpenClaw.
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Skill source
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SkillSource {
    /// URL to a tarball (.tar.gz or .tar.bz2).
    Url(String),
    /// GitHub `owner/repo` optionally with a subdirectory.
    GitHub { repo: String, subdir: Option<String> },
    /// Local directory path.
    Local(PathBuf),
}

// ---------------------------------------------------------------------------
// Install result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SkillInstallResult {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
}

// ---------------------------------------------------------------------------
// Installer
// ---------------------------------------------------------------------------

pub struct SkillInstaller {
    /// Root directory where skills are installed per-agent.
    pub skills_dir: PathBuf,
}

impl SkillInstaller {
    pub fn new(skills_dir: impl Into<PathBuf>) -> Self {
        Self { skills_dir: skills_dir.into() }
    }

    pub async fn install(&self, name: &str, source: SkillSource) -> Result<SkillInstallResult> {
        let dest = self.skills_dir.join(name);
        tokio::fs::create_dir_all(&dest).await?;

        match &source {
            SkillSource::Url(url) => {
                info!("[SkillInstall] Downloading skill '{}' from {}", name, url);
                self.download_and_extract(url, &dest).await?;
            }
            SkillSource::GitHub { repo, subdir } => {
                let url = format!("https://github.com/{}/archive/refs/heads/main.tar.gz", repo);
                info!("[SkillInstall] Cloning GitHub skill '{}' from {}", name, url);
                self.download_and_extract(&url, &dest).await?;
            }
            SkillSource::Local(path) => {
                info!("[SkillInstall] Copying local skill '{}' from {}", name, path.display());
                copy_dir_all(path, &dest).await?;
            }
        }

        info!("[SkillInstall] Installed skill '{}' → {}", name, dest.display());
        Ok(SkillInstallResult {
            name: name.to_string(),
            path: dest,
            source: format!("{:?}", source),
        })
    }

    async fn download_and_extract(&self, url: &str, dest: &Path) -> Result<()> {
        let client = reqwest::Client::new();
        let resp = client.get(url).send().await?;
        if !resp.status().is_success() {
            bail!("Download failed ({}): {}", resp.status(), url);
        }
        let bytes = resp.bytes().await?;
        // Write to temp file then extract with tar
        let tmp = std::env::temp_dir().join(format!("cf-skill-{}.tar.gz", uuid_v4()));
        tokio::fs::write(&tmp, &bytes).await?;

        let status = tokio::process::Command::new("tar")
            .args(["-xzf", tmp.to_str().unwrap_or(""), "-C", dest.to_str().unwrap_or(""), "--strip-components=1"])
            .status()
            .await?;

        tokio::fs::remove_file(&tmp).await.ok();

        if !status.success() {
            bail!("tar extraction failed for skill archive");
        }
        Ok(())
    }

    pub async fn uninstall(&self, name: &str) -> Result<()> {
        let path = self.skills_dir.join(name);
        if path.exists() {
            tokio::fs::remove_dir_all(&path).await?;
            info!("[SkillInstall] Uninstalled skill '{}'", name);
        }
        Ok(())
    }

    pub async fn list_installed(&self) -> Result<Vec<String>> {
        let mut names = vec![];
        if !self.skills_dir.exists() { return Ok(names); }
        let mut dir = tokio::fs::read_dir(&self.skills_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                names.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(names)
    }
}

async fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    tokio::fs::create_dir_all(dst).await?;
    let mut dir = tokio::fs::read_dir(src).await?;
    while let Some(entry) = dir.next_entry().await? {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().await?.is_dir() {
            Box::pin(copy_dir_all(&src_path, &dst_path)).await?;
        } else {
            tokio::fs::copy(&src_path, &dst_path).await?;
        }
    }
    Ok(())
}

fn uuid_v4() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
}
