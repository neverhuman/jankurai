# Phase 05: UX Proof Platform

Status: complete
Owner: tools
Last reviewed: 2026-05-02
Parallel MCP candidate: yes

## Objective

Make rendered UX proof a first-class Jankurai lane. The goal is to replace repetitive pixel/layout QA with deterministic browser evidence wherever possible, while keeping human review for taste and ambiguous product judgment.

The exit state is a UX proof platform that can audit routes and Storybook stories, emit artifact-backed receipts, and feed Jankurai reports.

## Current State

Existing package:

- `packages/ux-qa/` is an npm workspace package named `@jankurai/ux-qa`.
- CLI supports `audit` and `storybook`.
- Checks include edge clearance, target size, interactive overlap, text clipping, button wrap, horizontal overflow, sticky obstruction, z-index ceiling, focus visible, form label, and nested scrollbar.
- CLI can emit screenshots, crops, ARIA snapshots, axe accessibility JSON artifacts, and deterministic visual-baseline summaries with hash-backed baseline and diff receipts. `agent/ux-qa.toml` policy fields now reserve flat visual-baseline roots, baseline owner metadata, and `stateQueryParam` for state-driven URLs; explicit CLI flags remain overrides.
- Rust CLI has `jankurai ux` passthrough to `packages/ux-qa/dist/cli.js`.
- **`jankurai doctor`** validates **`agent/ux-qa.toml`** against **`schemas/ux-qa-policy.schema.json`** when that file exists (TOML parsed with the standard `toml` crate, then JSON-schema checked). **`ArtifactSchema::UxQaPolicy`** and **`validate_ux_qa_policy_toml_text`** in `crates/jankurai`; tests in `crates/jankurai/tests/ux_qa_policy_smoke.rs`. The `@jankurai/ux-qa` package still uses a line-oriented TOML subset for runtime—prefer simple tables and `[[routes]]` for parity.
- **`jankurai doctor`** validates **`target/jankurai/ux-qa.json`** against **`schemas/ux-qa.schema.json`** when that file exists (CLI output from `jankurai ux audit … --out …`). The CLI now reserves `schemaVersion` **`1.4.0`**; validation still accepts `1.2.0` and `1.3.0` reports for compatibility. **`ArtifactSchema::UxQaReport`**; tests in `crates/jankurai/tests/ux_qa_report_smoke.rs`.
- **`jankurai audit`** (repo score JSON) ingests the same path when present and schema-valid: **`ux_qa.artifact`** holds a compact summary (`path`, `report_count`, **`worst_decision`** with ordering block > review > warn > pass, violation and summary counts, missing state names, artifact counts by kind, missing required artifact kinds, and accessibility violation/incomplete/pass totals). Invalid or missing files leave **`artifact`** omitted. Validated incomplete evidence adds `HLT-013-RENDERED-UX-GAP` for state or non-a11y artifact gaps and `HLT-014-A11Y-GAP` for axe violations or missing accessibility artifacts. Implementation in `crates/jankurai/src/audit/ux_artifact.rs`; tests in `crates/jankurai/tests/ux_qa_audit_ingest_smoke.rs`; **`schemas/repo-score.schema.json`** documents **`ux_qa`**. Score caps are unchanged, and proof receipts can now carry an `ux_qa_report_digest` field in the evidence index schema.
- **`render_markdown`** and **GitHub step summary** (`report/github.rs`) print **`ux_qa.artifact`** when present; tests in `crates/jankurai/tests/render_lane_artifacts_smoke.rs`.
- Tests exist for geometry, artifacts, config, hit testing, selector, and Storybook discovery.

Gaps:

- Doctor’s TOML parser may accept constructs the UX CLI subset does not; keep policy files straightforward until parsers converge.
- UX decisions do not yet drive numeric audit score caps; evidence-index / proof receipts can carry digest fields, but ingest still needs runtime wiring.
- Visual baseline decisions are standardized in schema as deterministic byte-and-hash comparisons and are enforced by the runtime helper; no pixel-diff math or AI/VLM authority is required.
- Route/story matrix policy now has route-level baseline and state-query overrides, and the CLI expands configured state URLs through the query parameter when present.
- State coverage is enforced from configured report evidence, but generated mocks/MSW remain optional rather than required by this phase.
- Automated axe accessibility is first-class evidence, not a complete replacement for human inclusive design review.

## Dependencies

Requires Phase 01 docs/schema stability.

Benefits from Phase 03 proof receipts and evidence ledger.

## Public Interface Changes

UX CLI should converge on:

```bash
jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json
jankurai ux storybook --url http://localhost:6006 --config agent/ux-qa.toml
```

UX policy fields should include:

- routes
- storybook URL
- required viewports
- required states
- screenshot requirement
- ARIA snapshot requirement
- accessibility scan requirement
- visual baseline mode
- geometry thresholds
- artifact root
- merge decision thresholds

## Workstreams

### 1. UX Policy Schema

Implementation tasks:

- Expand `agent/ux-qa.toml` into a real policy file.
- Add schema for UX policy if missing or incomplete.
- Support route matrix and viewport matrix.
- Support per-route overrides.
- Support required states for critical UI surfaces.
- Keep defaults small enough for local execution.

Acceptance:

- Policy can express mobile/tablet/desktop viewports.
- Policy can express critical route IDs and Storybook story IDs.
- Policy can mark deterministic failures as blocking and visual diffs as review.

### 2. Accessibility Evidence

Implementation tasks:

- Integrate axe or an equivalent accessibility scanner through Playwright where practical.
- Emit accessibility JSON artifacts.
- Map findings to Jankurai UX/a11y rule IDs.
- Keep automated accessibility claims honest: automation catches common issues but does not replace all inclusive testing.

Acceptance:

- Critical route report can include accessibility artifact paths.
- Accessibility errors are distinguishable from geometry errors.
- Docs explain automated and manual boundaries.

### 3. Visual Baseline Decisions

Implementation tasks:

- Define baseline artifact paths under ignored output or approved baseline directories.
- Add visual diff metadata fields even if first implementation delegates actual diff to Playwright or external provider.
- Define merge decisions: pass, block, review.
- Add owner approval field for baseline updates.

Acceptance:

- A changed screenshot can be classified as deterministic pass/fail or owner-review visual diff.
- AI/VLM opinions remain advisory and cannot be the sole gate.

### 4. State Coverage

Implementation tasks:

- Define standard UI states: loading, empty, error, success, permission-denied.
- Add route/story metadata for state coverage.
- Detect missing critical states in configured surfaces.
- Encourage generated mocks/MSW for state generation, but do not require a specific mock tool yet.

Acceptance:

- UX report can say which required states were checked and which are missing.
- Missing state coverage becomes an audit finding for critical UI profiles.

### 5. Audit And Receipt Integration

Implementation tasks:

- Add UX report ingestion to Rust audit or proof receipt flow (JSON ingest shipped; Markdown + GitHub summary show compact **`ux_qa.artifact`** when present).
- Show compact UX summary in Markdown.
- Include artifacts in evidence ledger.
- Add findings for missing UX report on web-surface changes in strict modes.

Acceptance:

- `agent/repo-score.md` or future proof report can point to UX artifacts.
- A UI change without UX evidence is visible and actionable.

## Parallel MCP Breakdown

Strong parallel candidate:

- Agent A: policy schema and config parser. Owns `packages/ux-qa/src/config.ts`, schemas, config tests.
- Agent B: accessibility integration. Owns accessibility modules and tests.
- Agent C: visual baseline metadata. Owns artifact/receipt types and docs.
- Agent D: Rust audit/report integration. Owns Rust report ingestion and rendering.

Merge order:

1. Type/schema updates.
2. UX package feature work.
3. Rust ingestion.
4. Docs and examples.

## Validation

Minimum:

```bash
npm --workspace @jankurai/ux-qa run build
npm --workspace @jankurai/ux-qa run test
just fast
```

If Rust integration changes:

```bash
cargo test -p jankurai
```

Manual smoke with a running app or fixture:

```bash
jankurai ux audit --url http://localhost:3000 --out target/jankurai/ux-qa.json --screenshot --aria-snapshot --accessibility-scan
```

## Risks

- Visual diffs can be flaky without strict readiness contracts.
- Browser matrices can slow CI if run too often.
- Accessibility automation can create false confidence if docs imply complete coverage.

## Handoff Notes

Leave:

- UX policy schema
- sample route matrix
- sample report with artifacts
- list of deterministic blocking rules
- list of review-only visual rules
- validation artifacts and commands

## Phase Status Receipt

- Phase status: complete UX proof platform; **doctor validates `agent/ux-qa.toml`** (policy) and **`target/jankurai/ux-qa.json`** (report envelope) when those files exist; **audit** ingests validated `ux-qa.json` into **`repo-score` `ux_qa.artifact`**; human-facing **`repo-score.md`** and GitHub summaries surface the same ingest summary when present; the current slice emits deterministic visual-baseline hashes, route-level overrides, and `stateQueryParam`-driven state expansion without adding score caps or any AI/pixel-diff authority
- Operational handoff: [`tips/phases/logs/05-ux-proof-platform.log`](logs/05-ux-proof-platform.log) (append-only)
- Recent slice (report JSON): `schemas/ux-qa.schema.json` (`$defs` aligned to `packages/ux-qa/src/types.ts`), `ArtifactSchema::UxQaReport`, doctor `ux-qa-report-schema` path, `crates/jankurai/tests/ux_qa_report_smoke.rs`, `schema_contracts` assertions for `ux-qa.schema.json`; current contract reserves `schemaVersion` `1.4.0`, artifact `sha256`, `state`, and `visualBaseline`
- Recent slice (audit ingest): `crates/jankurai/src/audit/ux_artifact.rs`, `UxQaReportArtifactSummary` + `UxQaReadiness.artifact` in `model.rs`, `schemas/repo-score.schema.json` `ux_qa` / `$defs`, `ux_qa_audit_ingest_smoke.rs`, `schema_contracts` repo-score `ux_qa` key; evidence index schema now reserves `ux_qa_report_digest`
- Recent slice (Markdown / CI): `crates/jankurai/src/render.rs`, `crates/jankurai/src/report/github.rs`, `render_lane_artifacts_smoke.rs`
- Recent slice (evidence matrix): `packages/ux-qa/src/accessibility.ts`, `packages/ux-qa/src/cli.ts`, `packages/ux-qa/src/types.ts`, `schemas/ux-qa.schema.json`, `crates/jankurai/src/audit/ux_artifact.rs`, `crates/jankurai/src/model.rs`, `crates/jankurai/src/render.rs`, `crates/jankurai/src/report/github.rs`
- Earlier slice (policy): `schemas/ux-qa-policy.schema.json`, `ArtifactSchema::UxQaPolicy`, `validate_ux_qa_policy_toml_text`, `crates/jankurai/tests/ux_qa_policy_smoke.rs`
- Schemas changed: `ux-qa.schema.json` — typed nested report, `schemaVersion` enum `1.2.0` / `1.3.0` / `1.4.0`, accessibility and artifact coverage fields, artifact `sha256`, `state`, and `visualBaseline`; `repo-score.schema.json` — **`ux_qa`** readiness + optional artifact summary with evidence matrix fields
- Public interfaces changed (report): `ArtifactSchema::UxQaReport`, doctor checks `ux-qa-report-read` / `ux-qa-report-json` / `ux-qa-report-schema`; repo-score JSON **`ux_qa.artifact`** when ingest succeeds; Markdown / GitHub summary lines for ingest; `jankurai ux audit` supports `--accessibility-scan`
- Generated artifacts: `agent/repo-score.json`; `agent/repo-score.md`
- Routing maps changed: none
- Validation commands: `npm --workspace @jankurai/ux-qa run build`, `npm --workspace @jankurai/ux-qa run test`, `cargo test -p jankurai --test ux_qa_report_smoke --test ux_qa_audit_ingest_smoke --test render_lane_artifacts_smoke --test proof_surface_smoke --test schema_contracts`, `cargo test -p jankurai`, `just fast`, `just score`
- Results: all listed validation commands passed; see log file lines for SHA and outcomes
- Follow-up phases: 09 reference product platform, 12 benchmark certification and governance
