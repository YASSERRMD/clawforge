pub mod openrouter;
pub mod ollama;
pub mod mock;

use std::collections::HashMap;
use std::sync::Arc;

use clawforge_core::LlmProvider;

/// Registry of LLM providers, looked up by name.
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider by name.
    pub fn register(&mut self, name: impl Into<String>, provider: Arc<dyn LlmProvider>) {
        self.providers.insert(name.into(), provider);
    }

    /// Get providers matching the given names (in order).
    /// Unknown names are silently skipped.
    pub fn get_providers(&self, names: &[String]) -> Vec<Arc<dyn LlmProvider>> {
        names
            .iter()
            .filter_map(|name| self.providers.get(name).cloned())
            .collect()
    }

    /// Get all registered provider names.
    pub fn list(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use async_trait::async_trait;
    use clawforge_core::{LlmRequest, LlmResponse};

    struct MockProvider {
        name: String,
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }
        async fn complete(&self, _req: &LlmRequest) -> Result<LlmResponse> {
            Ok(LlmResponse {
                content: "mock response".into(),
                provider: self.name.clone(),
                model: "mock".into(),
                tokens_used: 10,
                latency_ms: 5,
            })
        }
    }

    #[test]
    fn test_registry_get_providers() {
        let mut registry = ProviderRegistry::new();
        registry.register(
            "mock1",
            Arc::new(MockProvider {
                name: "mock1".into(),
            }),
        );
        registry.register(
            "mock2",
            Arc::new(MockProvider {
                name: "mock2".into(),
            }),
        );

        let providers =
            registry.get_providers(&["mock1".into(), "mock2".into(), "missing".into()]);
        assert_eq!(providers.len(), 2);
    }
}
