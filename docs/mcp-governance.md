# MCP Governance

The MCP registry (`clawforge_controlplane::mcp`) governs the Model Context
Protocol servers an organisation exposes to its agents — the same discipline the
agent registry applies to agents. The Security Gateway consults agent
allow-lists; this registry is the source of truth for *which MCP servers exist,
who owns them, and whether they are approved*.

## What a server record captures

| Field | Meaning |
|-------|---------|
| `id` | Stable UUID |
| `name`, `description`, `owner` | Identity & accountability |
| `endpoint` | URL or command, per transport |
| `transport` | `stdio` / `http` / `sse` / `websocket` |
| `tools_exposed` | `McpTool`s with their required permission scopes |
| `permissions_required` | Server-wide permission scopes |
| `risk_level` | `low`…`critical` |
| `status` | `pending_approval` / `active` / `blocked` / … |
| `health` + `last_health_check` | Liveness from the latest check |
| `usage_count`, `cost_estimate` | Accumulated usage |

## Lifecycle

A server is registered in `pending_approval` and is **not usable** until
`approve`d (status `active`). `block` moves it to `blocked`. A server that
`requires_governance_review()` — high/critical risk *or* exposing sensitive
tools (network/fs/write/exec/pii) — should not be approved without scrutiny.

## API

```rust
use clawforge_controlplane::mcp::{McpRegistry, NewMcpServer, McpTool, TransportType, HealthStatus};
use clawforge_controlplane::constants::{RiskLevel, LifecycleStatus};

let reg = McpRegistry::open("clawforge-controlplane.db")?;

let server = reg.register(NewMcpServer {
    name: "records-mcp".into(),
    description: "Resident records access".into(),
    owner: "data-platform".into(),
    endpoint: "https://mcp.internal/records".into(),
    transport: TransportType::Http,
    tools_exposed: vec![McpTool { name: "lookup".into(), description: "read".into(), permissions: vec!["read".into()] }],
    permissions_required: vec!["read".into()],
    risk_level: RiskLevel::High,
})?;

reg.approve(&server.id)?;                       // make usable
reg.record_usage(&server.id, 0.02)?;            // track calls + cost
reg.record_health(&server.id, HealthStatus::Healthy)?;

let pending = reg.list_by_status(LifecycleStatus::PendingApproval)?;
// reg.block(&server.id)? to take it out of service
```

## Audit & observability

`record_usage` and `record_health` keep per-server counters current. Blocked
calls and risk events surface through the Security Gateway and Observability
layers, so MCP activity is auditable end to end.
