# Roadmap

The control plane is built; these are the next increments that turn it from a
strong foundation into a production platform.

## Near term

- **HTTP/gateway surface** — expose the control-plane stores over the existing
  `clawforge-gateway` so the React dashboard and external tools can drive them.
- **Gateway ↔ runtime wiring** — call `SecurityGateway::evaluate` from the live
  executor so checks run on real actions, and stream execution events into
  `ObservabilityStore`.
- **Control-plane UI** — registry, approval queue, fleet dashboard, MCP
  catalogue, marketplace, and compliance views (see `docs/screenshots/`).

## Mid term

- **Pluggable identity** — wire the SSO/Active Directory integrations into
  approver authentication and role-based access for governance actions.
- **Real digital signatures** — implement signing over `AuditEvidence`
  `content_hash` (the placeholder is already in the model).
- **Policy bundles** — versioned, exportable `SecurityPolicy` / compliance
  policy sets per environment (`local` / `staging` / `gov-prod`).
- **Scheduled health checks** — drive `McpRegistry::record_health` from the
  existing scheduler crate.

## Longer term

- **Multi-tenant** — first-class organisation/tenant isolation across all stores.
- **Additional compliance frameworks** — GDPR, NIST, ISO 27001 mappings
  alongside UAE PDPL.
- **Retention enforcement jobs** — act on `is_past_retention` to purge or
  archive data automatically (respecting investigation holds).
- **Marketplace ratings & reviews** — populate the `rating` field from real
  installer feedback.
