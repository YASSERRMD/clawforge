# Developer Guide

This guide orients contributors working on the `clawforge-controlplane` crate.

## Crate layout

```
backend/controlplane/src/
├── lib.rs            # crate docs + module wiring
├── config.rs         # ControlPlaneConfig (CLAWFORGE_CP_*)
├── constants.rs      # RiskLevel, DataAccessLevel, LifecycleStatus
├── error.rs          # ControlPlaneError + Result
├── logging.rs        # cp_info! / cp_warn! / cp_blocked!
├── registry/         # Agent Registry            (Phase 2)
├── governance/       # Governance Engine          (Phase 3)
├── observability/    # Execution events & metrics (Phase 4)
├── gateway/          # Security Gateway           (Phase 5)
├── mcp/              # MCP Governance             (Phase 6)
├── marketplace/      # Agent Marketplace          (Phase 7)
├── integrations/     # Enterprise Integrations    (Phase 8)
└── compliance/       # Government Compliance Pack (Phase 9)
```

Each domain follows the same shape: a `model.rs` of `serde` types, a `store.rs`
SQLite store with `open(path)` + `in_memory()`, and `#[cfg(test)]` unit tests
beside the code.

## Conventions

- **One vocabulary.** Reuse `constants::{RiskLevel, DataAccessLevel,
  LifecycleStatus}` rather than redefining per module.
- **Errors.** Return the crate-wide `Result<T>` (`error::Result`). `From` impls
  convert `rusqlite` and `serde_json` errors automatically.
- **Storage.** JSON-encode list/enum columns so schemas stay stable across
  vocabulary changes; provide both `open` and `in_memory` constructors.
- **Logging.** Emit via `cp_info!` / `cp_warn!` / `cp_blocked!` so audit and
  observability tooling sees a consistent `clawforge::controlplane` target.
- **No `unsafe`, edition 2021, idiomatic Rust.**

## Adding a new domain module

1. Create `src/<domain>/{mod.rs, model.rs, store.rs}`.
2. Wire `pub mod <domain>;` into `lib.rs` and re-export the key types.
3. Add `#[cfg(test)]` tests and a `docs/<domain>.md`.
4. Keep each commit atomic and green (`cargo test -p clawforge-controlplane`).

## Build & test

```bash
cargo build -p clawforge-controlplane
cargo test  -p clawforge-controlplane
cargo fmt
cargo clippy -p clawforge-controlplane   # optional lint pass
```

## How the modules compose

The Marketplace installs into the Registry; Governance approves Registry agents;
the Security Gateway reads the Registry (and is consulted on MCP/integration
use); Observability records what happened; Compliance reports over all of it.
See [diagrams.md](diagrams.md) and [use-cases.md](use-cases.md).
