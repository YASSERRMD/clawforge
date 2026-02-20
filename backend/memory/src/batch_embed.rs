/// Batch embedding pipeline — offline bulk embeddings via provider batch APIs.
///
/// Mirrors `src/memory/batch-openai.ts`, `batch-gemini.ts`, `batch-voyage.ts` from OpenClaw.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Provider config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum BatchEmbedProvider {
    OpenAI { api_key: String, model: String },
    Gemini { api_key: String, model: String },
    Voyage { api_key: String, model: String },
}

// ---------------------------------------------------------------------------
// Batch item
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedItem {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResult {
    pub id: String,
    pub embedding: Vec<f32>,
}

// ---------------------------------------------------------------------------
// Batch embedder
// ---------------------------------------------------------------------------

pub struct BatchEmbedder {
    provider: BatchEmbedProvider,
    /// Max items per batch request.
    pub batch_size: usize,
}

impl BatchEmbedder {
    pub fn new(provider: BatchEmbedProvider) -> Self {
        Self { provider, batch_size: 100 }
    }

    /// Embed all items, chunking into batch_size requests.
    pub async fn embed_all(&self, items: Vec<EmbedItem>) -> Result<Vec<EmbedResult>> {
        let mut results = Vec::with_capacity(items.len());
        for chunk in items.chunks(self.batch_size) {
            info!("[BatchEmbed] Embedding batch of {} items", chunk.len());
            let batch = self.embed_batch(chunk).await?;
            results.extend(batch);
        }
        Ok(results)
    }

    async fn embed_batch(&self, items: &[EmbedItem]) -> Result<Vec<EmbedResult>> {
        match &self.provider {
            BatchEmbedProvider::OpenAI { api_key, model } => {
                embed_openai(api_key, model, items).await
            }
            BatchEmbedProvider::Gemini { api_key, model } => {
                embed_gemini(api_key, model, items).await
            }
            BatchEmbedProvider::Voyage { api_key, model } => {
                embed_voyage(api_key, model, items).await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OpenAI batch embedding
// ---------------------------------------------------------------------------

async fn embed_openai(api_key: &str, model: &str, items: &[EmbedItem]) -> Result<Vec<EmbedResult>> {
    let client = reqwest::Client::new();
    let input: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
    let body = serde_json::json!({ "model": model, "input": input });
    let resp = client
        .post("https://api.openai.com/v1/embeddings")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("OpenAI embedding error: {}", resp.text().await.unwrap_or_default());
    }
    let json: serde_json::Value = resp.json().await?;
    let data = json["data"].as_array();
    let empty = vec![];
    let data = data.unwrap_or(&empty);
    Ok(items.iter().zip(data.iter()).map(|(item, entry)| {
        let embedding = entry["embedding"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            .unwrap_or_default();
        EmbedResult { id: item.id.clone(), embedding }
    }).collect())
}

// ---------------------------------------------------------------------------
// Gemini batch embedding
// ---------------------------------------------------------------------------

async fn embed_gemini(api_key: &str, model: &str, items: &[EmbedItem]) -> Result<Vec<EmbedResult>> {
    let client = reqwest::Client::new();
    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let body = serde_json::json!({
            "content": { "parts": [{ "text": item.text }] }
        });
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
            model, api_key
        );
        let resp = client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            warn!("[BatchEmbed/Gemini] item {} failed: {}", item.id, resp.status());
            results.push(EmbedResult { id: item.id.clone(), embedding: vec![] });
            continue;
        }
        let json: serde_json::Value = resp.json().await?;
        let embedding = json["embedding"]["values"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            .unwrap_or_default();
        results.push(EmbedResult { id: item.id.clone(), embedding });
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// Voyage AI batch embedding
// ---------------------------------------------------------------------------

async fn embed_voyage(api_key: &str, model: &str, items: &[EmbedItem]) -> Result<Vec<EmbedResult>> {
    let client = reqwest::Client::new();
    let input: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
    let body = serde_json::json!({ "model": model, "input": input });
    let resp = client
        .post("https://api.voyageai.com/v1/embeddings")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("Voyage embedding error: {}", resp.text().await.unwrap_or_default());
    }
    let json: serde_json::Value = resp.json().await?;
    let data = json["data"].as_array();
    let empty = vec![];
    let data = data.unwrap_or(&empty);
    Ok(items.iter().zip(data.iter()).map(|(item, entry)| {
        let embedding = entry["embedding"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            .unwrap_or_default();
        EmbedResult { id: item.id.clone(), embedding }
    }).collect())
}
