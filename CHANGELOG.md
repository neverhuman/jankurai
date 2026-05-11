# Changelog

All notable user-facing changes should be recorded here.

Jankurai is 1.0. Public CLI behavior, report schemas, generated scaffold paths, and agent-facing contracts should still receive compatibility notes when they change.

## Unreleased

No user-facing changes yet.

## 1.0.0 - 2026-05-11

### Fixed

- Switched advisory coverage sources for Rust line coverage and mutation evidence into optional auto mode so clean repos do not emit false `HLT-008-FALSE-GREEN-RISK` findings when those artifacts are absent.
- Added checked-in Tuiwright audit fixtures for a direct-flow positive case and a helper-wrapped negative case so the audit smoke test can prove gap-versus-full coverage behavior deterministically.

### Changed

- Bumped the auditor/action release to `1.0.0`, updated the GitHub Action reference, and wired the coverage audit into the fast, score, and CI lanes before the final repo audit runs.

## 0.8.16 - 2026-05-11

Issue #3 accepted scope: ship no-write migration evidence commands that help agents verify migration prompt claims and preflight selected migration slices before implementation. Deferred items remain out of scope: `jankurai ai audit`, full call-site inventory, postmortem feedback-loop automation, traffic mirroring, and cutover automation.

### Added

- `jankurai migrate verify-prompt <doc>` for schema-backed, no-write prompt claim verification before agents edit code. The command verifies `path:line`, `module::symbol`, class/base, and LLM-call claims, writes JSON/Markdown evidence, and remains advisory by default.
- `jankurai migrate slice-risk --plan <plan.json> --slice-id <id>` for static preflight of the selected migration slice. The command scans selected slice metadata and selected slice files, emits risk signals, and writes JSON/Markdown evidence without executing cutover behavior.
- `schemas/migration-prompt-verification.schema.json` and `schemas/migration-slice-risk.schema.json` for the new migration evidence envelopes under schema version `1.7.0`.
- CI now runs the migration prompt-verification and slice-risk fixture tests by name after the language bad-behavior fixture lane.

### Fixed

- Hardened prompt evidence path handling so missing, bad, traversal, unreadable, non-text, directory, and repo-escaping symlink paths become claim-level invalid results instead of command-wide crashes or unsafe reads.
- Tightened prompt evidence heuristics so ambiguous module evidence, broad symbol-only matches, comment/string-only evidence, Rust-like class/base uncertainty, and multiple LLM call sites become `review` instead of false certainty.
- Scoped `slice-risk` to selected slice files when `allowed_paths` are present. Missing selected paths now produce `slice-path-missing` review signals instead of falling back to whole-repo noise.
- Split signing/HMAC handling so hardcoded source-level secret/signing behavior remains high/blocking while prose prerequisites and env-presence checks remain review-only. `--check-env` records presence only and never prints values.

### Changed

- Bumped the auditor/action package release to `0.8.16`; standard compatibility remains `0.8.0`, report schema remains `1.7.0`, and paper edition remains `2026.05-ed8`.
- Updated release surfaces across `VERSION`, Cargo metadata, the installed template manifest, UX package metadata, README action examples, version tests, and changelog references.

## 0.8.14 - 2026-05-10

### Added

- `docs/BAD_MIGRATION.md`: canonical migration anti-pattern reference covering immediate rejections, expand/contract failures, lock recklessness, PostgreSQL/SQLite hazards, backfill failures, and ORM/AI dangers.
- `sql.migration.concurrent-in-txn` (HLT-030): fires on `CREATE INDEX CONCURRENTLY` inside `BEGIN`/`COMMIT` in migration files — PostgreSQL silently degrades CONCURRENTLY to a blocking build inside a transaction.
- `sql.migration.missing-lock-timeout` (HLT-030): fires on risky `ALTER TABLE` DDL without `lock_timeout`/`statement_timeout` in migration files.
- `sql.migration.cascade-convenience` (HLT-030): fires on `DROP`/`TRUNCATE CASCADE` without a structured dependency inventory in adjacent migration metadata.
- `sql.migration.blocking-index-create` (HLT-030): fires on `CREATE INDEX` without `CONCURRENTLY` in migration files — holds ACCESS EXCLUSIVE lock for the full index build duration.
- `sql.migration.not-valid-unvalidated` (HLT-030): fires when `NOT VALID` constraint has no `VALIDATE CONSTRAINT` in the same migration file — constraint enforces nothing until validated.
- All migration detectors gate on `is_migration_file_path()` and `.sql`/`.pgsql`/`.psql` extension — zero impact on repos without SQL or migration paths.

### Fixed

- `sql.migration.destructive-no-proof` false positive: bare `cascade` keyword (e.g., `ON DELETE CASCADE` in `CREATE TABLE`) no longer triggers the destructive-migration detector; scoped to `DROP`/`TRUNCATE` + `CASCADE` only.
- Added `DROP CONSTRAINT`, `DROP SCHEMA`, `DROP DATABASE` to destructive migration patterns.
- HLT-021 `docs_url` updated to `docs/BAD_MIGRATION.md`.
- HLT-030 `docs_url` updated to `docs/BAD_SQL.md`.

### Changed

- Bumped the auditor/action package release to `0.8.14`; standard compatibility remains `0.8.0`, report schema remains `1.6.1`, and paper edition remains `2026.05-ed8`.
- HLT-021 destructive migration suppression now requires structured adjacent metadata plus verify/check evidence. `jankurai:migration-safe`, "rollback", or other comment-only markers no longer suppress destructive migration findings.

## 0.8.13 - 2026-05-09

### Fixed

- HLT-001 placeholder/TODO patterns: bare `retry` substring no longer matches legitimate fields like `retry_after_seconds`; replaced with hostile-only phrases (`silent retry`, `unbounded retry`, `retry forever`). Bare `placeholder` substring no longer matches identifiers like `argumentSlots`; replaced with shape patterns (`// placeholder`, `# placeholder`, `placeholder!(`, `<placeholder>`). Short bare patterns (TODO/FIXME/HACK/XXX/stub) now require word boundaries (J1c).
- HLT-001 / HLT-008 / HLT-010 / HLT-011 / HLT-012 / HLT-023 / HLT-027 substring detectors in `audit/scan.rs` now consult `language_rules::common::nearby_allow`, so `// jankurai:allow HLT-XXX-NAME reason=... expires=YYYY-MM-DD` comments suppress the corresponding finding (J1d).
- HLT-010 secret assignment detection no longer flags bare identifier paths on the right-hand side (e.g. `api_key: model.api_key`); requires either a quoted literal (>= 8 char body) or a known high-entropy prefix from the existing strong-token list (J1e).
- HLT-016 supply-chain cap no longer treats a repo as high-risk when the only `package.json` / `Cargo.toml` / `go.mod` entries are gitignored runtime install directories (e.g. `.jekko/package.json`) (J1f).
- HLT-018 build-speed signals: dimension grants a +10 bonus when the command surface shows both an explicit cache marker (`turbo`, `nextest`, `just-cache`, `cargo --cached`, `sccache`) AND a narrow per-package target (`cargo check/test -p`, `cargo nextest run -p`, `vitest run`, `pytest -k`, `go test -run`), raising the score above the perf-concurrency cap when evidence is genuine (J1g).
- HLT-026 cost surface: `cost_budget_hits` now reads `agent/audit-policy.toml` and prefers explicit `[[cost_surface]]` declarations over the keyword-presence scan when the policy file enumerates them (J1g).
- HLT-021 / HLT-030 migration recognition extended to `packages/<name>/migration[s]/` and `apps/<name>/migration[s]/` paths (J1i).
- TypeScript / Rust / SQL language detectors honor the `[[zone]] path` list in `agent/generated-zones.toml`; declared zone paths are skipped from `is_typescript_surface` / `rust_files` / `is_sql_candidate` so HLT-029/030/031 no longer fires on outputs the manifest already declares as generated (J1b).
- `is_generated_or_reference_path` now treats `*.gen.{ts,tsx,js,mjs}` and `sst-env.d.ts` (anywhere in the tree) as generated regardless of directory depth (J1a).

### Changed

- Bumped the auditor/action package release to `0.8.13`; standard compatibility remains `0.8.0` and report schema is `1.6.1`.

### Notes

- HLT-008 per-crate proptest cap (J1h) was attempted but reverted because the existing `audit_repo_root_still_has_no_findings` smoke test relies on the legacy any-file marker logic; tightening to per-crate would require adding tests to `crates/tuiwright-cli/`. The rule remains untightened for v0.8.13.
- Conformance fixtures (J1j) for the new detector behaviors were not added in this release; existing unit tests under `audit::scan::tests` cover the regression checks for J1a/J1c/J1d/J1e.

## 0.8.12 - 2026-05-07

### Added

- Added certified reuse-registry cells for periodic cron jobs and billing subscriptions, including example Rust boundaries, OpenAPI contracts, migration/constraint evidence, docs, ops notes, UX route notes, schema coverage, and smoke tests.
- Added `jankurai version`, `jankurai versions`, `jankurai upgrade --score`, and update receipt schema coverage for version-aware local upgrades.

### Changed

- Bumped the auditor/action package release to `0.8.12`; standard compatibility remains `0.8.0` and report schema is `1.6.1`.
- Updated the release docs and version manifests to reflect `jankurai version`, `jankurai versions`, and `jankurai upgrade --score` behavior.
- Retagged the GitHub Action reference to `v0.8.12`.
- Replaced Tuiwright bitmap rendering with rusttype plus bundled JetBrains Mono for anti-aliased screenshots.

### Fixed

- Fixed Tuiwright missing Unicode box drawing glyphs in rendered output.
- Made line-based scaffold merges recipe-aware so `Justfile` updates do not append commands from already-existing recipes as orphan lines.
- Updated scaffold merge behavior so `agent/standard-version.toml` refreshes canonical version keys instead of keeping stale auditor/schema metadata.

## 0.8.11 - 2026-05-06

### Added

- Added reference-profile structure audit output and migration steering for detected canonical cells.
- Added `HLT-039-WEB-SECURITY-BAD-BEHAVIOR` with high-confidence detectors for exposed Vite dev servers, client-exposed Vite secrets, browser token storage, and credentialed wildcard CORS.
- Added `HLT-040-REPO-ROT-BAD-BEHAVIOR` with active-source old/backup/copy/archive path checks plus soft review signals for commented-out code blocks and hard-disabled branches.
- Added focused coverage for risky and safe web-security and repo-rot cases, including false-positive guards for docs, tips, reference, tests, generated output, API versions, and DB migrations.

### Changed

- Hardened `jankurai upgrade` for source-checkout upgrades: `--source auto` now prefers a newer local `crates/jankurai` package over registry lookup and reinstalls into the current Cargo root instead of a nested `bin` path.
- Bumped the auditor/action package release to `0.8.11`; standard compatibility remains `0.8.0` and report schema is `1.6.0`.

## 0.8.10 - 2026-05-06

### Added

- Added default audit inventory exclusion for `tips/`, plus user-configurable `[scan] excluded_paths` entries in `agent/audit-policy.toml`.
- Added bounded score history commands: `jankurai history latest/export/compact/restore`, plus bounded audit retention and optional mirror sink support.
- Added May 6 public-repository paper evidence, score tables, and a README score table for the `v0.8.8` Marketplace action release.
- Added accepted-baseline ratchet scaffolding and strict scoring-integrity smoke tests for fail-closed audit decisions.

### Changed

- Routed `jankurai score trend` through the shared score-history loader and added stable score-history entry/export schemas.
- Bumped the auditor/action package release to `0.8.10`; standard compatibility remains `0.8.0` and report schema remains `1.5.0`.
- Hardened CI scoring order, required proof/security evidence, SHA-pinned Actions usage, SARIF upload, and badge source routing for release readiness.
- Fixed the isolated empty-repository ratchet regression so `decision.ratchet.score_delta` is always emitted, including `--no-score-history` runs.
- Prepared the `v0.8.10` GitHub Marketplace action release for the hardened scoring-integrity lane.
- Scoped crates.io publication out of this Marketplace release until the proof crates are published first.

## 0.8.0 - 2026-05-05

### Added

- Added the GitTools bad-behavior policy surface, research note, detector family, fixtures, and stable `HLT-036-GITTOOLS-BAD-BEHAVIOR` rule.
- Added the `gittools-bad-behavior` hard cap for high-confidence hook-manager and Git tooling hazards.

### Changed

- Bumped the standard and auditor release to `0.8.0` and the paper edition to `2026.05-ed8`; report schema remains `1.5.0`.
- Reframed the paper around Jankurai as a versioned agent-native repository standard and bumped the paper edition to `2026.05-ed6`.
- Fixed generated adapter templates so every generated adapter satisfies the startup update marker verification and shows a valid client-start command.
- Fixed Marketplace action packaging so external consumers install the CLI from the action checkout, and documented `v0.8.0` GitHub Action usage, inputs, artifacts, and local runner behavior.

## 0.6.1 - 2026-05-04

### Changed

- Hardened vibe coverage taxonomy with reviewed canonical groups, detector/evidence status fields, and `0` uncovered source rows.
- Downgraded broad `absolute` claims to `partial` unless backed by detector and audit evidence.
- Strengthened `jankurai vibe validate` for title matching, duplicate/missing row checks, known rule/tool/lane references, reviewed rows, and absolute-evidence requirements.
- Added semantic coverage fixtures and HLT-022 through HLT-027 detector fixtures.
- Regenerated the paper coverage table with short rule labels and a separate legend.

## 0.6.0 - 2026-05-04

### Added

- Vibe coverage registry in `agent/vibe-coverage.toml` mapping all 260 `tips/vibe_coding` source rows.
- `jankurai vibe validate` and `jankurai vibe coverage` for JSON, Markdown, and generated TeX coverage reports.
- Optional repo-score `vibe_coverage` summary and stable `## Vibe Coding Coverage` Markdown section.
- Generated paper appendix table with green/yellow/red coverage status.
- Conditional MASTER_PLAN/phase adapter routing for explicit phase work only.

- v0.6.0 trustworthy-merge surface: `jankurai witness`, `jankurai score diff`, `jankurai score trend`, `jankurai rules export`, and `jankurai rules verify`.
- Merge witness, score diff/trend, rule registry, and rule-verify schemas.
- Token-budgeted context packs with source-trust labels and included/excluded file receipts.
- Baseline-required ratchet CI and audit behavior.
- Public `init --bootstrap-commit` and `--bootstrap-message` flags; hidden `--yolo` aliases remain for compatibility with deprecation warnings.
- Public open-source README structure with install, safe trial, adoption, update, AI-agent risk, support, security, license, and citation sections.
- Community health files for contributing, security, conduct, support, changelog, pull requests, and issues.
- Cargo package metadata for repository, homepage, README, keywords, and categories.
