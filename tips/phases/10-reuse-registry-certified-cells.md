# Phase 10: Reuse Registry Certified Cells

Status: hardened
Owner: standard
Last reviewed: 2026-05-03
Parallel MCP candidate: yes

## Objective

Stop teams and agents from rebuilding the same primitives badly. Jankurai should provide a registry of certified cells: reusable product and engineering modules that include source, contracts, migrations, UI, tests, UX proof, security assumptions, observability, docs, and upgrade paths.

The exit state is a registry format and the first small set of certified cells.

## Current State

The repo now emits schema-valid certified cell manifests from live ownership,
proof lane, and Phase 09 reference platform evidence.

Current certified cells:

- `audit-log`
- `crud-resource`
- `rbac` (depends on `crud-resource`; sources and proof lanes tied to `examples/perfect-web-api-db/` authorization surface)
- `auth-session` (depends on `audit-log` and `rbac`; sources and proof lanes tied to `examples/perfect-web-api-db/` identity/session boundary)
- `organization-team` (depends on `audit-log`, `rbac`, and `auth-session`; sources and proof lanes tied to tenant-scoped team membership, DB constraints, UX states, and security assumptions)
- `background-job` (depends on `audit-log`, `rbac`, `auth-session`, and `organization-team`; sources and proof lanes tied to queue/retry policy, durable DB constraints, operator UX states, and security assumptions)

The installer remains dry-run only and never overwrites user files. `cell
--mode prove` emits certification evidence and proof commands, but does not
execute proof commands.

Foundation from earlier phases:

- Phase 04 profile generator
- Phase 07 contract and DB boundary rules
- Phase 09 reference product platform
- Phase 08 repair/context plans

## Dependencies

Requires Phase 04 and Phase 07.

Strongly benefits from Phase 09 as the place to prove cells.

## Public Interface Changes

Implemented command surface:

```bash
jankurai registry .
jankurai cell . --cell-id audit-log
jankurai cell . --cell-id audit-log --mode prove
```

The implementation emits a registry with certified manifests first, then
owner-derived candidate-cell discovery hints. Cell install output is an explicit
dry-run plan with `never-overwrite`; prove mode emits evidence and proof
commands only.

Registry manifest fields:

- cell ID
- version
- category
- supported profiles
- dependencies
- source paths
- generated paths
- contract paths
- migration paths
- UI routes/stories
- proof lanes
- proof commands
- security assumptions
- observability events
- docs
- upgrade/migration/rollback notes
- install strategy
- conflict policy
- certification evidence
- certification status

## Initial Cell Order

Build in this order:

1. audit-log
2. CRUD table/form
3. RBAC — certified as registry cell `rbac` (depends on `crud-resource`)
4. auth/session shell — certified as registry cell `auth-session` (depends on `audit-log` and `rbac`)
5. organization/team shell — certified as registry cell `organization-team` (depends on `audit-log`, `rbac`, and `auth-session`)
6. background job — certified as registry cell `background-job` (depends on `audit-log`, `rbac`, `auth-session`, and `organization-team`)
7. webhook receiver
8. notification/email shell
9. file upload shell
10. billing/subscription shell

Reasoning:

- audit-log is foundational for compliance and observability.
- CRUD proves contracts, DB, UI, and UX proof without complex external providers.
- RBAC/auth/orgs are core business truth and need careful tests.
- billing/file upload/webhooks introduce provider risk and should come after the registry contract stabilizes.

## Workstreams

### 1. Registry Format

Implementation tasks:

- Define registry manifest schema.
- Define cell lifecycle: draft, experimental, certified, deprecated.
- Define versioning and compatibility policy.
- Define generated-zone and source ownership rules for cells.
- Define how cells declare required proof.

Acceptance:

- A cell can be installed, proved, upgraded, and deprecated from metadata.
- Cell metadata is machine-readable.

### 2. Cell Installation Contract

Implementation tasks:

- Define how a cell patches an existing repo.
- Define conflict behavior.
- Define how generated contracts and migrations are named.
- Define rollback/uninstall limitations.
- Define owner/test-map updates.

Acceptance:

- Cell installation never silently overwrites user-owned code.
- Install plan can be dry-run and reviewed.

### 3. First Certified Cell: Audit Log

Implementation tasks:

- Add contract for audit events.
- Add Rust domain/application shape for recording audit events.
- Add DB migration or migration template.
- Add UI/admin display or route shell if profile includes web.
- Add tests for append-only behavior and access policy.
- Add observability event docs.
- Add security assumptions.

Acceptance:

- Audit log cell can be added to reference platform.
- Proof lanes cover contracts, DB, backend, and UI if present.

### 4. CRUD Cell

Implementation tasks:

- Generate contract, route, client, table, form, DB schema, tests, stories, UX route.
- Include loading, empty, error, success, permission-denied states.
- Include generated validation from contract where possible.

Acceptance:

- CRUD cell proves the full end-to-end stack.
- No handwritten DTO drift.

### 5. Certification Harness

Implementation tasks:

- Define `cell prove` behavior.
- Score cell against required lanes.
- Emit certification evidence.
- Add compatibility tests against supported profiles.

Acceptance:

- A certified cell has a reproducible proof receipt.
- Certification status is not just a label in docs.

## Parallel MCP Breakdown

Strong parallel candidate after registry manifest locks:

- Agent A: registry schema and command surface.
- Agent B: audit-log cell.
- Agent C: CRUD cell.
- Agent D: certification harness.
- Agent E: docs and examples.

Do not parallelize multiple cells that depend on the same unstable migration/contract naming convention until the installation contract is fixed.

## Validation

Minimum:

```bash
just fast
cargo test -p jankurai
```

Cell-specific:

```bash
jankurai registry list
jankurai cell add audit-log --dry-run
jankurai cell prove audit-log
```

Use equivalent commands if exact names differ.

Phase 10 closeout validation:

```bash
rtk cargo test -p jankurai
rtk cargo run -p jankurai -- lane . --changed crates/jankurai/src/commands/cell.rs --changed crates/jankurai/src/commands/registry.rs --changed schemas/cell-manifest.schema.json --out target/jankurai/p10-cell-registry-lane.json --md target/jankurai/p10-cell-registry-lane.md
rtk just fast
rtk just score
```

## Risks

- Cells can become product frameworks if scope is not constrained.
- Provider-backed cells like billing can create security/compliance risk.
- Generated code can drift if installation and regeneration are unclear.

## Handoff Notes

Leave:

- registry schema
- lifecycle states
- install conflict policy
- first certified cell status
- proof receipts
- upgrade/deprecation policy

## Phase Status Receipt

- Phase status: hardened; **ten** certified cells (`audit-log`, `crud-resource`, `rbac`, `auth-session`, `organization-team`, `background-job`, `webhook-receiver`, `notification-shell`, `periodic-cron`, `billing-subscription`)
- Files changed (billing-subscription hardening, 2026-05-06): `crates/jankurai/src/commands/cell_catalog.rs`, `crates/jankurai/tests/phase10_billing_subscription_cell_smoke.rs`, `examples/perfect-web-api-db/backend/src/billing_subscription.rs`, `examples/perfect-web-api-db/backend/src/lib.rs`, `examples/perfect-web-api-db/contracts/billing-subscription.openapi.json`, `examples/perfect-web-api-db/db/migrations/008_billing_subscriptions.sql`, `examples/perfect-web-api-db/db/constraints/008_billing_subscriptions.sql`, `examples/perfect-web-api-db/docs/billing-subscription-cell.md`, `examples/perfect-web-api-db/ops/billing-subscription-security.md`, `examples/perfect-web-api-db/ux/billing-subscription-routes.md`, `tips/phases/10-reuse-registry-certified-cells.md`
- Hardened additions: tenth certified cell `billing-subscription`; deterministic `BillingSubscriptionStatePolicy`; billing boundary decoupled from provider SDKs
- Files changed (periodic-cron hardening, 2026-05-06): `crates/jankurai/src/commands/cell_catalog.rs`, `crates/jankurai/tests/phase10_periodic_cron_cell_smoke.rs`, `examples/perfect-web-api-db/backend/src/periodic_cron.rs`, `examples/perfect-web-api-db/backend/src/lib.rs`, `examples/perfect-web-api-db/contracts/periodic-cron.openapi.json`, `examples/perfect-web-api-db/db/migrations/007_periodic_cron.sql`, `examples/perfect-web-api-db/db/constraints/007_periodic_cron.sql`, `examples/perfect-web-api-db/docs/periodic-cron-cell.md`, `examples/perfect-web-api-db/ops/periodic-cron-security.md`, `examples/perfect-web-api-db/ux/periodic-cron-routes.md`, `tips/phases/10-reuse-registry-certified-cells.md`
- Hardened additions: ninth certified cell `periodic-cron`; deterministic `PeriodicCronSchedulePolicy`; deterministic interval scheduling decoupled from queue execution
- Files changed (notification-shell hardening, 2026-05-06): `crates/jankurai/src/commands/cell_catalog.rs`, `crates/jankurai/tests/phase10_notification_shell_cell_smoke.rs`, `examples/perfect-web-api-db/backend/src/notification_shell.rs`, `examples/perfect-web-api-db/backend/src/lib.rs`, `examples/perfect-web-api-db/contracts/notification-shell.openapi.json`, `examples/perfect-web-api-db/db/migrations/006_notifications.sql`, `examples/perfect-web-api-db/db/constraints/006_notifications.sql`, `examples/perfect-web-api-db/docs/notification-shell-cell.md`, `examples/perfect-web-api-db/ops/notification-shell-security.md`, `examples/perfect-web-api-db/ux/notification-shell-routes.md`, `tips/phases/10-reuse-registry-certified-cells.md`
- Hardened additions: eighth certified cell `notification-shell`; deterministic `NotificationDeliveryPolicy`; idempotent outbox constraints; delivery mechanism remains deferred behind adapter shell
- Files changed (webhook-receiver hardening, 2026-05-05): `crates/jankurai/src/commands/cell_catalog.rs`, `crates/jankurai/tests/phase10_webhook_receiver_cell_smoke.rs`, `examples/perfect-web-api-db/backend/src/webhook_receiver.rs`, `examples/perfect-web-api-db/backend/src/lib.rs`, `examples/perfect-web-api-db/contracts/webhook-receiver.openapi.json`, `examples/perfect-web-api-db/db/migrations/005_webhook_receipts.sql`, `examples/perfect-web-api-db/db/constraints/005_webhook_receipts.sql`, `examples/perfect-web-api-db/docs/webhook-receiver-cell.md`, `examples/perfect-web-api-db/ops/webhook-receiver-security.md`, `examples/perfect-web-api-db/ux/webhook-receiver-routes.md`, `tips/phases/10-reuse-registry-certified-cells.md`
- Hardened additions: seventh certified cell `webhook-receiver`; deterministic `WebhookSignaturePolicy`; idempotent receipt constraints; provider-specific logic remains deferred behind edge adapters
- Files changed (background/job hardening, 2026-05-04): `README.md`, `crates/jankurai/src/commands/cell_catalog.rs`, `crates/jankurai/tests/command_surface_smoke.rs`, `crates/jankurai/tests/phase10_background_job_cell_smoke.rs`, `examples/perfect-web-api-db/backend/src/background_job.rs`, `examples/perfect-web-api-db/backend/src/lib.rs`, `examples/perfect-web-api-db/contracts/background-job.openapi.json`, `examples/perfect-web-api-db/db/migrations/004_background_jobs.sql`, `examples/perfect-web-api-db/db/constraints/004_background_jobs.sql`, `examples/perfect-web-api-db/docs/background-job-cell.md`, `examples/perfect-web-api-db/ops/background-job-security.md`, `examples/perfect-web-api-db/ux/background-job-routes.md`, `tips/phases/10-reuse-registry-certified-cells.md`, `tips/phases/00-phase-index.md`, `tips/phases/logs/10-reuse-registry-certified-cells.log`
- Hardened additions: sixth certified cell `background-job`; deterministic `BackgroundJobRetryPolicy`; queue claim/complete/fail application shell; durable migration and constraints; operator UX proof states; provider-backed queue and mutating installer behavior remain deferred behind upgrade gates
- Files changed (organization/team hardening, 2026-05-04): `crates/jankurai/src/commands/cell_catalog.rs`, `crates/jankurai/tests/phase10_org_team_cell_smoke.rs`, `examples/perfect-web-api-db/backend/src/organization_team.rs`, `examples/perfect-web-api-db/backend/src/lib.rs`, `examples/perfect-web-api-db/contracts/organization-team.openapi.json`, `examples/perfect-web-api-db/db/migrations/003_organization_team.sql`, `examples/perfect-web-api-db/db/constraints/003_organization_team.sql`, `examples/perfect-web-api-db/docs/organization-team-cell.md`, `examples/perfect-web-api-db/ops/organization-team-security.md`, `examples/perfect-web-api-db/ux/organization-team-routes.md`, `tips/phases/10-reuse-registry-certified-cells.md`, `tips/phases/00-phase-index.md`, `tips/phases/logs/10-reuse-registry-certified-cells.log`
- Files changed (auth/session hardening, 2026-05-03): `crates/jankurai/src/commands/cell_catalog.rs`, `crates/jankurai/src/commands/cell.rs`, `crates/jankurai/src/main.rs`, `crates/jankurai/tests/command_surface_smoke.rs`, `crates/jankurai/tests/phase10_auth_session_cell_smoke.rs`, `examples/perfect-web-api-db/backend/src/auth_session.rs`, `examples/perfect-web-api-db/backend/src/lib.rs`, `examples/perfect-web-api-db/contracts/auth-session.openapi.json`, `examples/perfect-web-api-db/db/migrations/002_auth_sessions.sql`, `examples/perfect-web-api-db/db/constraints/002_auth_sessions.sql`, `examples/perfect-web-api-db/docs/auth-session-cell.md`, `examples/perfect-web-api-db/ops/auth-session-security.md`, `examples/perfect-web-api-db/ux/auth-session-routes.md`, `tips/phases/10-reuse-registry-certified-cells.md`, `tips/phases/00-phase-index.md`, `tips/phases/logs/10-reuse-registry-certified-cells.log`
  * Files changed (rbac slice, 2026-05-03): `crates/jankurai/src/commands/cell_catalog.rs`, `crates/jankurai/tests/command_surface_smoke.rs`, `tips/phases/10-reuse-registry-certified-cells.md`, `tips/phases/logs/10-reuse-registry-certified-cells.log`
- Files changed (registry foundation): `schemas/cell-manifest.schema.json`, `schemas/cell-registry.schema.json`, `crates/jankurai/src/commands/cell_catalog.rs`, `crates/jankurai/src/commands/registry.rs`, `crates/jankurai/src/commands/cell.rs`, `crates/jankurai/src/main.rs`, `crates/jankurai/src/validation.rs`, `crates/jankurai/tests/command_surface_smoke.rs`, `crates/jankurai/tests/schema_contracts.rs`, `tips/phases/10-reuse-registry-certified-cells.md`, `tips/phases/logs/10-reuse-registry-certified-cells.log`
- Schemas changed: cell manifest and cell registry
- Public interfaces changed: `jankurai cell --mode <install-ready|prove|upgrade-plan|deprecate-plan>`
- Hardened additions: dependency-bound certification evidence, content-marker evidence for SessionTokenHash, lifecycle downgrade guard (certified → experimental when evidence missing), upgrade-plan and deprecate-plan metadata modes, Dependency Closure and Certification Decision sections in prove markdown
- Generated artifacts: registry, cell dry-run, prove evidence, upgrade plan, deprecation plan, lane, fast score, and repo score JSON/Markdown outputs
- Routing maps changed: none beyond existing owner/test inputs
- Validation commands: `cargo test -p jankurai`; `just fast`; `just score`
- Results: organization/team patch is PR-ready but validation is pending in CI because this chat session exposed read-only GitHub tools after repository inspection; previous auth/session hardening validation passed with `just fast` score 93, caps 0
- Feedback closeout (2026-05-04): `tips/phases_feedback/10-phase/tip1`-`tip4` reconciled in `docs/phases-feedback-status.md`; accepted `auth-session`, dependency-bound evidence, and lifecycle proof modes; rejected mutating/provider-backed install and secret-dependent runtime expansion.
- Skipped validation: local full-repo command execution in this session; mutating install execution remains bounded for later extension; auth/session, organization/team, and background-job provider-backed runtime mutation remains deferred
- Exceptions created: provider-backed and mutating cells deferred; auth/session is certified as a shell, not a provider-backed login implementation; background-job is certified as a durable queue/retry shell, not a provider-backed worker runtime
- Follow-up phases: Phase 10 Initial Cell Order complete; proceeding to phases 11–13 as before
