/// Model catalog — provides a dynamic listing of available LLM models
/// across all configured providers.
///
/// Mirrors `src/agents/model-catalog.ts`.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A model entry in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub context_window: usize,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub is_default: bool,
}

/// The full model catalog organized by provider.
#[derive(Debug, Default)]
pub struct ModelCatalog {
    models: HashMap<String, ModelEntry>,
}

impl ModelCatalog {
    pub fn new() -> Self {
        let mut catalog = Self::default();
        // Built-in well-known models (kept in sync with planner defaults)
        catalog.register_defaults();
        catalog
    }

    fn register_defaults(&mut self) {
        let defaults = vec![
            ModelEntry {
                id: "gpt-4o".into(),
                provider: "openai".into(),
                display_name: "GPT-4o".into(),
                context_window: 128_000,
                supports_vision: true,
                supports_tools: true,
                is_default: true,
            },
            ModelEntry {
                id: "gpt-4o-mini".into(),
                provider: "openai".into(),
                display_name: "GPT-4o Mini".into(),
                context_window: 128_000,
                supports_vision: true,
                supports_tools: true,
                is_default: false,
            },
            ModelEntry {
                id: "claude-opus-4-5".into(),
                provider: "anthropic".into(),
                display_name: "Claude Opus 4.5".into(),
                context_window: 200_000,
                supports_vision: true,
                supports_tools: true,
                is_default: false,
            },
            ModelEntry {
                id: "claude-sonnet-4-5".into(),
                provider: "anthropic".into(),
                display_name: "Claude Sonnet 4.5".into(),
                context_window: 200_000,
                supports_vision: true,
                supports_tools: true,
                is_default: false,
            },
            ModelEntry {
                id: "gemini-2.0-flash".into(),
                provider: "google".into(),
                display_name: "Gemini 2.0 Flash".into(),
                context_window: 1_048_576,
                supports_vision: true,
                supports_tools: true,
                is_default: false,
            },
            ModelEntry {
                id: "gemini-2.5-pro".into(),
                provider: "google".into(),
                display_name: "Gemini 2.5 Pro".into(),
                context_window: 2_097_152,
                supports_vision: true,
                supports_tools: true,
                is_default: false,
            },
            ModelEntry {
                id: "llama3.3:70b".into(),
                provider: "ollama".into(),
                display_name: "Llama 3.3 70B (Local)".into(),
                context_window: 128_000,
                supports_vision: false,
                supports_tools: true,
                is_default: false,
            },
        ];
        for m in defaults {
            self.models.insert(m.id.clone(), m);
        }
    }

    /// Add or update a model in the catalog.
    pub fn register(&mut self, entry: ModelEntry) {
        self.models.insert(entry.id.clone(), entry);
    }

    /// List all models, optionally filtering by provider.
    pub fn list(&self, provider: Option<&str>) -> Vec<&ModelEntry> {
        let mut entries: Vec<&ModelEntry> = self
            .models
            .values()
            .filter(|m| provider.map_or(true, |p| m.provider == p))
            .collect();
        entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        entries
    }

    /// Get the default model (first one with `is_default: true`).
    pub fn default_model(&self) -> Option<&ModelEntry> {
        self.models.values().find(|m| m.is_default)
    }

    /// Look up a model by its ID.
    pub fn get(&self, id: &str) -> Option<&ModelEntry> {
        self.models.get(id)
    }
}
