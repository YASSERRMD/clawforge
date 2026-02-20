/// Web tools — web fetch and web search for agents.
///
/// Mirrors `src/agents/tools/web-fetch.ts` and `web-search.ts`.
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Web Fetch
// ---------------------------------------------------------------------------

/// Input for web_fetch tool.
#[derive(Debug, Deserialize)]
pub struct WebFetchInput {
    pub url: String,
    /// Optional max bytes to return (default 100 KB)
    pub max_bytes: Option<usize>,
}

/// Output from web_fetch tool.
#[derive(Debug, Serialize)]
pub struct WebFetchOutput {
    pub url: String,
    pub status: u16,
    pub content_type: String,
    pub body: String,
    pub truncated: bool,
}

const DEFAULT_MAX_BYTES: usize = 100_000;
const SSRF_BLOCKED_HOSTS: &[&str] = &["localhost", "127.0.0.1", "0.0.0.0", "::1", "169.254.169.254"];

/// Fetch a URL, stripping HTML to plain text.
/// Blocks SSRF targets (localhost, AWS metadata IP, etc.)
pub async fn web_fetch(client: &Client, input: WebFetchInput) -> Result<WebFetchOutput> {
    // SSRF guard
    let parsed = url::Url::parse(&input.url)?;
    let host = parsed.host_str().unwrap_or("").to_lowercase();
    if SSRF_BLOCKED_HOSTS.iter().any(|b| host.contains(b)) {
        anyhow::bail!("SSRF: blocked host {}", host);
    }

    let max_bytes = input.max_bytes.unwrap_or(DEFAULT_MAX_BYTES);
    let resp = client
        .get(&input.url)
        .header("User-Agent", "ClawForge/1.0 (+https://clawforge.ai)")
        .send()
        .await?;

    let status = resp.status().as_u16();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/html")
        .to_string();

    let bytes = resp.bytes().await?;
    let truncated = bytes.len() > max_bytes;
    let slice = &bytes[..bytes.len().min(max_bytes)];
    let raw = String::from_utf8_lossy(slice).to_string();

    // Strip HTML tags for cleaner content
    let body = if content_type.contains("html") {
        strip_html(&raw)
    } else {
        raw
    };

    Ok(WebFetchOutput {
        url: input.url,
        status,
        content_type,
        body,
        truncated,
    })
}

fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    // Collapse whitespace
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Web Search  (DuckDuckGo Instant Answer API — no key required)
// ---------------------------------------------------------------------------

/// Input for web_search tool.
#[derive(Debug, Deserialize)]
pub struct WebSearchInput {
    pub query: String,
    pub max_results: Option<usize>,
}

/// A single search result.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Output from web_search tool.
#[derive(Debug, Serialize)]
pub struct WebSearchOutput {
    pub query: String,
    pub hits: Vec<SearchHit>,
}

/// Search the web using the DuckDuckGo Instant Answer API.
pub async fn web_search(client: &Client, input: WebSearchInput) -> Result<WebSearchOutput> {
    let limit = input.max_results.unwrap_or(5).min(10);
    let url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_redirect=1&no_html=1",
        urlencoding::encode(&input.query)
    );

    #[derive(Deserialize)]
    struct DdgResult {
        #[serde(rename = "RelatedTopics")]
        related_topics: Vec<DdgTopic>,
    }

    #[derive(Deserialize)]
    struct DdgTopic {
        #[serde(rename = "FirstURL")]
        first_url: Option<String>,
        #[serde(rename = "Text")]
        text: Option<String>,
    }

    let res: DdgResult = client
        .get(&url)
        .header("User-Agent", "ClawForge/1.0")
        .send()
        .await?
        .json()
        .await?;

    let hits = res
        .related_topics
        .into_iter()
        .filter_map(|t| {
            Some(SearchHit {
                title: input.query.clone(),
                url: t.first_url?,
                snippet: t.text.unwrap_or_default(),
            })
        })
        .take(limit)
        .collect();

    Ok(WebSearchOutput {
        query: input.query,
        hits,
    })
}
