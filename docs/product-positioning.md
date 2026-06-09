# ClawForge — Product Positioning

## One-line

**ClawForge is the enterprise & government control plane for AI agents:
Kubernetes + ServiceNow + Splunk for AI Agents.**

## What ClawForge is *not*

ClawForge is **not** another agent framework, SDK, or model wrapper. It does not
compete with the layer that *builds* or *runs* an agent. Instead it sits **above**
those agents as a control plane — the layer an organisation uses to decide *which*
agents may exist, *what* they are allowed to do, *who* approved them, *what* they
actually did, and *whether* that was compliant.

## The category

Modern organisations are about to run dozens or hundreds of AI agents and MCP
servers across departments. Today that happens with no central registry, no
approval workflow, no per-action security checks, no unified audit trail, and no
compliance posture. ClawForge fills that gap.

| Layer | Analogy | ClawForge capability |
|-------|---------|----------------------|
| Orchestration & lifecycle | **Kubernetes** | Agent & MCP registries, lifecycle status, deactivation |
| Governance & change control | **ServiceNow** | Approval workflows, department ownership, change history |
| Observability & audit | **Splunk** | Execution metrics, risk events, immutable audit trail |

## Comparison with adjacent products

| Product | Primary verb | Scope |
|---------|--------------|-------|
| **Hermes** | Learns | Continual learning / knowledge |
| **OpenClaw** | Executes | Agent runtime that performs actions |
| **Paperclip** | Operates a company | AI company operating system |
| **ClawForge** | **Governs & controls** | **Enterprise/government agent control plane** |

ClawForge can govern agents built on *any* of the above. The underlying ClawForge
runtime happens to be an OpenClaw-compatible Rust implementation, but the control
plane is framework-agnostic by design.

## Target users

- Government entities and municipalities
- Enterprise IT teams
- AI platform teams
- Security and compliance teams
- Solution architects
- Agent developers and MCP server builders

## Core capabilities (build phases)

1. **Agent Registry** — single source of truth for every agent.
2. **Governance Engine** — approval workflows with human gates and change history.
3. **Observability** — task, cost, latency, failure, and risk metrics.
4. **Security Gateway** — pre-execution checks on every agent action.
5. **MCP Governance** — registry and approval for MCP servers and their tools.
6. **Agent Marketplace** — verified, reusable internal agent templates.
7. **Enterprise Integrations** — governed connectors (databases, SSO, GIS, ITSM).
8. **Government Compliance Pack** — PII classification, retention, audit evidence,
   approval chains, and compliance reporting (UAE PDPL-aware).

## Final product statement

> ClawForge is an enterprise-grade AI agent control plane for governing, securing,
> monitoring, auditing, and operating AI agents and MCP servers across government
> and enterprise environments.
