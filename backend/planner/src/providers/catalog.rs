//! Catalog of supported model providers and env-driven registration.
//!
//! ClawForge speaks OpenAI-compatible chat-completions for almost every hosted
//! provider, with a native client for Anthropic and Ollama. This catalog is the
//! authoritative list the runtime uses to wire providers up from environment
//! variables, and the governance layer uses to reason about model origin and
//! data residency.

use std::sync::Arc;

use tracing::info;

use super::anthropic::AnthropicProvider;
use super::openai_compatible::OpenAiCompatibleProvider;
use super::ProviderRegistry;

/// Wire protocol a provider speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    /// OpenAI-style `/chat/completions`.
    OpenAiCompatible,
    /// Anthropic Messages API.
    Anthropic,
    /// Native Ollama API (local).
    Ollama,
}

/// Hosting origin, relevant to data-residency review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    UnitedStates,
    Europe,
    China,
    /// Multi-region aggregator/router.
    Global,
    /// Runs on the operator's own hardware.
    Local,
}

/// Static metadata about a model provider.
#[derive(Debug, Clone, Copy)]
pub struct ProviderInfo {
    /// Stable id used by the registry and agent records.
    pub id: &'static str,
    /// Human-friendly name.
    pub display_name: &'static str,
    /// Hosting origin.
    pub region: Region,
    /// Base URL for the API.
    pub base_url: &'static str,
    /// Environment variable holding the credential (a URL for Ollama).
    pub env_var: &'static str,
    /// Wire protocol.
    pub wire: Wire,
    /// A few representative model names.
    pub example_models: &'static [&'static str],
    /// Short data-residency note for governance.
    pub data_residency: &'static str,
}

impl ProviderInfo {
    /// Whether this provider warrants a data-residency review before approval
    /// (anything not hosted locally or in the operator's own jurisdiction).
    pub fn needs_residency_review(&self) -> bool {
        !matches!(self.region, Region::Local)
    }
}

/// The full catalog of supported providers.
pub const CATALOG: &[ProviderInfo] = &[
    // ---- Global / Western ----
    ProviderInfo { id: "openrouter", display_name: "OpenRouter", region: Region::Global,
        base_url: "https://openrouter.ai/api/v1", env_var: "OPENROUTER_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["anthropic/claude-opus-4-8", "openai/gpt-4o", "google/gemini-2.5-pro"],
        data_residency: "Aggregator; routes to many providers and regions" },
    ProviderInfo { id: "openai", display_name: "OpenAI", region: Region::UnitedStates,
        base_url: "https://api.openai.com/v1", env_var: "OPENAI_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["gpt-4o", "gpt-4.1", "o3"], data_residency: "US-hosted" },
    ProviderInfo { id: "anthropic", display_name: "Anthropic", region: Region::UnitedStates,
        base_url: "https://api.anthropic.com", env_var: "ANTHROPIC_API_KEY", wire: Wire::Anthropic,
        example_models: &["claude-opus-4-8", "claude-sonnet-4-6", "claude-haiku-4"], data_residency: "US-hosted" },
    ProviderInfo { id: "google", display_name: "Google Gemini", region: Region::UnitedStates,
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai", env_var: "GEMINI_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["gemini-2.5-pro", "gemini-2.5-flash"], data_residency: "US-hosted; OpenAI-compatible endpoint" },
    ProviderInfo { id: "mistral", display_name: "Mistral AI", region: Region::Europe,
        base_url: "https://api.mistral.ai/v1", env_var: "MISTRAL_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["mistral-large-latest", "codestral-latest"], data_residency: "EU-hosted (France)" },
    ProviderInfo { id: "xai", display_name: "xAI Grok", region: Region::UnitedStates,
        base_url: "https://api.x.ai/v1", env_var: "XAI_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["grok-4", "grok-3"], data_residency: "US-hosted" },
    ProviderInfo { id: "groq", display_name: "Groq", region: Region::UnitedStates,
        base_url: "https://api.groq.com/openai/v1", env_var: "GROQ_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["llama-3.3-70b-versatile", "moonshotai/kimi-k2-instruct"], data_residency: "US-hosted" },
    ProviderInfo { id: "together", display_name: "Together AI", region: Region::UnitedStates,
        base_url: "https://api.together.xyz/v1", env_var: "TOGETHER_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["meta-llama/Llama-3.3-70B-Instruct-Turbo", "Qwen/Qwen2.5-72B-Instruct"], data_residency: "US-hosted" },
    ProviderInfo { id: "fireworks", display_name: "Fireworks AI", region: Region::UnitedStates,
        base_url: "https://api.fireworks.ai/inference/v1", env_var: "FIREWORKS_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["accounts/fireworks/models/deepseek-v3", "accounts/fireworks/models/qwen2p5-72b-instruct"], data_residency: "US-hosted" },
    ProviderInfo { id: "perplexity", display_name: "Perplexity", region: Region::UnitedStates,
        base_url: "https://api.perplexity.ai", env_var: "PERPLEXITY_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["sonar", "sonar-pro"], data_residency: "US-hosted" },
    ProviderInfo { id: "cohere", display_name: "Cohere", region: Region::UnitedStates,
        base_url: "https://api.cohere.ai/compatibility/v1", env_var: "COHERE_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["command-r-plus", "command-r"], data_residency: "North America-hosted" },
    ProviderInfo { id: "ollama", display_name: "Ollama (local)", region: Region::Local,
        base_url: "http://localhost:11434", env_var: "OLLAMA_URL", wire: Wire::Ollama,
        example_models: &["llama3.1", "qwen2.5", "deepseek-r1"], data_residency: "Runs on the operator's own hardware; no data leaves the host" },
    // ---- China ----
    ProviderInfo { id: "deepseek", display_name: "DeepSeek", region: Region::China,
        base_url: "https://api.deepseek.com/v1", env_var: "DEEPSEEK_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["deepseek-chat", "deepseek-reasoner"], data_residency: "China-hosted; review residency for regulated data" },
    ProviderInfo { id: "qwen", display_name: "Alibaba Qwen (DashScope)", region: Region::China,
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1", env_var: "DASHSCOPE_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["qwen-max", "qwen-plus", "qwen2.5-72b-instruct"], data_residency: "China-hosted; an international endpoint is also available" },
    ProviderInfo { id: "zhipu", display_name: "Zhipu AI (GLM, BigModel)", region: Region::China,
        base_url: "https://open.bigmodel.cn/api/paas/v4", env_var: "ZHIPU_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["glm-4.6", "glm-4-plus"], data_residency: "China-hosted (mainland BigModel endpoint)" },
    ProviderInfo { id: "zai", display_name: "Z.AI (Zhipu international, GLM)", region: Region::China,
        base_url: "https://api.z.ai/api/paas/v4", env_var: "ZAI_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["glm-4.6", "glm-4.5", "glm-4.5-air"],
        data_residency: "Zhipu's international GLM endpoint; coding-plan keys use /api/coding/paas/v4" },
    ProviderInfo { id: "moonshot", display_name: "Moonshot AI (Kimi)", region: Region::China,
        base_url: "https://api.moonshot.cn/v1", env_var: "MOONSHOT_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["moonshot-v1-128k", "kimi-k2-0905-preview"], data_residency: "China-hosted; a global endpoint is also available" },
    ProviderInfo { id: "baidu", display_name: "Baidu ERNIE (Qianfan)", region: Region::China,
        base_url: "https://qianfan.baidubce.com/v2", env_var: "QIANFAN_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["ernie-4.5-turbo-128k", "ernie-4.5-8k"], data_residency: "China-hosted" },
    ProviderInfo { id: "minimax", display_name: "MiniMax", region: Region::China,
        base_url: "https://api.minimax.chat/v1", env_var: "MINIMAX_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["minimax-text-01", "abab6.5s-chat"], data_residency: "China-hosted; an international endpoint is also available" },
    ProviderInfo { id: "tencent", display_name: "Tencent Hunyuan", region: Region::China,
        base_url: "https://api.hunyuan.cloud.tencent.com/v1", env_var: "HUNYUAN_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["hunyuan-turbo", "hunyuan-large"], data_residency: "China-hosted" },
    ProviderInfo { id: "yi", display_name: "01.AI Yi (Lingyiwanwu)", region: Region::China,
        base_url: "https://api.lingyiwanwu.com/v1", env_var: "YI_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["yi-large", "yi-lightning"], data_residency: "China-hosted" },
    ProviderInfo { id: "stepfun", display_name: "StepFun", region: Region::China,
        base_url: "https://api.stepfun.com/v1", env_var: "STEPFUN_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["step-2-16k", "step-1v-8k"], data_residency: "China-hosted" },
    ProviderInfo { id: "baichuan", display_name: "Baichuan AI", region: Region::China,
        base_url: "https://api.baichuan-ai.com/v1", env_var: "BAICHUAN_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["Baichuan4-Turbo", "Baichuan4"], data_residency: "China-hosted" },
    ProviderInfo { id: "iflytek", display_name: "iFlytek Spark", region: Region::China,
        base_url: "https://spark-api-open.xf-yun.com/v1", env_var: "SPARK_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["generalv3.5", "4.0Ultra"], data_residency: "China-hosted" },
    ProviderInfo { id: "sensetime", display_name: "SenseTime SenseNova", region: Region::China,
        base_url: "https://api.sensenova.cn/compatible-mode/v1", env_var: "SENSENOVA_API_KEY", wire: Wire::OpenAiCompatible,
        example_models: &["SenseChat-5"], data_residency: "China-hosted" },
];

/// Look up a provider by id.
pub fn get(id: &str) -> Option<&'static ProviderInfo> {
    CATALOG.iter().find(|p| p.id == id)
}

/// All providers hosted in a given region.
pub fn by_region(region: Region) -> Vec<&'static ProviderInfo> {
    CATALOG.iter().filter(|p| p.region == region).collect()
}

/// All China-hosted providers.
pub fn chinese() -> Vec<&'static ProviderInfo> {
    by_region(Region::China)
}

/// Register every catalog provider whose credential environment variable is set.
///
/// `openrouter` and `ollama` are skipped here because the CLI wires them from
/// its own `Config` (so they are never double-registered). Returns the ids that
/// were registered.
pub fn register_from_env(registry: &mut ProviderRegistry) -> Vec<String> {
    let mut registered = Vec::new();
    for p in CATALOG {
        if p.id == "openrouter" || p.id == "ollama" {
            continue;
        }
        let key = match std::env::var(p.env_var) {
            Ok(v) if !v.trim().is_empty() => v,
            _ => continue,
        };
        match p.wire {
            Wire::Anthropic => {
                registry.register(p.id, Arc::new(AnthropicProvider::new(key).with_base_url(p.base_url)));
            }
            Wire::OpenAiCompatible => {
                registry.register(p.id, Arc::new(OpenAiCompatibleProvider::new(p.id, p.base_url, key)));
            }
            Wire::Ollama => continue,
        }
        info!(provider = p.id, region = ?p.region, "Registered model provider from env");
        registered.push(p.id.to_string());
    }
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<&str> = CATALOG.iter().map(|p| p.id).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate provider ids in catalog");
    }

    #[test]
    fn covers_major_chinese_providers() {
        for id in ["deepseek", "qwen", "zhipu", "zai", "moonshot", "baidu", "minimax", "tencent", "yi"] {
            assert!(get(id).is_some(), "missing Chinese provider {id}");
        }
        assert!(chinese().len() >= 10, "expected at least 10 China-hosted providers");
    }

    #[test]
    fn zai_is_openai_compatible_with_z_ai_base() {
        let z = get("zai").unwrap();
        assert_eq!(z.wire, Wire::OpenAiCompatible);
        assert!(z.base_url.contains("api.z.ai"));
        assert_eq!(z.env_var, "ZAI_API_KEY");
    }

    #[test]
    fn anthropic_uses_native_wire() {
        assert_eq!(get("anthropic").unwrap().wire, Wire::Anthropic);
        assert_eq!(get("deepseek").unwrap().wire, Wire::OpenAiCompatible);
    }

    #[test]
    fn local_provider_skips_residency_review() {
        assert!(!get("ollama").unwrap().needs_residency_review());
        assert!(get("deepseek").unwrap().needs_residency_review());
    }

    #[test]
    fn register_from_env_picks_up_set_keys() {
        // Use a provider unlikely to be set in the ambient environment.
        std::env::set_var("STEPFUN_API_KEY", "sk-test");
        let mut reg = ProviderRegistry::new();
        let ids = register_from_env(&mut reg);
        std::env::remove_var("STEPFUN_API_KEY");
        assert!(ids.contains(&"stepfun".to_string()));
        assert!(reg.list().contains(&"stepfun".to_string()));
        // openrouter/ollama are intentionally not registered here.
        assert!(!ids.contains(&"ollama".to_string()));
    }
}
