# Installation Guide

## Prerequisites

- **Rust ≥ 1.80** (workspace edition 2021) — install via [rustup](https://rustup.rs).
- **Node ≥ 20** — only needed for the React dashboard in `frontend/`.
- A C toolchain (for the bundled SQLite used by `rusqlite`).

## Get the code

```bash
git clone https://github.com/YASSERRMD/clawforge.git
cd clawforge
```

## Configure

```bash
cp .env.example .env
# edit .env — at minimum set OPENROUTER_API_KEY for the runtime.
```

Control-plane settings (all optional, sensible local-first defaults):

| Variable | Default | Purpose |
|----------|---------|---------|
| `CLAWFORGE_CP_DB` | `clawforge-controlplane.db` | SQLite path for control-plane stores |
| `CLAWFORGE_CP_ENV` | `local` | Environment label (`local`/`staging`/`gov-prod`) |
| `CLAWFORGE_CP_ORG` | `ClawForge` | Owning organisation (in audit records) |
| `CLAWFORGE_CP_REQUIRE_APPROVAL` | `true` | Mandate human approval for high-risk actions |
| `CLAWFORGE_CP_BUDGET_LIMIT` | `100` | Default per-agent daily spend ceiling |

## Build & test the control plane

```bash
cargo build -p clawforge-controlplane
cargo test  -p clawforge-controlplane     # 80+ tests
```

## Run the runtime (optional)

```bash
export OPENROUTER_API_KEY="sk-or-v1-..."
cargo run -p clawforge-cli -- serve --port 3000
```

Dashboard, in a second terminal:

```bash
cd frontend && npm install && npm run dev
```

## Docker

```bash
docker-compose up --build
```

## Embedding the control plane

The control plane is a library. In another workspace crate:

```toml
# Cargo.toml
clawforge-controlplane = { path = "../controlplane" }
```

```rust
use clawforge_controlplane::registry::AgentRegistry;
let registry = AgentRegistry::open("clawforge-controlplane.db")?;
```

See the [developer guide](developer-guide.md) to go further.
