# Bad Migration Behavior: Comprehensive Guide

This document organizes the worst database migration behaviors that are inexcusable in production. Every item is drawn from PostgreSQL/SQLite official documentation, mature migration tooling guidance (Flyway, Atlas, Prisma), GitLab's production migration playbooks, and the jankurai agent-native engineering standard.

## Known Best Practices

To counteract the anti-patterns listed below, ensure adherence to these core migration best practices:

- **Version, order, and immutabilize**: Every migration is a committed, checksummed artifact. Once applied to a permanent environment, never edit it; create a new migration and roll forward.
- **Structured metadata over keyword comments**: A migration passes audit not because it contains the word "rollback" or `jankurai:migration-safe`, but because it carries machine-readable evidence in adjacent `.meta.toml` / `.jankurai.toml` / `meta.toml` / `migration.toml` metadata and verification artifacts.
- **Expand / backfill / contract**: Safe production changes are phased. Expand adds nullable or additive schema; backfill migrates data in batches; contract removes old structures only after all deployed code no longer needs them.
- **Lock awareness**: Every migration touching an existing table must declare `lock_timeout`, `statement_timeout`, expected lock level, and table-size estimate.
- **Separate schema from data**: DDL migrations should be short and deterministic. Large data rewrites belong in separate, batched, resumable backfills.
- **Dialect awareness**: PostgreSQL and SQLite have fundamentally different locking, transactional DDL, and constraint behaviors. Migrations must be engine-explicit.
- **Proof over trust**: Backup confidence, restore drills, verification queries, rollback plans, and human ownership are required evidence — not optional comments.

## 1. Immediate rejection behaviors

1. **Destructive DDL without structured safety evidence.** `DROP TABLE`, `DROP COLUMN`, `TRUNCATE`, `DROP INDEX`, `DROP CONSTRAINT`, or `CASCADE` in a migration without a companion rollback or roll-forward plan, backup/restore confidence or explicit irreversible approval, lock/timeout posture, verification artifact, dependency inventory when cascading, and human owner/approval declaration. A comment saying "safe", "rollback", or `jankurai:migration-safe` is not evidence.
2. **Editing a previously applied migration.** Once a versioned migration has been applied to staging or production, its checksum is locked. Editing it breaks replay, checksum validation, and downstream environment consistency. Create a new migration and roll forward. Flyway tracks applied versioned migrations with checksums and recommends rolling forward rather than editing applied migrations.
3. **`DROP ... CASCADE` as a convenience.** `CASCADE` silently drops dependent views, constraints, functions, triggers, and policies. It must never be used unless every dependency has been inventoried and the cascade scope is explicitly documented and approved.
4. **`CREATE INDEX CONCURRENTLY` inside a transaction block.** PostgreSQL forbids concurrent index creation inside `BEGIN`/`COMMIT` blocks. Attempting it fails at runtime. More dangerously, wrapping it silently converts it to a blocking `CREATE INDEX`. Any tool or migration runner that wraps SQL in implicit transactions can trigger this.
5. **Manual production schema changes outside version control.** Hand-applied DDL in a production console bypasses review, audit trail, rollback planning, and environment consistency. Every schema change must be a committed migration artifact.
6. **Running ORM/AI-generated migrations without reviewing the SQL.** Prisma, Alembic, ActiveRecord, Diesel, and AI tools generate migration SQL that may contain destructive operations, dialect mismatches, missing constraints, or unsafe type changes. Generated migrations are proposals, not trusted outputs.
7. **Destructive migrations without backup and restore confidence.** A `DROP TABLE` or `TRUNCATE` is not recoverable from application code. There must be evidence that a backup exists and that restore has been tested or a restore drill is referenced.
8. **Running destructive migrations and code deployment in one irreversible step.** Destructive schema changes must be decoupled from application deployment. Combining them eliminates the ability to roll back either independently.

## 2. Migration structure and ordering failures

1. Name migrations ambiguously: `fix.sql`, `update.sql`, `migration.sql`, `new_final.sql`, `001.sql`.
2. Use non-sortable identifiers that make ordering indeterminate.
3. Use sequence numbers without timestamps, allowing parallel development branches to collide.
4. Skip version numbers, creating gaps that confuse replay and audit.
5. Store migrations outside the canonical `db/migrations/` root without declaring the path in `agent/boundaries.toml`.
6. Mix PostgreSQL and SQLite migrations in a single directory without dialect-aware subdirectories.
7. Omit the `.down.sql` or rollback artifact without an explicit `rollback = "roll-forward only"` rationale in metadata.
8. Omit the `.verify.sql` or check query that proves the migration achieved its intended state.
9. Omit the `.meta.toml` sidecar that declares engine, phase, risk, owner, timeouts, and rollback strategy.
10. Allow migration files to be empty or contain only comments.
11. Allow migrations that are not idempotent when the deployment tooling may retry.
12. Depend on current application code inside migrations that may be rerun months later against different app versions.

## 3. Expand/contract pattern failures

1. Drop a column in the same deploy that stops using it. Old code may still be running and querying the removed column.
2. Rename a column or table in one step while old code may still reference the old name.
3. Change a column type on a large hot table without rewrite/lock analysis and a multi-step migration plan.
4. Add `NOT NULL` to an existing large column in one step without first backfilling and then validating.
5. Combine schema change, data rewrite, and constraint addition in a single migration file.
6. Require app-code sequencing without referencing the feature flag, deploy order, or compatibility plan in migration metadata.
7. Drop a table before proving no code, jobs, reports, views, functions, or replicas depend on it.
8. No compatibility window between expand and contract phases.
9. No release choreography documentation for multi-step schema changes.
10. Treat staging success on tiny data as production proof for expand/contract timing.

## 4. Lock and timeout recklessness

1. Run migrations without `lock_timeout` set. A migration waiting for a lock can queue behind it and block all other queries on the table.
2. Run migrations without `statement_timeout` set. A long-running migration can hold locks for minutes or hours.
3. Run DDL while long transactions are open. Long transactions prevent lock acquisition and can cause cascading timeouts.
4. Run `ALTER TABLE` on a hot table without knowing the lock level required.
5. Run `ALTER TABLE` on a table without knowing the table size.
6. Add `NOT NULL`, defaults, generated columns, foreign keys, unique constraints, or indexes to large live tables without checking locks, rewrites, validation strategy, and version-specific behavior.
7. Add multiple foreign keys in one risky migration, each of which acquires locks independently.
8. Run `VACUUM FULL`, `CLUSTER`, blocking `REINDEX`, or table rewrites during normal traffic without an explicit maintenance plan.
9. Omit `idle_in_transaction_session_timeout` for migration sessions.
10. Assume advisory locks provide correctness guarantees without understanding their scope and lifecycle.

## 5. PostgreSQL-specific migration hazards

1. `CREATE INDEX` on a write-heavy production table without `CONCURRENTLY`. Blocks all writes for the entire index build.
2. `CREATE INDEX CONCURRENTLY` inside a transaction block. Fails or silently degrades to blocking mode.
3. `CREATE INDEX CONCURRENTLY` without a cleanup plan for invalid indexes left behind by failures.
4. `CREATE INDEX` with anonymous/generated names instead of explicit named indexes.
5. Assume concurrent index creation is safe without monitoring progress and failure cleanup.
6. Add foreign keys without supporting indexes on the referencing column. Deletes and updates on the parent table scan the child without an index.
7. Add foreign keys and leave them unvalidated forever. `NOT VALID` constraints skip initial validation but must eventually be validated.
8. Add unique constraints without duplicate detection and cleanup first.
9. Use `NOT VALID` for constraints without ever scheduling the `VALIDATE CONSTRAINT` step.
10. Create extensions in production migrations without privilege, version, and environment checks.
11. Use `SECURITY DEFINER` functions without a fixed `search_path` and restricted `EXECUTE` grants.
12. Modify RLS policies, privileges, or security-definer functions without security review.
13. Run migrations as a superuser or table owner role when a dedicated migration role should be used.
14. Let the migration role also serve as the runtime application role.
15. Ignore `search_path` for object resolution in migration SQL that will persist.
16. Mark table-reading or time-dependent functions as `IMMUTABLE` in migration-created functions.

## 6. SQLite-specific migration hazards

1. Table rebuilds (`CREATE TABLE new_...; INSERT INTO new_... SELECT * FROM old_...; DROP TABLE old_...; ALTER TABLE new_... RENAME TO ...`) without running `PRAGMA foreign_key_check` and `PRAGMA integrity_check` after the rebuild.
2. Assume `ALTER TABLE` in SQLite supports the same operations as PostgreSQL. SQLite `ALTER TABLE` is limited to `RENAME TABLE`, `RENAME COLUMN`, `ADD COLUMN`, and `DROP COLUMN` (SQLite 3.35+).
3. Disable `PRAGMA foreign_keys` during migrations and forget to re-enable it.
4. Run migrations that assume WAL mode without explicitly setting it.
5. Fail to handle concurrent write contention in SQLite migrations (SQLite uses database-level locking).
6. Use SQLite-specific syntax without documenting that the migration is SQLite-only.

## 7. Backfill and data migration failures

1. Run huge backfills in one transaction. A single `UPDATE users SET normalized_email = lower(email)` on a million-row table locks the table for the entire rewrite.
2. Run huge deletes in one transaction without batching and vacuum planning.
3. Backfill without batching.
4. Backfill without progress logging.
5. Backfill without pause/resume capability.
6. Backfill without idempotency. A crashed backfill that cannot be safely rerun is a production incident.
7. Backfill without throttling to manage WAL, replication lag, and autovacuum pressure.
8. Backfill without an index supporting the batch predicate.
9. Backfill without monitoring replication lag, locks, and vacuum impact.
10. Backfill without an upper bound, progress table, or break condition.
11. Backfill that can infinite-loop.
12. Update every row when only changed rows need updates.
13. Mix schema DDL and large data backfill in the same migration file.
14. Run backfills through application code that may change before the backfill finishes.
15. Use ORM models inside migrations when model definitions can drift from the schema at migration time.
16. Backfill without measuring WAL volume, bloat, and job duration.

## 8. ORM and AI-generated migration dangers

1. Accept ORM-generated migrations without reading the SQL.
2. Accept AI-generated migrations without reading every line.
3. Assume the ORM's migration diff tool understands your production locking, data volume, and rollback needs.
4. Let the ORM generate `DROP`/`TRUNCATE`/`CASCADE` operations that pass silently.
5. Let the ORM choose column types, defaults, indexes, and constraints without review.
6. Use development-only commands (`db push`, migration reset, schema sync, destructive re-baselining) in production configurations.
7. Production migration lanes deploy uncommitted or generated-but-unreviewed migrations.
8. Trust AI-generated rollback SQL without verifying it actually undoes the forward migration.
9. Accept "AI said it's safe" as a substitute for lock analysis, table-size awareness, and backup confidence.

## 9. Operational and rollback negligence

1. No tested restore path. A backup that has not been restored is an assumption, not a recovery strategy.
2. No backup before destructive migrations. Running `DROP TABLE` without a recent backup is irreversible data loss.
3. No rollback plan or roll-forward plan. Every migration must declare how it is undone or how the system moves forward if it fails.
4. No migration owner watching production during risky changes.
5. No fresh backup or PITR confidence before destructive work.
6. No production-like data volume for migration testing. Staging with 100 rows does not prove a migration is safe on 100 million rows.
7. No migration replay from an empty database in CI. If migrations cannot be replayed from scratch, the migration history is broken.
8. Leave half-applied migrations unresolved.
9. Leave invalid indexes after failed concurrent index creation.
10. Leave schema drift between environments.
11. Leave old columns, old indexes, old triggers, and old functions forever because cleanup is scary.
12. No migration observability: locks, waits, rows processed, lag, errors, and rollback state.
13. Treat staging success on tiny data as production proof.
14. No alert when migration duration exceeds expected bounds.
15. No runbook for failed migrations, lock pileups, or partial application.

## 10. Migration safety evidence requirements

Every migration that touches an existing table or removes/modifies schema should carry structured evidence. For destructive or risky exceptions, comment-only proof is rejected: `jankurai:migration-safe`, "rollback", "backup", and similar comments do not suppress the audit unless the same migration also has structured metadata plus a verification/check artifact.

1. **Engine**: `postgres`, `sqlite`, or both.
2. **Phase**: `expand`, `backfill`, `validate`, `contract`, `cleanup`, `seed`, `rls`, `constraint`, `index`, `repair`.
3. **Risk level**: `low`, `medium`, `high`, `critical`.
4. **Owner**: The person or team responsible for this migration in production.
5. **Affected objects**: Which tables, columns, constraints, or indexes change.
6. **Backward compatibility**: Whether old and new app versions can coexist.
7. **Transaction mode**: Whether the migration runs in a transaction, uses `CONCURRENTLY`, or is non-transactional.
8. **Timeouts**: `lock_timeout` and `statement_timeout` values.
9. **Rollback strategy**: `safe-before-writes`, `fix-forward`, `companion-down-migration`, or explicit rationale.
10. **Backfill**: Whether a separate backfill is required, its batch size, resume key, and throttle.
11. **Verification**: Pre-check and post-check queries that prove migration correctness.
12. **Approval**: Whether human approval is required for this migration.

## 11. "Not automatically bad" tools that become inexcusable when cargo-culted

1. **`DROP TABLE`** is fine when preceded by dependency inventory, backup confidence, data-retention compliance, and a compatibility window. It is bad as a cleanup shortcut.
2. **`CASCADE`** is fine when the cascade scope has been explicitly inventoried and approved. It is bad as a way to silence dependency errors.
3. **ORM migrations** are fine as starting proposals. They are bad when deployed without reading the generated SQL.
4. **AI-generated SQL** is fine for drafting. It is bad when trusted without human review, lock analysis, and backup confidence.
5. **`CONCURRENTLY`** is fine and preferred for index creation. It is bad inside transaction blocks or without invalid-index cleanup.
6. **`NOT VALID`** is fine for phased constraint addition. It is bad when `VALIDATE CONSTRAINT` is never scheduled.
7. **Backfills** are fine when batched, idempotent, resumable, and throttled. They are bad as one giant transaction.
8. **Advisory locks** are fine for coordinating migration runners. They are bad as security or correctness controls.
9. **`search_path`** manipulation is fine for multi-schema setups. It is bad when migration-created objects depend on session state.
10. **Migration runners** are fine. They are bad when they wrap `CONCURRENTLY` in implicit transactions or swallow errors.

## 12. Code-review "hard reject" summary for migrations

1. Destructive DDL without rollback, backup, dependency, and owner evidence.
2. Edited previously-applied migration files.
3. `CASCADE` without explicit dependency inventory.
4. `CREATE INDEX` on hot tables without `CONCURRENTLY` or maintenance window.
5. `CREATE INDEX CONCURRENTLY` inside a transaction block.
6. Missing `lock_timeout` and `statement_timeout` for production migrations.
7. Mixed schema DDL and large data backfill in one migration.
8. Unbatched, non-resumable, non-idempotent backfills.
9. No verification query proving migration correctness.
10. Generated migration accepted without SQL review.
11. No backup confidence before destructive work.
12. No compatibility window for expand/contract changes.
13. Production schema changes applied outside version-controlled migrations.
14. Migration uses runtime application role instead of dedicated migration role.
15. SQLite rebuild without `foreign_key_check` and `integrity_check`.

## 13. Jankurai audit enforcement

The following migration behaviors are enforced by jankurai audit rules:

### Hard blocks (HLT-021-DESTRUCTIVE-MIGRATION)

- `DROP TABLE`, `DROP DATABASE`, `DROP SCHEMA` without safety evidence
- `TRUNCATE TABLE` without safety evidence
- `DROP COLUMN`, `DROP INDEX`, `DROP CONSTRAINT` without safety evidence
- `DELETE FROM` without `WHERE` clause in migration files
- `ALTER TABLE ... DROP` without safety evidence
- All of the above are suppressed only when adjacent same-stem or same-directory metadata (`.meta.toml`, `.jankurai.toml`, `meta.toml`, or `migration.toml`) declares owner/approval, rollback or roll-forward, backup/restore confidence or explicit irreversible approval, lock/timeout posture, and verify/check evidence. `jankurai:migration-safe` or "rollback" comments alone are not sufficient proof.

### Hard blocks (HLT-030-SQL-BAD-BEHAVIOR — migration sub-detectors)

- `CREATE INDEX CONCURRENTLY` inside a `BEGIN`/`START TRANSACTION` block
- `DROP ... CASCADE` or `TRUNCATE ... CASCADE` without structured dependency inventory
- Risky PostgreSQL DDL without `lock_timeout`
- Risky PostgreSQL DDL without `statement_timeout`
- Unbatched full-table `UPDATE`/`DELETE` in migration files
- `VACUUM FULL`, `CLUSTER`, or blocking `REINDEX` without maintenance-window metadata
- SQLite unsafe PRAGMAs (`writable_schema = ON`, `journal_mode = OFF`, or disabling `foreign_keys` without re-enable/checks)
- SQLite table rebuilds without `foreign_key_check` and `quick_check`/`integrity_check`
- String-built or dynamically concatenated SQL reaching execution sinks

### Soft recommendations

- Use expand/contract phasing for schema changes
- Separate schema DDL from data backfills
- Use `NOT VALID` / `VALIDATE CONSTRAINT` for constraints on large tables
- Prefer `CREATE INDEX CONCURRENTLY` for live PostgreSQL tables, or document an approved maintenance window
- Keep dialect-specific migrations in explicit PostgreSQL/SQLite folders when both engines exist
- Prefer explicit `idle_in_transaction_session_timeout` for migration sessions
- Review ORM- or AI-generated migrations as proposals, not trusted outputs
- Declare explicit role separation (migration runner vs. app runtime)
- Include `.verify.sql` companion for every migration
- Use sortable timestamp-based migration naming
