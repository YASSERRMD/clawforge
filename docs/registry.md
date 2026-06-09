# Agent Registry

The Agent Registry (`clawforge_controlplane::registry`) is the single source of
truth for every agent an organisation runs. Governance, the security gateway,
and observability all key off the records stored here.

## What a record captures

| Field | Meaning |
|-------|---------|
| `id` | Stable UUID assigned on creation |
| `name`, `description` | Human-facing identity |
| `owner`, `department` | Accountability |
| `framework` | Runtime/framework the agent is built on |
| `model_provider`, `model_name` | Which model it uses |
| `tools_allowed` | Tools it may invoke |
| `mcp_servers_allowed` | MCP servers it may use |
| `data_access_level` | Highest data sensitivity (`none`…`restricted`) |
| `risk_level` | Assessed risk (`low`…`critical`) |
| `status` | Lifecycle state |
| `version` | Bumped on each metadata update |
| `created_at`, `updated_at` | Unix timestamps |

`data_access_level`, `risk_level`, and `status` come from the shared vocabularies
in `constants.rs`, so the whole control plane reasons about them consistently.

## Lifecycle

```text
Draft ──submit──▶ PendingApproval ──approve──▶ Active ──suspend──▶ Suspended
  │                     │                         │                    │
  │                     └──reject──▶ Draft         └────── Blocked ◀────┘
  ▼
Deactivated  (terminal; reachable from any non-deactivated state)
```

New agents start in `Draft`. They can only reach `Active` by passing through
`PendingApproval` — the registry rejects a direct `Draft → Active` jump so an
agent cannot become operational without governance approval. `Blocked` and
`Deactivated` are available as administrative overrides from any live state.
Transition rules live in `registry::lifecycle::can_transition`.

## API

```rust
use clawforge_controlplane::registry::{AgentRegistry, NewAgent, AgentUpdate};
use clawforge_controlplane::constants::{DataAccessLevel, RiskLevel, LifecycleStatus};

let reg = AgentRegistry::open("clawforge-controlplane.db")?;

// Create (validated; starts as Draft v1)
let agent = reg.create(NewAgent { /* … */ })?;

// Read
let one = reg.get(&agent.id)?;
let all = reg.list()?;

// Update metadata (bumps version, re-validates)
let updated = reg.update(&agent.id, AgentUpdate { risk_level: Some(RiskLevel::High), ..Default::default() })?;

// Lifecycle
reg.set_status(&agent.id, LifecycleStatus::PendingApproval)?;
reg.set_status(&agent.id, LifecycleStatus::Active)?;
reg.deactivate(&agent.id)?;
```

## Validation

`validate_new_agent` (on create) and `validate_record` (after an update patch)
enforce: required non-empty `name`/`owner`/`department`/`framework`/model fields,
bounded field lengths, and that `restricted` data access is never paired with
`low` risk. Invalid input returns `ControlPlaneError::Validation`.

## Storage

SQLite via `rusqlite`, following the workspace pattern: `AgentRegistry::open(path)`
for persistence and `AgentRegistry::in_memory()` for tests. List/enum fields are
stored as JSON text so the schema is stable across vocabulary changes.

## Seed data

`registry::seed::seed(&reg)` inserts three realistic municipality/enterprise
example agents (permit intake, citizen records lookup, IT ops runbook) for demos.
