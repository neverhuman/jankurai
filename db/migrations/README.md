# db/migrations

Status: active routing surface
Owner: standard
Last reviewed: 2026-05-02
Applies to: `db/migrations/`

Put versioned PostgreSQL schema changes here.

Each migration should note rollback, backfill, lock behavior, and the app or adapter layer that owns the write path.
Prefer app-owned transactions for the mutation boundary and keep the migration focused on schema shape, data movement, and safety markers.

When a migration is paired with a backfill or expand/contract deploy, document the staged rollout, the rollback path, and any lock timeout or advisory-lock choice in the same change set.

Keep the files small and explicit so audit and review can prove the change without guessing at hidden state.
