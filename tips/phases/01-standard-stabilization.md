# Phase 01: Standard Stabilization

Status: hardened
Owner: standard
Last reviewed: 2026-05-03
Applies to: `tips/phases/01-standard-stabilization.md`

## Purpose

This file is the canonical router for Phase 01. It tells a future implementation agent what to read, what to own, what to avoid, which contracts must stay stable, how to split parallel work, and which evidence closes the phase.

This file does not implement Phase 01 itself. It is a docs-only execution plan for the implementation that follows.

Hardening receipt: the standard parity guard, schema keyword coverage, typed doctor diagnostics, policy-aware security evidence, and proof verification envelope are now implemented; the remaining sections below are historical planning context for the now-hardened phase.

## Phase 01 Goal

Stabilize the v0.4 foundation so the current jankurai release surface is coherent, versioned, and defensible before any later phase adds more behavior.

Phase 01 must:

- stabilize v0.4 foundations
- clarify schema and report contracts
- add compatibility guard expectations
- verify doctor and security lane reality
- separate implemented behavior from moonshot roadmap docs
- define release evidence conventions

## Read-First Ritual

Before starting any Phase 01 implementation, read these in order:

1. `agent/JANKURAI_STANDARD.md`
2. `docs/agent-native-standard.md`
3. `docs/moonshot.md`
4. `tips/phases/00-phase-index.md`
5. this file
6. `docs/release-plan.md`
7. `docs/audit-rubric.md`
8. `agent/owner-map.json`
9. `agent/test-map.json`
10. `agent/generated-zones.toml`
11. `agent/proof-lanes.toml`
12. `agent/boundaries.toml`
13. `agent/audit-policy.toml`
14. `agent/standard-version.toml`
15. `agent/ux-qa.toml`

## Current Baseline

As of 2026-05-02, the repo already has:

- Rust CLI package at `crates/jankurai/`
- `audit`, `init`, `doctor`, `ci install`, `issues export`, `explain`, `versions`, adapter sync/verify, and UX passthrough commands
- JSON, Markdown, SARIF, JUnit, GitHub summary, and repair queue exports
- machine-readable agent files under `agent/`
- schemas for the report and policy surfaces in `schemas/`
- UX QA runtime under `packages/ux-qa/`
- canonical standard, release, and mission docs
- `docs/moonshot.md` as the north star for the phase sequence
- phase files `00` through `13` under `tips/phases/`

Known Phase 01 gaps to verify or close:

- ~~repo-score JSON schema drift vs emitted `Report`~~ **closed 2026-05-02** (`schemas/repo-score.schema.json` + `audit_smoke::audit_report_serializes_against_repo_score_schema`)
- ~~schema coverage is incomplete for optional standalone JSON schemas for every agent TOML/JSON control file (doctor validates many surfaces; full enum coverage remains incremental)~~ **closed 2026-05-02** for `owner-map`, `test-map`, `generated-zones`, `proof-lanes`, `standard-version`, and `audit-policy` (`validation` helpers, `doctor` checks, `schema_contracts` fixture test)
- ~~report compatibility needs ongoing semantic tests when Markdown or auxiliary exports change shape~~ **baseline closed 2026-05-02** (`crates/jankurai/tests/report_compatibility_guard.rs`, `just compat`); extend assertions when SARIF/JUnit/summary/JSONL contracts evolve
- doctor and security lane diagnostics need to distinguish missing tools from broken repo state
- docs need a hard boundary between implemented behavior and roadmap-only claims
- release evidence paths need a convention that future agents can reuse
- new durable paths must be routed through owner/test maps when they are introduced
- this docs-only plan edit must not touch the paper

## Phase 01 Scope

Phase 01 implementation may touch:

- `crates/jankurai/` report, doctor, and validation code
- `schemas/` for additive or tightened compatibility surfaces
- `docs/` for boundary, release, and validation wording
- `agent/` for generated-zone, version, policy, and receipt metadata
- `Justfile` only if a validation lane or command needs explicit wiring
- generated outputs only through their source command, never by hand

Phase 01 must not:

- implement moonshot features
- change the default stack identity
- rename canonical report fields without a migration note
- move canonical score artifacts away from `agent/repo-score.json` and `agent/repo-score.md`
- add new required external tools to `just fast`
- edit `paper/` in this phase

## Non-Goals

- No new product features beyond Phase 01 contract stabilization.
- No paper updates.
- No broad repo cleanup unrelated to Phase 01.
- No generated-file hand edits.
- No implementation of later phases' moonshot ideas.
- No changes to `reference/`.

## Repository Constraints

- This docs-only plan edit changes only `tips/phases/01-standard-stabilization.md`.
- Stage all `tips/` material with Git before editing so untracked phase material is preserved.
- Do not update the paper or any generated artifact in this step.
- Do not hand-edit generated outputs.
- Do not update `agent/owner-map.json` or `agent/test-map.json` for this plan-only edit.
- Future Phase 01 implementation must update `agent/owner-map.json` and `agent/test-map.json` only when new durable paths are added.
- Keep `just fast` as the required validation for this docs-only plan edit.
- Treat the dirty worktree as project state; do not revert unrelated changes.
- Use `rtk`-prefixed commands for shell work in this repo.

## Dependency Inputs

Required docs:

- `docs/moonshot.md`
- `docs/agent-native-standard.md`
- `docs/release-plan.md`
- `docs/audit-rubric.md`
- `agent/JANKURAI_STANDARD.md`

Required schema surfaces:

- `schemas/audit-policy.schema.json`
- `schemas/boundaries.schema.json`
- `schemas/finding.schema.json`
- `schemas/repair-queue.schema.json`
- `schemas/repo-score.schema.json`
- `schemas/ux-qa.schema.json`

Required agent artifact inputs:

- `agent/owner-map.json`
- `agent/test-map.json`
- `agent/generated-zones.toml`
- `agent/proof-lanes.toml`
- `agent/boundaries.toml`
- `agent/audit-policy.toml`
- `agent/standard-version.toml`
- `agent/ux-qa.toml`

## Shared Contracts

Phase 01 stabilizes these shared contracts:

- report outputs: JSON, Markdown, SARIF, JUnit, GitHub summary, repair queue JSONL
- schema compatibility: additive or tightened validation only unless a migration note exists
- version fields: `standard_version`, `auditor_version`, `schema_version`, `paper_edition`, `target_stack`, `target_stack_id`
- routing metadata: owner map, test map, generated-zone manifest, proof lanes, boundary policy, audit policy, standard version, UX QA policy
- release evidence: canonical score artifacts, doctor output, security-lane output, compatibility tests, and the phase completion receipt
- documentation boundary: `docs/` must describe current behavior separately from moonshot intent

### Public Interface Boundaries

Allowed later without migration notes:

- additive schemas
- semantic report compatibility tests
- doctor diagnostics
- release receipt docs

Forbidden later without migration notes:

- renamed report fields
- moved canonical score artifacts
- new external tools in `just fast`
- default target stack changes

## Workstream Table

| Workstream | Objective | Owned paths | Forbidden paths | Input contracts | Output contracts | Implementation tasks | Acceptance evidence | Validation commands | Handoff notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Artifact Contract Audit | Make every machine-readable artifact point to a schema or documented exemption. | `schemas/`, `docs/release-plan.md`, `docs/agent-native-standard.md`, `crates/jankurai/tests/` | `paper/`, `reference/`, hand-edited generated outputs, `agent/owner-map.json`, `agent/test-map.json` for this plan-only edit | schema list, agent artifact inputs, report outputs | schema index or matrix, parsed fixtures, compatibility notes | Verify all artifacts; add missing schemas; add tests for representative artifacts; document exemptions | Each artifact has a schema or documented reason; `agent/repo-score.json` remains parseable; no report field is removed without a migration note | `just fast`; `cargo test -p jankurai` | List new schemas and any intentional exemptions. |
| Report Compatibility Guard | Keep report JSON, Markdown, SARIF, JUnit, and repair queue shapes stable. | `crates/jankurai/src/report/`, `crates/jankurai/tests/`, `schemas/finding.schema.json`, `schemas/repair-queue.schema.json`, `schemas/repo-score.schema.json` | `paper/`, `reference/`, generated outputs by hand | version fields, report shape, finding shape, repair queue shape | semantic assertions on top-level and finding fields | Add or expand report compatibility tests; verify SARIF structure; verify repair queue JSONL; verify Markdown score and findings sections | Tests fail if required version fields or finding keys disappear | `just fast`; `cargo test -p jankurai` | Include exact test names and any compatibility carve-outs. |
| Doctor And Security Lane Reality Check | Distinguish missing tools from broken repo state and make security lane failures actionable. | `crates/jankurai/src/commands/doctor.rs`, `Justfile`, `docs/testing.md`, `docs/release-plan.md`, `.github/workflows/jankurai.yml` | `paper/`, report schema changes, generated artifacts by hand | `agent/audit-policy.toml`, `agent/boundaries.toml`, `agent/proof-lanes.toml`, `just security` command shape | actionable missing-tool diagnostics, security prerequisites, lane-specific failure text | Inspect `just security` and workflow; distinguish missing tools from repo defects; document prerequisites; flag placeholder proof | `doctor --fail-on high` reports missing control files and lane issues; security failures are actionable | `just fast`; `cargo run -p jankurai -- doctor --fail-on high`; `just security` | Note any tools that remain optional and any new prerequisite text. |
| Documentation Boundary | Make current behavior and roadmap-only behavior easy to tell apart. | `docs/release-plan.md`, `docs/moonshot.md`, `docs/agent-native-standard.md`, `README.md` if needed | `paper/`, `reference/`, generated outputs, runtime code | current baseline, release lines, phase sequence | explicit current-vs-future wording and links between release plan and phase plans | Separate implemented behavior from moonshot claims; keep root guidance short; avoid contradiction among docs | A new reader can tell what is implemented today and what is future-only | `just fast` | Cite any doc section that now points to the phase plan router. |
| Release Evidence Convention | Define where receipts live and what evidence a release candidate must show. | `docs/release-plan.md`, `docs/testing.md`, `agent/standard-version.toml`, `agent/repo-score.*`, ignored local receipt paths if introduced | `paper/`, generated output hand edits, moving canonical score artifacts | score report, doctor output, security output, UX proof paths, compatibility tests | receipt convention, evidence path list, source-vs-generated guidance | Define receipt location; document evidence bundle contents; describe which artifacts are source and which are generated | A release candidate can point to audit, security, UX, contract, and paper evidence paths without guesswork | `just fast`; `just score` | Include receipt path convention and any ignored-output assumptions. |
| Routing Map And Generated Zone Hygiene | Keep routing metadata and generated-zone declarations current for any new durable path. | `agent/generated-zones.toml`, `agent/proof-lanes.toml`, `agent/boundaries.toml`, `agent/audit-policy.toml`, `agent/standard-version.toml` | `paper/`, `reference/`, hand-edited generated files, report shape changes without compatibility notes | generated-zone manifest, proof lanes, boundary policy, version manifest | updated routing metadata and generated-zone declarations for any new durable path | Confirm every new durable path has owner/test coverage; keep generated zones stamped; update routing metadata only when new paths appear | Generated outputs are declared, stamped, and reproducible; new durable paths are routed before use | `just fast`; `cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md` | Mention any new path that forced a map update. |
| Final Receipt And Phase Handoff | Collect the closeout evidence and make the next phase unambiguous. | this phase plan file and any receipt or handoff doc for Phase 01 | implementation files outside the Phase 01 scope, paper edits, generated-output hand edits | results from all previous workstreams | completion receipt, unblocked Phase 02 and 03 checklist, unresolved exceptions | Gather evidence; record validation; note skipped commands; summarize remaining risks | The phase completion receipt can be filled without inventing facts | `just fast` | Note the Phase 02 and Phase 03 readiness gate status and any follow-up tickets. |

## Parallel MCP Work Plan

Use parallel MCP workers only after the shared contracts above are locked and only when write scopes are disjoint.

Safe split:

- Worker A owns `schemas/` and report compatibility tests.
- Worker B owns doctor and security lane reality checks.
- Worker C owns documentation boundary and release evidence wording.
- Worker D owns routing metadata and generated-zone hygiene.

Merge order:

1. lock schema and report compatibility
2. land doctor and security diagnostics
3. update docs boundary and release evidence language
4. update routing metadata only for any new durable path
5. finalize the receipt and handoff notes

Stop conditions:

- no worker edits `paper/` or `reference/`
- no worker hand-edits generated outputs
- no two workers edit `agent/test-map.json` at the same time
- no worker broadens `just fast` with new mandatory external tools

This phase is docs-only right now, so no parallel worker is needed to apply the plan itself. The parallel model exists for the Phase 01 implementation that this plan routes.

## Validation Matrix

| Change surface | Minimum validation | Add when relevant |
| --- | --- | --- |
| Phase 01 plan-only edit | `just fast` | `git status --short tips`, `git diff -- tips/phases/01-standard-stabilization.md`, `git diff --cached --name-only -- tips` |
| Schemas or report contracts | `just fast` | `cargo test -p jankurai`, semantic compatibility checks |
| Doctor or security lane changes | `just fast` | `cargo run -p jankurai -- doctor --fail-on high`, `just security` |
| Routing metadata or generated zones | `just fast` | `cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md` |
| Release evidence docs | `just fast` | `just score` |
| Paper | `just paper` | only if paper files change, which Phase 01 should avoid |

## Cleanup And Deprecation Policy

- No cleanup or deprecation is required for this docs-only plan edit beyond replacing the thinner draft.
- Do not remove or rename public report fields, canonical score artifacts, or command lanes without migration notes.
- Generated artifacts are never cleaned up by hand.
- If a later phase deprecates a schema, route, or command, it must add a migration note and a compatibility window.
- Existing dirty worktree changes are project state, not cleanup targets.
- `reference/` remains read-only.

## Phase 01 Completion Receipt

```md
## Phase 01 Completion Receipt

- Phase completed: 01 standard stabilization
- Files changed: `Justfile`, `.github/workflows/jankurai.yml`, `crates/jankurai/src/audit/mod.rs`, `crates/jankurai/src/commands/ci.rs`, `crates/jankurai/src/commands/doctor.rs`, `docs/release-plan.md`, `docs/testing.md`, `db/README.md`, `tools/security-lane.sh`, and the phase logs under `target/jankurai/phase-logs/`
- Schemas added or changed: report and repair/schema surfaces under `schemas/`
- Public report fields changed: compatibility preserved; additive receipt/policy/decision bindings added
- Generated artifacts: `agent/repo-score.json`, `agent/repo-score.md`
- Routing maps changed: `agent/owner-map.json`, `agent/test-map.json`, `agent/generated-zones.toml`
- Validation commands: `cargo test -p jankurai`, `just fast`, `just security`, `npm --workspace @jankurai/ux-qa run build`, `npm --workspace @jankurai/ux-qa run test`, `just score`
- Results: passed
- Skipped validation: none
- Exceptions created: wrapper-based security lane is advisory for missing optional tools outside strict mode
- Phase 02 unblocked: yes
- Phase 03 unblocked: yes
```

## Ready For Phase 02 And 03 Checklist

- Phase 01 receipt is complete and stored with the implementation evidence.
- Report compatibility fields are stable and covered by semantic tests.
- The schema surface either covers each artifact or names the exemption.
- `doctor` and `security` diagnostics distinguish missing tools from repo defects.
- The current-vs-future docs boundary is clear and does not contradict release docs.
- Release evidence paths are documented and reusable.
- Any new durable path has owner and test routing before it is used.
- `just fast` passes, and any additional validation required by touched paths also passes.
- No paper edits were needed for Phase 01.
- Phase 02 can start once report compatibility and artifact contracts are stable.
- Phase 03 can start once proof-routing and receipt conventions are stable.
