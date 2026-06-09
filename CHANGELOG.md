# Changelog

## Control Plane - initial build

Adds the `clawforge-controlplane` crate: the enterprise/government control plane
for governing, securing, observing, auditing, and operating AI agents and MCP
servers. Built in ten phases, each merged via its own pull request.

### Added

- **Foundation** - `ControlPlaneConfig`, shared vocabularies (`RiskLevel`,
  `DataAccessLevel`, `LifecycleStatus`), unified `ControlPlaneError`, structured
  logging macros.
- **Agent Registry** - CRUD, validation, and an enforced lifecycle state
  machine (no `Draft → Active` without approval).
- **Governance Engine** - approval workflow with human gate, mandatory decision
  reasons, and append-only change history.
- **Observability** - append-only execution events with on-demand per-agent and
  fleet-wide metric summaries.
- **Security Gateway** - pre-execution checks (agent state, tool, MCP, model,
  data access, capabilities, budget, human approval), risk scoring, and a
  blocked-execution log.
- **MCP Governance** - registry with approval, health, and usage tracking.
- **Agent Marketplace** - verified, reusable templates with verification and
  compliance badges; install into the registry.
- **Enterprise Integrations** - governed connectors (DBs, SSO, GIS, ITSM, …)
  with credential *references* (never secrets) and risk classification.
- **Government Compliance Pack** - PII classification, retention, approval
  chains, audit evidence, investigation holds, and compliance reporting
  (UAE PDPL-aware).
- **Docs & demo** - per-domain docs, Mermaid diagrams, government and enterprise
  use cases, installation and developer guides, roadmap, limitations, security
  disclaimer, and a runnable end-to-end example.

### Tests

`cargo test -p clawforge-controlplane` - **82 passing, 0 failing**.

### Notes

The control plane is a self-contained library crate. It is not yet wired into
the live runtime or exposed over HTTP; see [docs/roadmap.md](docs/roadmap.md)
and [docs/limitations.md](docs/limitations.md).
