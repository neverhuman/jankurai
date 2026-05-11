# Database Root

Status: planning source
Owner: standard
Last reviewed: 2026-05-02
Applies to: `db/`

This root is reserved for PostgreSQL migrations, constraints, and durable-truth policy.

The machine-readable route lives in `agent/boundaries.toml` under `[db]`.

Use `db/migrations/` for versioned schema changes and rollback/backfill notes.
Use `db/constraints/` for named constraints, RLS, and invariant notes.

Later `db/` work should keep durable truth in migrations and constraints, with rollback, backfill, lock, and application-owned transaction proof documented alongside the change.
