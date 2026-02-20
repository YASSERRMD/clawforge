/// Link understanding — extract metadata and content from URLs.
///
/// Mirrors `src/link-understanding/` from OpenClaw.
use anyhow::Result;
use tracing::info;

/// The result of understanding a URL.
#[derive(Debug, Clone)]
pub struct LinkUnderstanding {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content_type: String,
    /// Extracted plain-text body (if HTML).
    pub text: Option<String>,
}

/// Detect the content type from a URL extension or HTTP response headers.
pub fn detect_content_type(url: &str) -> &'static str {
    let lower = url.to_lowercase();
    if lower.ends_with(".pdf") { return "application/pdf"; }
    if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
        || lower.ends_with(".gif") || lower.ends_with(".webp") {
        return "image";
    }
    if lower.ends_with(".mp4") || lower.ends_with(".webm") || lower.ends_with(".mov") {
        return "video";
    }
    if lower.ends_with(".mp3") || lower.ends_with(".wav") || lower.ends_with(".ogg") {
        return "audio";
    }
    "text/html"
}

/// Fetch and understand a URL — return a brief plain-text summary.
pub async fn understand_link(url: &str) -> Result<LinkUnderstanding> {
    info!("[LinkUnderstanding] Fetching {}", url);
    let client = reqwest::Client::new();
    let resp = client.get(url).header("User-Agent", "ClawForge/1.0").send().await?;
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/html")
        .to_string();
    let body = resp.text().await?;

    let text = if content_type.contains("html") {
        Some(strip_html(&body))
    } else {
        None
    };

    Ok(LinkUnderstanding {
        url: url.to_string(),
        title: extract_html_tag(&body, "title"),
        description: extract_meta_description(&body),
        content_type,
        text,
    })
}

fn strip_html(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    // Collapse whitespace
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_html_tag(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let lower = html.to_lowercase();
    let start = lower.find(&open)? + open.len();
    let end = lower.find(&close)?;
    if end > start { Some(html[start..end].trim().to_string()) } else { None }
}

fn extract_meta_description(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let needle = "name=\"description\" content=\"";
    let start = lower.find(needle)? + needle.len();
    let end = html[start..].find('"')?;
    Some(html[start..start + end].to_string())
}
