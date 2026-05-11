# db/migrations

Status: active routing surface
Owner: standard
Last reviewed: 2026-05-02
Applies to: `db/migrations/`

Put versioned PostgreSQL schema changes here.

Each migration should note rollback, backfill, lock behavior, and the app or adapter layer that owns the write path.

Keep the files small and explicit so audit and review can prove the change without guessing at hidden state.
