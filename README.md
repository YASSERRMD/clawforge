# 🦞 ClawForge — Personal AI Assistant Runtime

<p align="center">
  <strong>The experimental, blazingly fast Rust implementation of the OpenClaw standard.</strong>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/Rust-1.80+-orange.svg?style=for-the-badge" alt="Rust Version">
</p>

**ClawForge** is a _personal AI assistant runtime_ you run on your own devices, achieving identical topology to the [OpenClaw](https://openclaw.ai) reference implementation but built entirely in Rust for maximum performance, multi-threading, and memory safety.

It orchestrates autonomous agents over a central WebSocket control plane (Gateway) to interact across channels (WhatsApp, Telegram, Discord), perform actions via Tools (`browser.control`, Docker sandboxing), and manage memory.

## Install & Quick Start

Requires **Rust ≥ 1.80** and **Node ≥ 20** (for frontend UI).

```bash
# Clone and build the workspace
git clone https://github.com/YASSERRMD/clawforge.git
cd clawforge

# Export required configs
export OPENROUTER_API_KEY="sk-or-v1-..."

# Run the local-first Gateway Daemon
cargo run -p clawforge-cli -- serve --port 3000
```

Start the frontend dashboard in a separate terminal:
```bash
cd frontend
npm install
npm run dev
```

Alternatively, run everything via Docker Compose:
```bash
docker-compose up --build
```

## Highlights

- **Local-first Rust Gateway** — A robust Tokio-based WebSocket control plane for sessions, tools, and events.
- **Multi-channel integration** — Full deep adapters for Telegram, Discord, Slack, LINE, iMessage, and WhatsApp via the `clawforge-channels` crate.
- **Advanced Tooling & Plugins** — Out of the box CDP browser automation, sandboxed WASM Plugin System, and strict Docker sandboxing for untrusted bash/python executions.
- **Media Pipeline** — Dedicated crate for STT transcription hooks (Deepgram/Whisper), OCR visual text extraction, and document mining.
- **Tailscale Serve** — Natively bind the runtime to Tailscale for secure remote access.
- **Declarative Environments** — Fully reproducible developer environment via `flake.nix`.

## Everything We Built So Far

### Core Platform
- **`clawforge-core`**: The central vocabulary schemas (`AgentSpec`, `Message`, `Event`).
- **`clawforge-scheduler`**: Cron and Webhook evaluations that wake up agents.
- **`clawforge-planner`**: LLM provider integrations (OpenRouter, Ollama) with tool-call parsing and Reflection.
- **`clawforge-executor`**: Sandboxes and evaluates actions.
- **`clawforge-supervisor`**: SQLite persistence, policy checks, run state tracking (Active, Paused, AwaitingInput).
- **`clawforge-memory`**: Vector store implementations for RAG.

### Apps + Nodes (Stubs)
- Built stubs for macOS native app and mobile companion nodes.
- Engineered a `Canvas` React component in the Frontend for Agent-to-UI visual workspace control.

### Deep Adapters & Integrations
- **`clawforge-channels`**: Complete webhook and websocket adapters for Telegram, Discord, Slack, LINE, iMessage, and WhatsApp.
- **`clawforge-plugins`**: Sandboxed WASM plugin loader with granular permission scopes and internal event bus.
- **`clawforge-browser`**: Native CDP client for Playwright-style DOM observation, A11y queries, and synthetic interactions.
- **`clawforge-understanding`**: Media cracking pipelines for OCR, STT, PDF extraction, and ffmpeg native video thumbnails.
- **`clawforge-infra` & `clawforge-acp`**: Secure mDNS peer pairing, device identity management, Canvas hosting routines, and hierarchical Agent Control Protocol routing.

## Planned Missing OpenClaw Features (WIP Phases)

Compared to the upstream `openclaw` repository, ClawForge is expanding next into:

- **Skills Registry (Phase 13)**: Connecting to ClawHub for dynamic integrations (Notion, GitHub, 1Password).
- **Long-tail Channels (Phase 25)**: Matrix, Signal, and MS Teams adapters.
- **Full Control UI & Bots (Phase 15)**: Replacing stubs with full WebChat, Moltbot, and Clawdbot profiles.
- **Complete Native Apps (Phase 16)**: Bringing the full macOS menu bar app and iOS/Android capabilities (Voice Wake, Screen Recording).

## Security Model (Important)

OpenClaw connects to real messaging surfaces. Treat inbound DMs as **untrusted input**. 
In ClawForge, any `ShellTool` executions are strictly evaluated. For group/channel safety, you can enforce execution to route through isolated Docker containers instead of the host machine, matching the OpenClaw sandboxing specs.

## Configuration

ClawForge uses TOML bridging and environment variables. Tailscale access can be enabled quickly:
```bash
export CLAWFORGE_ENABLE_TAILSCALE=1
cargo run -p clawforge-cli -- serve
```

## Structure
```
WhatsApp / Telegram / Slack / Discord / WebChat
               │
               ▼
┌───────────────────────────────┐
│        ClawForge Gateway      │
│       (Rust control plane)    │
│     ws://127.0.0.1:3000       │
└──────────────┬────────────────┘
               │
               ├─ Planner (LLM RPC)
               ├─ Supervisor (SQLite)
               ├─ Tools (CDP/Docker)
               └─ Frontend Web Dashboard
``` 

## Community
AI/vibe-coded PRs welcome! We are consistently tracking the `openclaw` reference repository and mapping its TypeScript concepts to idiomatic Rust abstractions.
