# ClawForge

**ClawForge** is a blazing-fast, robust AI agent runtime built in Rust. It serves as an experimental implementation of the OpenClaw standard, designed to orchestrate autonomous agents with advanced tooling, memory, and perception capabilities.

## Architecture

ClawForge is designed as a multi-crate Rust workspace leveraging an event-driven, actor-like architecture connected via a central `ClawBus` (using Tokio channels).

### Core Components
- **`clawforge-core`**: Defines the shared vocabulary (`AgentSpec`, `Message`, `Event`, `Tool` trait).
- **`clawforge-scheduler`**: Evaluates cron expressions and webhook triggers to wake up agents.
- **`clawforge-planner`**: Interacts with LLM providers (OpenRouter, Ollama) and handles tool-call parsing and reasoning.
- **`clawforge-executor`**: Sandboxes and executes tools (Browser, Node, Shell/Docker) and returns observations.
- **`clawforge-supervisor`**: Tracks run state (HITL, Cancellation), handles safety/budget policies, and persists events to SQLite.
- **`clawforge-tools`**: The standard library of capabilities (CDP Browser, Node Invocation, File I/O).
- **`clawforge-memory`**: Vector store implementation for RAG and agent reflection.
- **`clawforge-channels`**: Integration adapters for Telegram, Discord, and WhatsApp.
- **`clawforge-media`**: Pipeline for handling audio STT and image perception.

### Frontend Dashboard
The `frontend` directory contains a Vite + React + TypeScript dashboard. It provides:
- Live streaming of agent events via Server Data (SSE/WebSockets).
- A unified "Agent-to-UI" (A2UI) Canvas.
- Controls for pausing, cancelling, and providing Human-in-the-loop (HITL) input to agents.

## Getting Started

### Prerequisites
- [Rust](https://rustup.rs/) (1.80+)
- [Node.js](https://nodejs.org/) (v20+)
- [Docker](https://www.docker.com/) (For sandboxing and deployment)
- SQLite

### Running Locally

1. **Start the Backend**
   ```bash
   # Export required API keys
   export OPENROUTER_API_KEY="sk-or-v1-..."
   
   # Run the server on port 3000
   cargo run -p clawforge-cli -- serve --port 3000
   ```

2. **Start the Frontend**
   ```bash
   cd frontend
   npm install
   npm run dev
   ```
   The dashboard will be available at `http://localhost:5173`.

### Docker Deployment
You can easily spin up both the backend and frontend using Docker Compose:
```bash
docker-compose up --build
```
This serves the API on `:3000` and the static frontend UI on `:8080`.

## Advanced Features

### Nix Environment
A `flake.nix` is provided for fully reproducible development environments. 
```bash
nix develop
```

### Tailscale Serve
The runtime can automatically bind to a Tailscale funnel for secure, remote UI access without exposing ports:
```bash
export CLAWFORGE_ENABLE_TAILSCALE=1
cargo run -p clawforge-cli -- serve
```

## Status
ClawForge has achieved feature parity with the OpenClaw Phase 1-12 specifications. Future work will focus on expanding the native Node ecosystem (macOS/Mobile) and stabilizing the Canvas data layer.
