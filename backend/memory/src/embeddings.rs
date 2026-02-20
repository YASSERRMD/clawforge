/// Embedding providers for ClawForge memory.
///
/// Supports: OpenAI, Voyage AI, Google Gemini
/// All providers implement the `EmbeddingProvider` trait.
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Return the embedding dimension for this provider/model.
    fn dimension(&self) -> usize;
    /// Embed a single text string.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    /// Embed a batch of texts (default: sequential).
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut out = Vec::with_capacity(texts.len());
        for text in texts {
            out.push(self.embed(text).await?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// OpenAI
// ---------------------------------------------------------------------------

pub struct OpenAIEmbeddings {
    api_key: String,
    model: String,
    dimension: usize,
    client: Client,
}

impl OpenAIEmbeddings {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let model = model.unwrap_or_else(|| "text-embedding-3-small".to_string());
        let dimension = if model.contains("3-large") { 3072 } else { 1536 };
        Self { api_key, model, dimension, client: Client::new() }
    }
}

#[derive(Serialize)]
struct OpenAIEmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct OpenAIEmbedResponse {
    data: Vec<OpenAIEmbedData>,
}

#[derive(Deserialize)]
struct OpenAIEmbedData {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddings {
    fn dimension(&self) -> usize { self.dimension }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let body = OpenAIEmbedRequest { model: &self.model, input: text };
        let res: OpenAIEmbedResponse = self.client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        res.data.into_iter().next()
            .map(|d| d.embedding)
            .ok_or_else(|| anyhow::anyhow!("Empty OpenAI embedding response"))
    }
}

// ---------------------------------------------------------------------------
// Voyage AI
// ---------------------------------------------------------------------------

pub struct VoyageEmbeddings {
    api_key: String,
    model: String,
    dimension: usize,
    client: Client,
}

impl VoyageEmbeddings {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let model = model.unwrap_or_else(|| "voyage-3".to_string());
        let dimension = 1024; // Voyage-3 default
        Self { api_key, model, dimension, client: Client::new() }
    }
}

#[derive(Serialize)]
struct VoyageEmbedRequest<'a> {
    model: &'a str,
    input: Vec<&'a str>,
}

#[derive(Deserialize)]
struct VoyageEmbedResponse {
    data: Vec<VoyageEmbedData>,
}

#[derive(Deserialize)]
struct VoyageEmbedData {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingProvider for VoyageEmbeddings {
    fn dimension(&self) -> usize { self.dimension }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let body = VoyageEmbedRequest { model: &self.model, input: vec![text] };
        let res: VoyageEmbedResponse = self.client
            .post("https://api.voyageai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        res.data.into_iter().next()
            .map(|d| d.embedding)
            .ok_or_else(|| anyhow::anyhow!("Empty Voyage embedding response"))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let body = VoyageEmbedRequest { model: &self.model, input: texts.to_vec() };
        let res: VoyageEmbedResponse = self.client
            .post("https://api.voyageai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res.data.into_iter().map(|d| d.embedding).collect())
    }
}

// ---------------------------------------------------------------------------
// Google Gemini
// ---------------------------------------------------------------------------

pub struct GeminiEmbeddings {
    api_key: String,
    model: String,
    dimension: usize,
    client: Client,
}

impl GeminiEmbeddings {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let model = model.unwrap_or_else(|| "text-embedding-004".to_string());
        Self { api_key, model, dimension: 768, client: Client::new() }
    }
}

#[derive(Serialize)]
struct GeminiEmbedRequest<'a> {
    content: GeminiContent<'a>,
}

#[derive(Serialize)]
struct GeminiContent<'a> {
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Serialize)]
struct GeminiPart<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct GeminiEmbedResponse {
    embedding: GeminiEmbedValues,
}

#[derive(Deserialize)]
struct GeminiEmbedValues {
    values: Vec<f32>,
}

#[async_trait]
impl EmbeddingProvider for GeminiEmbeddings {
    fn dimension(&self) -> usize { self.dimension }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
            self.model, self.api_key
        );
        let body = GeminiEmbedRequest {
            content: GeminiContent {
                parts: vec![GeminiPart { text }],
            },
        };
        let res: GeminiEmbedResponse = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res.embedding.values)
    }
}

// ---------------------------------------------------------------------------
// Factory from config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum EmbeddingProviderKind {
    OpenAI { api_key: String, model: Option<String> },
    Voyage { api_key: String, model: Option<String> },
    Gemini { api_key: String, model: Option<String> },
}

pub fn create_provider(kind: EmbeddingProviderKind) -> Box<dyn EmbeddingProvider> {
    match kind {
        EmbeddingProviderKind::OpenAI { api_key, model } => Box::new(OpenAIEmbeddings::new(api_key, model)),
        EmbeddingProviderKind::Voyage { api_key, model } => Box::new(VoyageEmbeddings::new(api_key, model)),
        EmbeddingProviderKind::Gemini { api_key, model } => Box::new(GeminiEmbeddings::new(api_key, model)),
    }
}
