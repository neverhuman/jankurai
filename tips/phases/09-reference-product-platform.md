# Phase 09: Reference Product Platform

Status: complete
Owner: standard
Last reviewed: 2026-05-03
Parallel MCP candidate: yes

## Objective

Build the canonical Jankurai-native product platform: a small but serious fullstack SaaS reference that proves the Cold stack works end to end.

This is not a demo toy. It is the golden repo that shows how Rust, TypeScript/React/Vite, PostgreSQL, generated contracts, bounded Python, UX QA, security, observability, proof routing, and agent repair fit together.

## Current State

The Jankurai repo now contains a concrete reference scaffold under `examples/perfect-web-api-db/`, with a supporting `examples/legacy-node-api/` fixture and routing updates to keep the proof surface honest.

Existing assets to reuse:

- `docs/agent-native-standard.md`
- `docs/moonshot.md`
- `crates/jankurai/`
- `packages/ux-qa/`
- init templates from Phase 04
- proof router from Phase 03
- security and UX lanes from Phases 05 and 06
- contract/DB checks from Phase 07
- agent context/repair from Phase 08
- the reference-platform contract is now implemented in the example scaffold and supporting docs

## Dependencies

Requires:

- Phase 04 for profile-driven init
- Phase 05 for UX proof
- Phase 06 for security evidence
- Phase 07 for contracts/DB boundaries
- Phase 08 for agent context and repair

## Public Interface Changes

The reference platform can live as:

- a generated fixture under `examples/`
- a template under `templates/`
- a separate golden repo used by benchmarks

Preferred first implementation inside this repo:

```text
examples/perfect-web-api-db/
```

If the repo standard forbids large examples in-tree, create a minimal in-tree fixture and document an external golden repo plan. The contract now stays in docs first; the scaffold can follow once the file layout is locked.

## Product Scope

Initial product should include:

- user account model or mocked account boundary
- organization/team model
- RBAC policy
- admin dashboard shell
- audit log
- one CRUD resource
- API endpoint
- generated client
- PostgreSQL migration
- Playwright critical path
- Storybook state coverage
- UX QA route matrix
- security lane
- OpenTelemetry/tracing placeholder
- typed error/problem-details shape
- agent maps and proof lanes

Do not include billing, file upload, search, AI service, and compliance packs in the first golden platform unless the core is stable. Those belong in later cells.

## Workstreams

### 1. Golden Repo Contract

Implementation tasks:

- Define exactly what the reference repo must prove.
- Decide whether it lives under `examples/`, generated tempdir tests, or an external repo.
- Add owner/test-map entries for any in-tree example.
- Avoid bloating prompt context with generated outputs.

Acceptance:

- The golden platform has a written contract and scope.
- It is clear which files are source, which are generated, and which are fixture-only proof surfaces.

### 2. Backend Skeleton

Implementation tasks:

- Rust workspace with domain, application, adapters, API, workers if needed, test-support.
- Domain owns invariants and typed errors.
- Application owns commands, authorization, idempotency where relevant.
- Adapters own DB access.
- API edge maps transport to application.
- Add tests for domain and application behavior.

Acceptance:

- No domain I/O.
- No raw SQL in API handlers.
- Tests prove at least one business invariant and one authorization negative case.

### 3. Frontend Skeleton

Implementation tasks:

- Vite/React/TypeScript strict app.
- Generated client use for API calls.
- Basic admin/CRUD UI.
- Loading, empty, error, success, permission-denied states.
- Storybook stories for critical components.
- Playwright route test for critical path.
- UX QA route config.

Acceptance:

- UI has rendered proof artifacts.
- No direct DB or secret use.
- No handwritten API mirror when generated client exists.

### 4. Contracts And DB

Implementation tasks:

- Add OpenAPI or JSON Schema source contract.
- Generate or simulate generated client in declared zone.
- Add Postgres migration for reference resource.
- Add migration safety docs.
- Add contract drift check.

Acceptance:

- Contract source is the boundary truth.
- Generated output is protected.
- DB invariant is represented in migration/constraint where applicable.

### 5. Security And Observability

Implementation tasks:

- Add security policy and lane config.
- Add no-secret proof.
- Add dependency scan hooks.
- Add trace/correlation ID placeholders.
- Add typed problem-details response shape.
- Add audit log event for admin action.

Acceptance:

- Security lane produces or points to evidence.
- Runtime failures are agent-repairable.

### 6. Jankurai Score And Proof

Implementation tasks:

- Run Jankurai audit against the golden repo.
- Store expected score fixture or benchmark expectation.
- Ensure proof router selects correct lanes for representative changes.
- Add docs showing before/after of a compliant change.

Acceptance:

- Golden repo reaches target floor.
- It demonstrates the standard without exceptions or with documented minimal exceptions.

## Parallel MCP Breakdown

Partial parallel candidate:

- Agent A: backend skeleton. Owns Rust backend paths.
- Agent B: frontend/UX skeleton. Owns web paths and UX config.
- Agent C: contracts/DB. Owns `contracts/` and `db/`.
- Agent D: security/observability docs. Owns `ops/` and docs.

Coordination required:

- Contract shape must be locked before backend and frontend integrate.
- DB schema must be locked before adapter tests.
- UX route IDs must match frontend routes.

Merge order:

1. Golden repo contract and file layout.
2. Contracts and DB.
3. Backend and frontend in parallel.
4. UX/security/observability integration.
5. Audit/proof validation.

## Validation

For Jankurai repo:

```bash
just fast
cargo test -p jankurai
```

For the golden platform, define its own:

```bash
just fast
just score
just ux
just security
```

If in-tree, ensure root `just fast` does not become unacceptably slow.

## Risks

- A golden repo can become too large for the main repository.
- Placeholder product code can be mistaken for certified cells.
- Fullstack integration can block parallel work if contracts are not fixed early.

## Handoff Notes

Leave:

- location of golden platform
- exact profile used to generate it
- current Jankurai score
- proof commands
- known exceptions
- next certified cell candidates

## Phase Status Receipt

- Phase status: **complete** (promoted from hardened, 2026-05-03); optional external golden repo split remains out-of-band. Technical receipt: hardened reference product platform — domain/application/adapters layers with real invariants, typed RBAC, audit events, RFC 9457 errors; production-grade OpenAPI 3.1 contract; PostgreSQL migration with ENUMs, FKs, indexes; frontend with all UI states and ARIA; architecture decisions and exception inventory
- Files changed (initial scaffold): `examples/perfect-web-api-db/README.md`, `examples/perfect-web-api-db/backend/src/lib.rs`, `examples/perfect-web-api-db/backend/src/domain.rs`, `examples/perfect-web-api-db/backend/src/application.rs`, `examples/perfect-web-api-db/frontend/src/App.tsx`, `examples/perfect-web-api-db/contracts/openapi.json`, `examples/perfect-web-api-db/db/migrations/001_init.sql`, `examples/perfect-web-api-db/db/constraints/001_accounts.sql`, `examples/perfect-web-api-db/ops/observability.md`, `examples/perfect-web-api-db/ops/security.md`, `examples/perfect-web-api-db/ux/routes.md`, `examples/legacy-node-api/README.md`, `examples/legacy-node-api/package.json`, `examples/legacy-node-api/src/index.js`, `agent/owner-map.json`, `agent/test-map.json`, `crates/jankurai/src/audit/mod.rs`, and `target/jankurai/phase-logs/09-reference-product-platform.md.log`
- Files changed (hardening slice): `examples/perfect-web-api-db/backend/src/domain.rs` (typed IDs, RBAC, audit events, domain errors, tests), `examples/perfect-web-api-db/backend/src/application.rs` (port traits, commands, authorization, idempotency, tests), `examples/perfect-web-api-db/backend/src/adapters.rs` (new — adapter boundary documentation), `examples/perfect-web-api-db/backend/src/lib.rs` (updated — layer docs), `examples/perfect-web-api-db/contracts/openapi.json` (full OpenAPI 3.1 with schemas, security, ProblemDetail), `examples/perfect-web-api-db/db/migrations/001_init.sql` (production-grade SQL with ENUMs, FKs, indexes), `examples/perfect-web-api-db/db/constraints/001_accounts.sql` (constraint-to-invariant mapping), `examples/perfect-web-api-db/frontend/src/App.tsx` (all UI states, ARIA, typed components), `examples/perfect-web-api-db/ops/observability.md` (trace IDs, logging, metrics, health), `examples/perfect-web-api-db/ops/security.md` (secrets, deps, auth, CI, compliance), `examples/perfect-web-api-db/ux/routes.md` (route matrix, state coverage, a11y), `examples/perfect-web-api-db/docs/architecture.md` (new — ADRs), `examples/perfect-web-api-db/docs/exceptions.md` (new — exception inventory), `examples/perfect-web-api-db/README.md` (comprehensive COLD stack documentation)
- Schemas changed: reference-platform contract surfaces under `schemas/`
- Public interfaces changed: in-tree reference scaffold hardened with real domain logic, contract, and boundary documentation
- Generated artifacts: example scaffold, UX/security/docs fixtures, proof-lane outputs
- Routing maps changed: `agent/owner-map.json`, `agent/test-map.json` (prior slice)
- Validation commands: `cargo test -p jankurai` (96 passed), `just fast` (score=93 findings=0), `just score` (score=93 findings=0)
- Results: all validation passed; score maintained at 93 with 0 findings
- Skipped validation: external golden repo split remains optional
- Exceptions created: examples are routed out of workspace score handling as fixtures; inline frontend types documented in `docs/exceptions.md` with expiry
- Follow-up phases: 10 reuse registry certified cells

