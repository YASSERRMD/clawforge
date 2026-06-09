# Demo

A runnable, end-to-end walkthrough of the control plane lives at
[`backend/controlplane/examples/demo.rs`](../backend/controlplane/examples/demo.rs).

## Run it

```bash
cargo run -p clawforge-controlplane --example demo
```

## What it does

Everything runs in-memory, exercising every module in one agent's journey:

1. **Marketplace** - seeds verified listings and installs one into the Registry.
2. **MCP Governance** - registers and approves the `records-mcp` server.
3. **Governance** - submits and approves the agent, moving it `Draft →
   PendingApproval → Active`.
4. **Security Gateway** - evaluates a real action (tool + MCP + data access +
   budget) and prints an allow/deny verdict with a risk score.
5. **Observability** - records the execution and prints the task success rate.
6. **Compliance** - classifies the agent (UAE PDPL, PII, retention, approval
   chain) and prints a compliance report with findings.

## Expected output

```
== ClawForge Control Plane demo ==

1. Marketplace listing 'Permit Intake Assistant' trusted=true
   installed agent Permit Bot A (status Draft)
2. MCP server 'records-mcp' approved
3. Governance approved; agent now Active
4. Gateway decision: ALLOW (risk: medium, score: 26)
5. Observability: 1 task(s), success rate 100%
6. Compliance: framework UAE-PDPL, compliant=false (2 finding(s))
     - regulated PII handled but no audit evidence collected
     - approval chain is incomplete

== demo complete ==
```

The compliance findings are intentional: they show the report correctly
flagging an agent that handles PII without collected evidence or a completed
approval chain - exactly the kind of gap the control plane surfaces.
