/// Session export and slug utilities.
///
/// Mirrors `src/agents/session-slug.ts` + `src/gateway/session-utils.fs.ts` from OpenClaw.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

// ---------------------------------------------------------------------------
// Session slug
// ---------------------------------------------------------------------------

/// Generate a human-readable slug for a session (e.g. "swift-falcon-42").
pub fn session_slug(session_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    const ADJECTIVES: &[&str] = &[
        "swift", "bold", "calm", "sharp", "bright", "quiet", "brave", "keen", "dark", "warm",
    ];
    const NOUNS: &[&str] = &[
        "falcon", "river", "forge", "stone", "flame", "tide", "claw", "dawn", "pine", "wave",
    ];

    let mut h = DefaultHasher::new();
    session_id.hash(&mut h);
    let hash = h.finish();
    let adj = ADJECTIVES[(hash % ADJECTIVES.len() as u64) as usize];
    let noun = NOUNS[((hash >> 16) % NOUNS.len() as u64) as usize];
    let num = hash % 100;
    format!("{}-{}-{}", adj, noun, num)
}

// ---------------------------------------------------------------------------
// Session export (HTML)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp_ms: u64,
}

pub struct SessionExporter {
    pub output_dir: PathBuf,
}

impl SessionExporter {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self { output_dir: output_dir.into() }
    }

    pub async fn export_html(
        &self,
        session_id: &str,
        title: &str,
        messages: &[SessionMessage],
    ) -> Result<PathBuf> {
        tokio::fs::create_dir_all(&self.output_dir).await?;
        let slug = session_slug(session_id);
        let filename = format!("{}-{}.html", slug, session_id_short(session_id));
        let path = self.output_dir.join(&filename);

        let html = render_html(title, messages);
        tokio::fs::write(&path, &html).await?;
        info!("[SessionExport] Exported {} → {}", session_id, path.display());
        Ok(path)
    }
}

fn session_id_short(id: &str) -> &str {
    if id.len() > 8 { &id[..8] } else { id }
}

fn render_html(title: &str, messages: &[SessionMessage]) -> String {
    let msg_html = messages.iter().map(|m| {
        let role_class = match m.role.as_str() {
            "assistant" => "msg-assistant",
            "user" => "msg-user",
            _ => "msg-system",
        };
        format!(
            r#"<div class="message {rc}"><span class="role">{role}</span><div class="content">{content}</div></div>"#,
            rc = role_class,
            role = html_escape(&m.role),
            content = html_escape(&m.content),
        )
    }).collect::<String>();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>{title}</title>
<style>
body {{ font-family: system-ui, sans-serif; max-width: 800px; margin: 2rem auto; padding: 0 1rem; background: #0d1117; color: #c9d1d9; }}
.message {{ margin-bottom: 1rem; border-radius: 8px; padding: 0.75rem 1rem; }}
.msg-user {{ background: #1f2937; }}
.msg-assistant {{ background: #111827; border-left: 3px solid #3b82f6; }}
.msg-system {{ background: #1a1a1a; color: #6b7280; font-style: italic; }}
.role {{ font-weight: 700; font-size: 0.75rem; text-transform: uppercase; color: #6b7280; display: block; margin-bottom: 0.25rem; }}
.content {{ white-space: pre-wrap; line-height: 1.6; }}
</style>
</head>
<body>
<h1>{title}</h1>
{messages}
</body>
</html>"#,
        title = html_escape(title),
        messages = msg_html,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
