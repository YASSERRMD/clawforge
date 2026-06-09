# ClawForge Diagrams

Mermaid diagrams for the control plane. (GitHub renders these inline.)

## Control-plane architecture

How the control-plane modules relate to each other and to the agent runtime.

```mermaid
flowchart TB
    subgraph CP["ClawForge Control Plane"]
        REG[Agent Registry]
        GOV[Governance Engine]
        GW[Security Gateway]
        OBS[Observability]
        MCP[MCP Governance]
        MKT[Marketplace]
        INT[Enterprise Integrations]
        COMP[Compliance Pack]
    end

    MKT -- install --> REG
    GOV -- approves --> REG
    GW  -- reads --> REG
    GW  -- denials --> OBS
    MCP -- governs --> GW
    INT -- governs --> GW
    REG -- subjects --> COMP
    OBS -- metrics --> COMP
    GOV -- decisions --> COMP

    CP -. authorises actions .-> RT[Agent Runtime]
    RT -. execution events .-> OBS
```

## Agent lifecycle

The registry state machine. An agent can only become operational by passing
through approval — a direct `Draft → Active` jump is rejected.

```mermaid
stateDiagram-v2
    [*] --> Draft
    Draft --> PendingApproval: submit
    PendingApproval --> Active: approve
    PendingApproval --> Draft: reject
    Active --> Suspended: suspend
    Suspended --> Active: resume
    Active --> Blocked: block
    Blocked --> Active: unblock
    Draft --> Deactivated: retire
    Active --> Deactivated: retire
    Suspended --> Deactivated: retire
    Blocked --> Deactivated: retire
    Deactivated --> [*]
```

## Governance approval workflow

```mermaid
sequenceDiagram
    participant Dev as Requester
    participant GOV as Governance Engine
    participant App as Approver
    participant REG as Agent Registry

    Dev->>GOV: submit(kind, subject, justification)
    GOV-->>GOV: record "submitted" (Pending)
    App->>GOV: approve / reject (reason required)
    GOV-->>GOV: record decision + history
    alt approved
        GOV-->>REG: move agent → Active
    else rejected
        GOV-->>Dev: rejected with reason
    end
```
