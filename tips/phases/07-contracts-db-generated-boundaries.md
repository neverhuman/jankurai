# Phase 07: Contracts DB And Generated Boundaries

Status: hardened
Owner: standard
Last reviewed: 2026-05-03
Parallel MCP candidate: yes

## Objective

Make boundary truth executable. Jankurai must enforce that public APIs, event schemas, generated clients, database migrations, and durable invariants are declared, generated, tested, and owned.

The exit state is a contracts and DB lane that makes handwritten drift, wrong-layer persistence, and unsafe migrations visible.

## Current State

Existing policy:

- `docs/agent-native-standard.md` defines contracts, generated zones, DB truth, and layer boundaries.
- `agent/generated-zones.toml` exists.
- `agent/boundaries.toml` exists.
- Audit detects generated-zone risks, contract surfaces, direct DB wrong-layer hits, destructive SQL hits, and streaming runtime drift.
- **`jankurai doctor`** validates committed **`agent/boundaries.toml`** against **`schemas/boundaries.schema.json`** (TOML parsed and checked as JSON-shaped instance). **`ArtifactSchema::Boundaries`** in Rust; tests in `crates/jankurai/tests/boundaries_manifest_smoke.rs`.
- **`jankurai audit`** ingests the same manifest when present and schema-valid: **`boundaries.artifact`** holds a compact summary (path, content fingerprint, stack id/version, queue path counts, streaming-exception count). Invalid or missing files leave **`artifact`** omitted. Implementation in `crates/jankurai/src/audit/boundaries_artifact.rs`; tests in `crates/jankurai/tests/boundaries_audit_ingest_smoke.rs`; **`schemas/repo-score.schema.json`** documents **`boundaries`**. Score caps unchanged. **Rendered output:** `crates/jankurai/src/render.rs` and `crates/jankurai/src/report/github.rs` surface the same digest in **`repo-score.md`** and CI step summaries (see workstream 5 / slice 3).
- **`jankurai prove`** may record optional **`boundaries_manifest_path`** (`agent/boundaries.toml`) on **`evidence-index.json`** when that file exists (`schemas/evidence-index.schema.json`, `crates/jankurai/src/commands/proof.rs`).
- **`jankurai audit`** (Phase 07 slice 4) requires every **`[[zone]]`** in **`agent/generated-zones.toml`** to declare non-empty **`path`**, **`source`**, and **`command`** (trimmed). Incomplete rows yield one aggregated **HLT-002-GENERATED-MUTATION** finding on the manifest path and count toward the **`generated-zone-mutation-risk`** cap. Implementation in **`crates/jankurai/src/audit/scan.rs`** (`generated_zone_manifest_metadata_issues`); tests in **`crates/jankurai/tests/generated_zones_manifest_smoke.rs`**.
- **`jankurai audit`** (Phase 07 slice 5) scans SQL under migration roots (including **`db/`**, paths containing **`/db/migrations/`** or **`/db/constraints/`**, manifest **`[db]`** `migration_paths` / `root_paths` / `constraint_paths`, and legacy **`migrations/`** / **`apps/api/migrations/`** / **`crates/adapters/`** trees) for destructive statements unless the file documents safety (rollback, down migration, backfill, lock timeout / advisory lock, staged deploy, expand-contract markers, or **`jankurai:migration-safe`**). Hits emit **`HLT-021-DESTRUCTIVE-MIGRATION`** (cap bucket **`destructive-migration-risk`**). Routing: **`agent/test-map.json`** prefix **`db/migrations/`** and proof lane **`db-migration-analyze`** in **`agent/proof-lanes.toml`**.
- **`jankurai audit`** (Phase 07 slice 5b) computes **`destructive_sql_hits` once** per run, passes the presence bit into **`caps_applied`**, and threads hits into **`build_findings`** so caps and findings stay consistent without redundant scans. Extended tests in **`crates/jankurai/tests/migration_safety_audit_smoke.rs`** (cap, **`RepoCatalog`** lane resolution, TRUNCATE, marker suppression, golden example).
- **`jankurai audit`** (Phase 07 slice 5c) refines **`delete without where`**: if **`WHERE`** begins a later executable line within a short lookahead, the migration is not treated as unbounded delete noise for **HLT-021**. Implementation in **`crates/jankurai/src/audit/scan.rs`** (`delete_has_where_on_following_lines`); regression in **`migration_safety_audit_smoke.rs`**; **`docs/testing.md`** limitations updated.
- **`jankurai audit`** (Phase 07 slice 6 / B1) enriches SARIF: repo-relative **`helpUri`** values are expanded to **`https://github.com/jeppsontaylor/jankurai/blob/main/...`**; each result **`region`** has **`startLine`**, **`endLine`**, and a **`snippet`** (first evidence line, truncated, or a short **`problem`** excerpt). Implementation in **`crates/jankurai/src/report/sarif.rs`**; regressions in **`crates/jankurai/tests/audit_smoke.rs`** and **`migration_safety_audit_smoke.rs`** (HLT-021).
- **`jankurai audit`** (Phase 07 slice 7) keeps the contract-drift cap honest with a regression that proves a contract surface without generated contracts or drift checks yields **`generated-contracts-or-public-api-drift-untested`** / **`HLT-007-HANDWRITTEN-CONTRACT`**. Test in **`crates/jankurai/tests/audit_smoke.rs`**.

Gaps (all closed by hardening slices H1–H5):

- ~~Contract parsing and diffing are limited beyond boundary manifest shape~~ — **H1** adds `scan::contract_source_hits` which detects contract files under `contracts/` without matching `[[zone]]` entries, emitting HLT-007.
- ~~Generated-zone reproducibility is not fully enforced~~ — **H2** adds `scan::generated_zone_existence_hits` which verifies declared zone files exist on disk and carry `Generated by:` / `DO NOT EDIT` headers, emitting HLT-002.
- DB migration safety: **Phase 07 slices 5–5c** add **`HLT-021-DESTRUCTIVE-MIGRATION`**, file-level safety markers, **`db-migration-analyze`** routing, **`destructive-migration-risk`** cap tied to a **single** `destructive_sql_hits` pass per audit, multi-line **`DELETE`/`WHERE`** lookahead (5c), and tests for cap, lane resolution, TRUNCATE, `jankurai:migration-safe`, and the **`examples/perfect-web-api-db`** fixture. **Slice 6 / B1** adds SARIF **`helpUri`** expansion and **`snippet`**/**`endLine`** parity. **H3** deduplicates SARIF `rules[]` array with `ruleIndex` references. SQLx-aware checks and procedural-SQL are documented limitations (line-scoped scanner + `jankurai:migration-safe` escape is the v0 credible surface).
- ~~Event contract boundaries are policy-level, not deeply validated~~ — **H4** adds `scan::event_contract_path_hits` which validates `agent/boundaries.toml` `[queues] event_contract_paths` exist on disk, emitting HLT-007.

## Dependencies

Requires Phase 01 stabilization and benefits strongly from Phase 02 semantic oracle.

Benefits from Phase 03 proof receipts.

## Public Interface Changes

Potential commands:

```bash
jankurai contracts --check
jankurai db --check
jankurai generated --check
```

If separate commands are too much, implement as audit dimensions first.

Contract policy fields:

- source contract paths
- generated output paths
- generator command
- compatibility command
- breaking-change policy
- owner
- lane

DB policy fields:

- migration paths
- rollback policy
- destructive-change policy
- schema drift command
- query check command
- RLS policy requirement
- PII classification requirement

## Workstreams

### 1. Contract Source Detection

Implementation tasks:

- Detect OpenAPI, JSON Schema, Protobuf, and TypeSpec source paths.
- Detect generated clients under declared generated zones.
- Detect handwritten DTO/client drift in TypeScript and Python surfaces.
- Detect contract source changes without generated output or proof lane mapping.
- Add fixtures for REST and protobuf cases.

Acceptance:

- Public API surfaces without contracts are findings.
- Handwritten API client mirrors are findings.
- Generated clients are protected from hand edits.

### 2. Generated Zone Reproducibility

Implementation tasks:

- Enforce generated file metadata where policy applies:
  - generator
  - source
  - command
  - do-not-edit marker
- Add checksum or manifest support if practical.
- Add `doctor` or audit checks for generated zones with missing source command.
- **Slice 4 (done):** Audit rejects zone rows with empty `path` / `source` / `command` (same cap bucket as other generated-zone risk). Deeper doctor checksum work remains open.

Acceptance:

- Generated files tell agents exactly what source to edit and command to run.
- Missing generator metadata is actionable.

### 3. DB Migration Safety

Implementation tasks:

- Parse or scan migrations for destructive operations.
- Require rollback/backfill/lock-timeout/staged-deploy evidence for destructive migrations.
- Detect raw SQL outside allowed paths.
- Detect product invariants that appear app-only where DB constraints should exist, initially as advisory.
- Support SQLx check command detection where Rust/SQLx is used.

Acceptance:

- `DROP`, `TRUNCATE`, unbounded `DELETE`, and dangerous `ALTER` produce high-confidence findings unless exception evidence exists.
- Wrong-layer DB access points to adapters/db repair.
- DB proof lane appears in proof routing.
- **Phase 07 slice 5:** destructive SQL in migration trees yields **HLT-021-DESTRUCTIVE-MIGRATION** unless the migration file documents safety (rollback / down / backfill / lock / staged / expand-contract / `jankurai:migration-safe`); **`db/migrations/`** is routed to **`db-migration-analyze`** (`jankurai migrate . --analyze --json target/jankurai/migration-report.json`).
- **Phase 07 slice 5b:** **`destructive-migration-risk`** cap is covered by tests together with HLT-021; **`RepoCatalog`** proves **`db/migrations/`** resolves to **`db-migration-analyze`**; golden **`examples/perfect-web-api-db`** stays free of HLT-021; audit uses one **`destructive_sql_hits`** scan per run.
- **Phase 07 slice 5c:** multi-line **`DELETE`/`WHERE`** lookahead so bounded deletes split across lines do not false-positive **HLT-021**; regression in **`migration_safety_audit_smoke`**; **`docs/testing.md`** limitations updated.
- **Phase 07 slice 6 / B1:** SARIF **`helpUri`** uses absolute **`blob/main/...`** URLs for repo-relative rule docs; results carry **`snippet`** (evidence or **`problem`** excerpt) and **`endLine`**; tests in **`audit_smoke`** and **`migration_safety_audit_smoke`** (HLT-021).

### 4. Event And Streaming Contracts

Implementation tasks:

- Enforce that event schemas live under `contracts/`.
- Enforce broker clients live under declared queue adapter paths.
- Require Kafka brownfield exceptions to include owner, expiry, reason/classification, and migration path.
- Add docs and fixtures for Kafka/Tansu/Iggy/Fluvio/NATS/Redis Streams markers.

Acceptance:

- `HLT-019-STREAMING-RUNTIME-DRIFT` remains stable and better evidenced.
- Streaming findings route to adapters/queue owners.

### 5. Contract And DB Report Integration

Implementation tasks:

- Add dimension evidence for contracts and DB truth.
- Add proof receipt links for contract and DB lanes.
- Add SARIF locations for contract and migration findings.
- Boundary manifest digest on **`repo-score`** JSON and evidence-index companion path (slice 2: audit ingest + **`boundaries_manifest_path`** when file exists).
- **Slice 3:** Markdown report (`render_markdown`) emits **`## Boundary manifest (ingested)`** when **`boundaries.artifact`** is present; GitHub step summary **`#### lane artifacts`** includes a **`boundaries`** line and appears when **only** boundaries ingest exists (truncated **`sha256:`** fingerprint in summary).

Acceptance:

- Contract and DB failures show in JSON, Markdown, SARIF, and repair queue.
- Findings include proof command and docs URL.

## Parallel MCP Breakdown

Strong parallel candidate:

- Agent A: contract source/generated detection. Owns contract modules and fixtures.
- Agent B: DB migration safety. Owns SQL/DB checks and fixtures.
- Agent C: streaming/event boundaries. Owns streaming checks and docs.
- Agent D: report integration. Owns rendering and schema updates.

Merge order:

1. Shared policy/schema fields.
2. Contract and DB analyzers.
3. Streaming analyzer.
4. Report integration and docs.

## Validation

Minimum:

```bash
cargo test -p jankurai
just fast
```

Fixture validation should include:

- contract source changed without generated output
- generated file without metadata
- destructive migration without evidence
- DB access from wrong layer
- streaming client outside adapter

## Risks

- Inferring app-only invariants can be noisy. Start advisory.
- Contract tooling varies; Jankurai should support generic source/generator metadata before choosing one generator.
- Migration safety needs careful exception policy to avoid blocking legitimate changes without a path forward.

## Handoff Notes

Leave:

- supported contract formats
- generated metadata requirements
- migration finding examples
- exception examples
- proof lane mappings
- known limitations

## Phase Status Receipt

- Phase status: complete contracts, DB, and generated boundaries; **doctor** validates boundary manifest; **audit** surfaces validated manifest digest on **`repo-score`** JSON; **Markdown audit output** and **GitHub step summaries** include **`## Boundary manifest (ingested)`** / lane-artifacts **`boundaries`** line when **`boundaries.artifact`** is present; **prove** may add **`boundaries_manifest_path`** on evidence index
- Operational handoff: [`tips/phases/logs/07-contracts-db-generated-boundaries.log`](logs/07-contracts-db-generated-boundaries.log) (append-only)
- Files changed (slice 1): `schemas/boundaries.schema.json`, `crates/jankurai/src/validation.rs`, `crates/jankurai/src/commands/doctor.rs`, `crates/jankurai/tests/boundaries_manifest_smoke.rs`, `crates/jankurai/tests/init_doctor.rs`, `crates/jankurai/tests/schema_contracts.rs`, `docs/moonshot.md`, `docs/testing.md`, `tips/phases/07-contracts-db-generated-boundaries.md`, `tips/phases/logs/README.txt`
- Files changed (slice 2): `crates/jankurai/src/model.rs`, `crates/jankurai/src/audit/boundaries_artifact.rs`, `crates/jankurai/src/audit/mod.rs`, `schemas/repo-score.schema.json`, `schemas/evidence-index.schema.json`, `crates/jankurai/src/commands/proof.rs`, `crates/jankurai/tests/boundaries_audit_ingest_smoke.rs`, `crates/jankurai/tests/schema_contracts.rs`, `crates/jankurai/tests/proof_surface_smoke.rs`, phase doc + log
- Files changed (slice 3): `crates/jankurai/src/render.rs`, `crates/jankurai/src/report/github.rs`, `crates/jankurai/tests/render_lane_artifacts_smoke.rs`, phase doc + log
- Files changed (slice 4): `crates/jankurai/src/audit/scan.rs`, `crates/jankurai/src/audit/mod.rs`, `crates/jankurai/src/audit/caps.rs`, `crates/jankurai/tests/generated_zones_manifest_smoke.rs`, `crates/jankurai/src/commands/migrate.rs`, `schemas/migration-report.schema.json`, `schemas/migration-plan.schema.json`, phase doc + log
- Files changed (slice 5): `crates/jankurai/src/audit/scan.rs`, `crates/jankurai/src/audit/mod.rs`, `crates/jankurai/src/audit/rules.rs`, `crates/jankurai/src/audit/finding_builder.rs`, `crates/jankurai/src/boundaries/sql.rs`, `crates/jankurai/src/commands/repair_plan.rs`, `crates/jankurai/src/commands/context_pack.rs`, `agent/proof-lanes.toml`, `agent/test-map.json`, `crates/jankurai/tests/migration_safety_audit_smoke.rs`, `crates/jankurai/tests/rule_registry_smoke.rs`, `crates/jankurai/tests/render_lane_artifacts_smoke.rs` (visual-baseline count expectations), phase doc + log
- Files changed (slice 5c): `crates/jankurai/src/audit/scan.rs`, `crates/jankurai/tests/migration_safety_audit_smoke.rs`, `docs/testing.md`, phase doc + log
- Files changed (slice 6): `crates/jankurai/src/report/sarif.rs`, `crates/jankurai/tests/audit_smoke.rs`, `crates/jankurai/tests/migration_safety_audit_smoke.rs`, `docs/testing.md`, phase doc + log
- Files changed (slice 7): `crates/jankurai/tests/audit_smoke.rs`, `tips/phases/07-contracts-db-generated-boundaries.md`, `tips/phases/logs/07-contracts-db-generated-boundaries.log`
- Schemas changed: `boundaries.schema.json` (slice 1); `repo-score.schema.json`, `evidence-index.schema.json` (slice 2); `migration-report.schema.json`, `migration-plan.schema.json` (slice 4: command/status envelope)
- Public interfaces changed: `ArtifactSchema::Boundaries`, `validation::validate_boundaries_toml_text`, doctor **`boundaries-manifest-schema`**; repo-score **`boundaries.artifact`**; evidence index **`boundaries_manifest_path`**; audit rule **`HLT-021-DESTRUCTIVE-MIGRATION`**
- Generated artifacts: none
- Routing maps changed: **`agent/test-map.json`** (`db/migrations/`); **`agent/proof-lanes.toml`** (`db-migration-analyze`)
- Validation commands: `cargo test -p jankurai`, `cargo run -p jankurai -- lane . --changed crates/jankurai/tests/audit_smoke.rs --out target/jankurai/p07-contract-cap-lane.json --md target/jankurai/p07-contract-cap-lane.md`, `just fast`
- Results: validation passed; DB enforcement remains partial; contract-drift cap now has an explicit regression; SARIF uses absolute **helpUri** for rule docs, **endLine**, and **snippet** (evidence or **problem** excerpt)
- Skipped validation: none
- Exceptions created: none
- Follow-up phases: 09 reference product platform, 10 reuse registry certified cells, 11 migration engine
