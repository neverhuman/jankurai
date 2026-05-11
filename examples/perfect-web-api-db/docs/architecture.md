# Architecture Decisions

## ADR-001: Domain Layer Purity

**Decision**: Domain layer has zero IO dependencies.

**Context**: The Jankurai standard requires that `crates/domain` owns IDs,
invariants, and pure decisions but never owns IO, env, time, random, DB, or
framework types.

**Consequences**:
- All persistence is abstracted through repository port traits in application.
- Domain tests require no database, network, or filesystem.
- Authorization decisions are pure functions on `Account` and `Role`.

## ADR-002: Contract-First API Design

**Decision**: `contracts/openapi.json` is the source of truth for all API
endpoints and request/response shapes.

**Context**: The Jankurai standard forbids handwritten DTO mirrors when
generated clients exist. The contract file drives client generation, server
stub generation, and documentation.

**Consequences**:
- Frontend types must come from generated clients, not hand-maintained interfaces.
- Backend response shapes must match the contract schemas.
- Contract drift is detected by the `contract` proof lane.

## ADR-003: Database Constraints Mirror Domain Invariants

**Decision**: Critical domain invariants are enforced at both the domain layer
(Rust types and validation) and the database layer (SQL constraints, ENUMs, FKs).

**Context**: Defense in depth. If a bug bypasses domain validation, the database
provides a safety net. If the database constraint is too coarse, the domain
provides precise error messages.

**Consequences**:
- Email uniqueness: domain `Account::new()` + DB `UNIQUE` constraint.
- Role validity: domain `Role` enum + DB `account_role` ENUM type.
- Resource ownership: domain `Resource.owner_id` + DB FK.
- Constraint documentation lives in `db/constraints/` for agent visibility.

## ADR-004: Audit Events Are Append-Only

**Decision**: The `audit_events` table has no UPDATE or DELETE operations.

**Context**: Compliance and observability require immutable audit trails. The
application layer emits audit events through the `AuditLog` port trait; the
adapter writes to PostgreSQL.

**Consequences**:
- No `update_audit_event` or `delete_audit_event` commands exist.
- The table grows over time; archival is an ops concern, not a domain concern.
- Production should add RLS or triggers to prevent accidental mutation.

## ADR-005: RFC 9457 Problem Details For Errors

**Decision**: All API error responses use the RFC 9457 Problem Details shape.

**Context**: Typed, machine-readable errors make agent-driven repair possible.
String-only error messages require human interpretation.

**Consequences**:
- `ProblemDetail` schema is defined in `contracts/openapi.json`.
- Frontend error display uses the `detail` and `title` fields.
- Agents can parse error responses and map them to repair actions.
