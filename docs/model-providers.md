# Model Providers

ClawForge is multi-provider. The runtime speaks the OpenAI-compatible
`/chat/completions` protocol for almost every hosted provider, with a native
client for Anthropic and a native client for local Ollama. A single catalog
(`clawforge_planner::providers::catalog`) is the source of truth, and the
control plane governs whichever providers you approve (the Agent Registry stores
`model_provider` plus `model_name`, and the Security Gateway checks the action's
model against the agent's approved model).

## How registration works

On `clawforge-cli serve`, the runtime registers every provider whose API key is
present in the environment. Nothing is hard-coded to a single vendor: set the
keys you want and those providers become available. Ollama needs no key (set
`OLLAMA_URL` or use the local default), and OpenRouter is an aggregator that
reaches many models through one key.

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export DEEPSEEK_API_KEY="sk-..."
cargo run -p clawforge-cli -- serve
# logs: Registered additional model providers from environment ["anthropic", "deepseek"]
```

## Supported providers

### Global and Western

| id | Provider | Region | Wire | Env var |
|----|----------|--------|------|---------|
| `openrouter` | OpenRouter (aggregator) | Global | OpenAI-compatible | `OPENROUTER_API_KEY` |
| `openai` | OpenAI | US | OpenAI-compatible | `OPENAI_API_KEY` |
| `anthropic` | Anthropic | US | Native Messages API | `ANTHROPIC_API_KEY` |
| `google` | Google Gemini | US | OpenAI-compatible | `GEMINI_API_KEY` |
| `mistral` | Mistral AI | EU | OpenAI-compatible | `MISTRAL_API_KEY` |
| `xai` | xAI Grok | US | OpenAI-compatible | `XAI_API_KEY` |
| `groq` | Groq | US | OpenAI-compatible | `GROQ_API_KEY` |
| `together` | Together AI | US | OpenAI-compatible | `TOGETHER_API_KEY` |
| `fireworks` | Fireworks AI | US | OpenAI-compatible | `FIREWORKS_API_KEY` |
| `perplexity` | Perplexity | US | OpenAI-compatible | `PERPLEXITY_API_KEY` |
| `cohere` | Cohere | North America | OpenAI-compatible | `COHERE_API_KEY` |
| `ollama` | Ollama (local) | Local | Native | `OLLAMA_URL` |

### Chinese

| id | Provider | Region | Wire | Env var |
|----|----------|--------|------|---------|
| `deepseek` | DeepSeek | China | OpenAI-compatible | `DEEPSEEK_API_KEY` |
| `qwen` | Alibaba Qwen (DashScope) | China | OpenAI-compatible | `DASHSCOPE_API_KEY` |
| `zhipu` | Zhipu AI (GLM) | China | OpenAI-compatible | `ZHIPU_API_KEY` |
| `moonshot` | Moonshot AI (Kimi) | China | OpenAI-compatible | `MOONSHOT_API_KEY` |
| `baidu` | Baidu ERNIE (Qianfan) | China | OpenAI-compatible | `QIANFAN_API_KEY` |
| `minimax` | MiniMax | China | OpenAI-compatible | `MINIMAX_API_KEY` |
| `tencent` | Tencent Hunyuan | China | OpenAI-compatible | `HUNYUAN_API_KEY` |
| `yi` | 01.AI Yi (Lingyiwanwu) | China | OpenAI-compatible | `YI_API_KEY` |
| `stepfun` | StepFun | China | OpenAI-compatible | `STEPFUN_API_KEY` |
| `baichuan` | Baichuan AI | China | OpenAI-compatible | `BAICHUAN_API_KEY` |
| `iflytek` | iFlytek Spark | China | OpenAI-compatible | `SPARK_API_KEY` |
| `sensetime` | SenseTime SenseNova | China | OpenAI-compatible | `SENSENOVA_API_KEY` |

## Data residency and governance

Each catalog entry carries a `region` and a short `data_residency` note. For
government and regulated deployments this matters: routing regulated or PII data
to a provider hosted in another jurisdiction may breach data-protection rules.
The catalog flags every non-local provider with `needs_residency_review()`, and
the compliance pack (`ExportControl`, `PiiClassification`) is the place to record
and enforce the decision. Local Ollama keeps all data on the operator's hardware
and is the safest default for sensitive workloads.

## Adding a provider

Most providers are one catalog line because they are OpenAI-compatible. Add a
`ProviderInfo` to `CATALOG` in `backend/planner/src/providers/catalog.rs` with
the id, base URL, env var, and `Wire::OpenAiCompatible`. Providers with a
bespoke wire format get a dedicated client implementing the `LlmProvider` trait
(see `anthropic.rs` for the pattern) and a matching `Wire` variant.
