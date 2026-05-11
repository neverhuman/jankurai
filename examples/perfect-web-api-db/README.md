# perfect-web-api-db

Jankurai-native reference platform proving the COLD stack end to end.

## What This Proves

This scaffold demonstrates the Jankurai agent-native engineering standard
applied to a real fullstack SaaS skeleton:

- **Contracts**: OpenAPI 3.1 source of truth at `contracts/openapi.json` drives
  generated clients. No handwritten DTO mirrors.
- **Ownership**: Every layer has declared boundaries (domain, application,
  adapters, API edge, frontend). No layer violates another's ownership.
- **Lanes**: Proof routing maps changed paths to the smallest credible test
  commands. Changes to `backend/src/domain.rs` trigger Rust tests, not
  frontend builds.
- **Durability**: Domain invariants are enforced at two levels (Rust types +
  PostgreSQL constraints). Audit events are append-only. Error shapes are
  typed (RFC 9457 Problem Details).

## Stack

| Layer | Technology | Owns | Never Owns |
|-------|-----------|------|-----------|
| Domain | Rust | IDs, invariants, RBAC, errors | IO, DB, HTTP, env |
| Application | Rust | Commands, authz, audit events | UI, raw SQL |
| Adapters | Rust (sqlx) | DB queries, external APIs | Domain rules |
| API | Rust (axum) | HTTP edge, extraction, mapping | Domain rules, raw SQL |
| Frontend | TypeScript/React/Vite | UI, forms, generated clients | Secrets, DB, authz |
| Database | PostgreSQL | Migrations, constraints, indexes | App logic |
| Contracts | OpenAPI 3.1 | API shape, generated clients | Handwritten drift |

## File Layout

```text
backend/
  src/
    lib.rs          — module root with layer documentation
    domain.rs       — pure domain: typed IDs, invariants, RBAC, audit events
    application.rs  — commands, authorization, port traits, audit emission
    adapters.rs     — DB adapter contracts and boundary documentation
contracts/
  openapi.json      — OpenAPI 3.1 source of truth for API contract
db/
  migrations/
    001_init.sql    — production-grade schema with ENUMs, FKs, indexes
  constraints/
    001_accounts.sql — constraint-to-invariant documentation
docs/
  architecture.md   — architecture decision records
  exceptions.md     — time-bounded exception inventory
frontend/
  src/
    App.tsx         — React app with all UI states, ARIA, generated client slots
ops/
  observability.md  — trace IDs, structured logging, metrics, health check
  security.md       — secrets, dependencies, auth, CI hardening, compliance
ux/
  routes.md         — route matrix, state coverage, accessibility, client policy
```

## Running The Scaffold

This scaffold is intentionally a file-layout reference, not a running
application. To build a running version:

1. `cargo init --lib backend` and add the domain/application/adapters code.
2. Add `axum` for the API edge layer.
3. Run `openapi-typescript-codegen` to generate the frontend client.
4. Add `sqlx` for PostgreSQL adapters.
5. Run `jankurai init --profile rust-ts-postgres` for agent routing files.
6. Run `jankurai audit` to verify the score.

## Proof Lanes

| Changed Surface | Proof Command |
|----------------|---------------|
| `backend/src/domain.rs` | `cargo test -p perfect-web-api-db` |
| `backend/src/application.rs` | `cargo test -p perfect-web-api-db` |
| `contracts/openapi.json` | Contract drift check + client regeneration |
| `db/migrations/` | Migration review + constraint verification |
| `frontend/src/` | `npm run build && npm run test` |
| `ops/` | `just score` |
| `docs/` | `just score` |

## Exceptions

See [`docs/exceptions.md`](docs/exceptions.md) for the current exception
inventory. All exceptions have an owner, expiry condition, and migration path.

## Architecture Decisions

See [`docs/architecture.md`](docs/architecture.md) for the key design choices
and their relationship to the Jankurai standard.
