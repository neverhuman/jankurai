# Database Root

Status: planning source
Owner: standard
Last reviewed: 2026-05-02
Applies to: `db/`

This root is reserved for PostgreSQL migrations, constraints, and durable-truth policy.

The machine-readable route lives in `agent/boundaries.toml` under `[db]`.

Use `db/migrations/` for versioned schema changes and rollback/backfill notes.
Use `db/constraints/` for named constraints, RLS, and invariant notes.

Later `db/` work should keep durable truth in migrations, constraints, adapters, and application-owned transactions. Do not move business truth into app code that bypasses those layers.

Every DB change should make rollback, backfill, lock, and app-owned transaction ownership visible to the auditor. If the write path is not in a migration, a named constraint, or an adapter-owned transaction, it is the wrong layer.
