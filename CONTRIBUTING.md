# Contributing to ClawForge

Thanks for helping build the ClawForge control plane. This guide covers the
conventions specific to the `clawforge-controlplane` crate and the wider Rust
workspace.

## Repository layout

- `backend/*` - Rust workspace crates (the agent runtime).
- `backend/controlplane` - the **control plane** crate (registry, governance,
  observability, security gateway, MCP governance, marketplace, integrations,
  compliance).
- `frontend/` - React dashboard.
- `docs/` - architecture and per-domain documentation.

## Building & testing

```bash
# Build just the control plane
cargo build -p clawforge-controlplane

# Run its tests
cargo test -p clawforge-controlplane

# Build / test the whole workspace
cargo build
cargo test
```

Run tests **before** opening a pull request.

## Code conventions

- Edition 2021, idiomatic Rust, no `unsafe`.
- Domain types are plain `serde`-derived structs/enums; storage is `rusqlite`
  with both `open(path)` and `in_memory()` constructors (see `backend/security`
  for the reference pattern).
- Return the crate-wide `Result<T>` (`controlplane::error::Result`) from domain
  operations; reserve `anyhow` for binaries and glue.
- Reuse the shared vocabularies in `constants.rs` (`RiskLevel`,
  `DataAccessLevel`, `LifecycleStatus`) rather than redefining them.
- Emit structured logs through the `cp_info!` / `cp_warn!` / `cp_blocked!`
  macros so audit/observability tooling sees a consistent target.
- Add `#[cfg(test)]` unit tests next to the code they cover.

## Commit & PR workflow

- Keep commits **atomic** - one logical change per commit, with a clear message.
- Do not combine unrelated changes.
- Branch per unit of work; never commit directly to `main`. Open a pull request
  and merge through GitHub.

## Documentation

Every new domain module ships with a matching `docs/<domain>.md` that explains
the model, the operations, and how it fits the control-plane story. Keep docs
practical, not marketing.
