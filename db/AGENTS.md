# Database Guidance

Read `agent/JANKURAI_STANDARD.md` first.

Owns durable database truth under `db/`: migrations, constraints, adapter-owned write paths, rollback notes, backfills, and lock safety.
Forbidden: application logic, transport routing, UI concerns, and any write path that bypasses a migration, a named constraint, or an adapter-owned transaction.
Proof lane: migration and constraint tests, plus the DB proof route in `agent/test-map.json`.
