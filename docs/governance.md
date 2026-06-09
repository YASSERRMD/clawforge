# Governance Engine

The Governance Engine (`clawforge_controlplane::governance`) is the approval
workflow that decides whether an agent, tool, MCP server, or model is allowed to
be used. It is the **ServiceNow** layer of the control plane: a human approval
gate with department ownership, risk awareness, and a full change history.

## What can be approved

`ApprovalKind` covers the four governed subjects:

| Kind | Example subject |
|------|-----------------|
| `agent` | a registered agent (by id) |
| `tool` | a tool name (e.g. `shell`) |
| `mcp` | an MCP server |
| `model` | a model name |

## Request lifecycle

```text
submit ──▶ Pending ──approve──▶ Approved
                   └─reject───▶ Rejected
```

A request is created in `Pending`. Exactly one terminal decision
(`Approved`/`Rejected`) may be applied; a second decision returns
`ControlPlaneError::Conflict`. Every decision **must** carry a non-empty reason,
which is stored for audit.

## Data captured

`ApprovalRequest` records `kind`, `subject_id`/`subject_name`, `requested_by`,
`department`, `risk_level`, `justification`, `status`, `decided_by`,
`decision_reason`, and timestamps. Each state change also appends an
`ApprovalEvent` (the change history): `submitted`, `approved`, or `rejected`,
with actor, reason, and time.

## API

```rust
use clawforge_controlplane::governance::{GovernanceEngine, NewApprovalRequest, ApprovalKind, ApprovalStatus};
use clawforge_controlplane::constants::RiskLevel;

let gov = GovernanceEngine::open("clawforge-controlplane.db")?;

let req = gov.submit(NewApprovalRequest {
    kind: ApprovalKind::Agent,
    subject_id: agent.id.clone(),
    subject_name: agent.name.clone(),
    requested_by: "platform-team".into(),
    department: "Licensing".into(),
    risk_level: RiskLevel::High,
    justification: "Needed for permit triage".into(),
})?;

gov.approve(&req.id, "ciso", "meets data-access policy")?;
// or: gov.reject(&req.id, "ciso", "insufficient justification")?;

// Listing & filtering
let all      = gov.list()?;
let pending  = gov.list_by_status(ApprovalStatus::Pending)?;
let mine     = gov.list_by_owner("platform-team")?;

// Audit trail
let history = gov.history(&req.id)?;
```

## How it ties together

The engine deliberately does not mutate the Agent Registry itself — it records
*decisions*. The intended flow is: register an agent (`Draft`), submit it for
approval, and on `Approved` move it through `PendingApproval → Active` in the
registry. This keeps the source-of-truth (registry) and the decision log
(governance) cleanly separated and independently auditable.
