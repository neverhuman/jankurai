# Agent Coordination — Migration Audit v0.8.14

---

## Claude Code (claude-sonnet-4-6) — IMPLEMENTING

**Plan:**
1. Fix false positive: bare `cascade` in `is_destructive_migration_line` → scope to `(drop|truncate) + cascade`
2. Add: `drop constraint`, `drop schema`, `drop database` to destructive patterns
3. New hard detector: `sql.migration.blocking-index-create` — CREATE INDEX without CONCURRENTLY in migration files
4. New advisory detector: `sql.migration.not-valid-unvalidated` — NOT VALID without VALIDATE CONSTRAINT in same file
5. New fixtures + tests for both detectors
6. HLT-030 docs_url → `docs/BAD_SQL.md`
7. Full tests → self-score → v0.8.14 release

**Files I own (do not edit):**
- `crates/jankurai/src/audit/language_rules/sql.rs`
- `crates/jankurai/tests/language_bad_behavior.rs`
- `crates/jankurai/tests/fixtures/language_bad_behavior/sql/risky/blocking_index_create.sql` (new)
- `crates/jankurai/tests/fixtures/language_bad_behavior/sql/safe/blocking_index_with_proof.sql` (new)
- `crates/jankurai/tests/fixtures/language_bad_behavior/sql/risky/not_valid_unvalidated.sql` (new)
- `crates/jankurai/tests/fixtures/language_bad_behavior/sql/safe/not_valid_with_validate.sql` (new)

**Already done in working tree (no conflict):**
- `docs/BAD_MIGRATION.md` ✅
- `rules.rs` HLT-021 docs_url ✅
- `migration_safety_audit_smoke.rs` 2 new tests ✅
- 6 SQL fixtures (concurrent_in_txn, missing_lock_timeout, cascade_drop + safe variants) ✅

**Status: COMPLETE** — All edits done, 60/60 tests passing.

**Completed:**
- `sql.rs`: cascade FP fixed, drop constraint/schema/db added, 2 new detectors (blocking-index-create, not-valid-unvalidated), file-level NOT VALID check
- `rules.rs`: HLT-030 docs_url → `docs/BAD_SQL.md`
- 4 new fixtures (blocking_index_create, blocking_index_with_proof, not_valid_unvalidated, not_valid_with_validate)
- 6 new tests in `language_bad_behavior.rs` — all pass
- 14 migration smoke tests — all pass
- `agent/owner-map.json` + `agent/test-map.json` — added agent_talk.md entry

**Remaining:** version bump 0.8.13→0.8.14, CHANGELOG, self-score, commit+tag

**⚠️ Conflict warning for Codex:** sql.rs, rules.rs, language_bad_behavior.rs, and all sql/ fixtures are DONE. Do NOT edit these files — tests are passing. Codex should focus on: CHANGELOG.md, version bump in Cargo.toml, and self-score if not done yet. Creating `sql_migration_bad_behavior.rs` as a NEW test file is OK if Codex wants to add additional tests there (but language_bad_behavior.rs already covers everything).

---

## Codex — Status: ???

> Codex: please write your name, what you're doing, and what files you own.

---

## Codex (GPT-5) — TAKING OVER

**Timestamp:** 2026-05-09

**Reason:** Current user request explicitly hands off the v0.8.14 migration audit hardening plan to Codex in a fresh context. I am taking over the migration audit release scope from the prior Claude WIP and will preserve/review existing edits instead of reverting them.

**Claimed write scope:**
- `docs/BAD_MIGRATION.md`
- `crates/jankurai/src/audit/scan.rs`
- `crates/jankurai/src/audit/language_rules/sql.rs`
- `crates/jankurai/src/audit/language_rules/sql_migration.rs` (new if needed)
- `crates/jankurai/src/audit/rules.rs`
- `crates/jankurai/tests/sql_migration_bad_behavior.rs` (new)
- migration-focused fixtures under `crates/jankurai/tests/fixtures/language_bad_behavior/sql/`
- required version/release files for `0.8.14`
- generated audit artifacts only through documented commands

**Intended validation:**
- `cargo test -p jankurai --test sql_migration_bad_behavior`
- `cargo test -p jankurai --test language_bad_behavior`
- `cargo test -p jankurai --test migration_safety_audit_smoke`
- `cargo run -p jankurai -- versions`
- `cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md`
- broader lanes from the handoff plan as time and failures allow

**Coordination note:** I will not hand-edit generated conformance paper output; if it remains changed, I will inspect and either regenerate with the documented command or leave a receipt explaining provenance.

---

## Codex (GPT-5) — FINAL RECEIPT

**Completed work:**
- Hardened HLT-021 destructive migration evidence so comments alone no longer suppress it.
- Split migration-specific SQL logic into `crates/jankurai/src/audit/language_rules/sql_migration.rs`.
- Added focused SQL migration tests and fixtures.
- Bumped release bindings to `0.8.14` while keeping `standard_version = 0.8.0`, `schema_version = 1.6.1`, and `paper_edition = 2026.05-ed8`.
- Regenerated conformance and paper outputs through their documented commands.

**Validation passed:**
- `cargo test -p jankurai --test sql_migration_bad_behavior`
- `cargo test -p jankurai --test language_bad_behavior`
- `cargo test -p jankurai --test migration_safety_audit_smoke`
- `cargo run -p jankurai -- versions`
- `cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md`
- `just fast`
- `cargo test -p jankurai`
- `just check`
- `just conformance`

**Git:**
- Release commit on `main`: `f522380` (`docs: refresh paper pdf after release validation`)
- Release source commit: `e99a7be` (`feat(audit): migration detectors + BAD_MIGRATION.md — v0.8.14`)
- Tag: `v0.8.14`
- Push: `origin/main` and `origin/v0.8.14`

**Remaining local noise:**
- `agent/score-history.csv`
- `agent/score-history.jsonl`

---
