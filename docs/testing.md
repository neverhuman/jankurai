# jankurai Testing

Testing is routed proof. Agents should not guess which tests matter.

| Lane | Purpose |
| --- | --- |
| `fast` | deterministic local proof for most edits |
| `contract` | API/schema generation and drift checks |
| `db` | migrations, constraints, schema drift |
| `db-migration-analyze` | migration liability report from `jankurai migrate --analyze` (routed for `db/migrations/` in `agent/test-map.json`) |
| `web` | TypeScript typecheck, component tests, rendered UX QA |
| `e2e` | Playwright critical product flows |
| `security` | secrets, dependencies, SBOM/SCA, workflow lint |
| `observability` | traces, request IDs, structured error payloads |
| `audit` | jankurai repo score and hard-rule findings |
| `copy-code` | exact and high-confidence duplicate source scan |
| `proofbind` | changed-path to semantic-surface obligation routing |
| `proofmark-rust` | changed Rust line coverage, focused mutation, and negative proof receipt evidence |
| `full` | release/merge gate |

## SQL migration safety (audit)

Jankurai flags destructive statements in SQL files under migration roots (for example `db/`, `**/db/migrations/`, paths from `agent/boundaries.toml` `[db]`, and common `migrations/` layouts). Findings use stable rule ID **`HLT-021-DESTRUCTIVE-MIGRATION`** and cap bucket **`destructive-migration-risk`**.

**Documented safety (file-level):** the audit skips the finding when the migration file mentions rollback, down migration, backfill, lock timeout or advisory lock, staged deploy, expand/contract, or contains the marker **`jankurai:migration-safe`**. That marker is a **policy escape hatch**: it suppresses the finding only when a human has explicitly approved the exception; it does not prove the SQL is safe.

**Proof lane:** changes under `db/migrations/` route in `agent/test-map.json` to `cargo run -p jankurai -- migrate . --analyze --json target/jankurai/migration-report.json`, named lane **`db-migration-analyze`** in `agent/proof-lanes.toml`.

**Limitations:** most destructive checks are line-oriented; unbounded `DELETE FROM` is refined with a short lookahead (following lines) for a leading `WHERE`, but pathological SQL (strings, procedural bodies) can still confuse the scanner. Prefer keeping destructive statements and safety notes clearly commented in the same file.

**SARIF:** audit exports map repo-relative rule `docs_url` paths (for example `docs/testing.md`) to an absolute GitHub `blob/main/...` URL in each rule's `helpUri`. Finding regions include matching `startLine`/`endLine` and a `snippet` from evidence or a short `problem` excerpt.

For this workspace:

- `jankurai kickoff` is the no-write first-hour intake command. It writes `target/jankurai/kickoff.json` and `target/jankurai/kickoff.md`, surfaces read-first files, ownership boundaries, proof lanes, generated-zone and forbidden-path constraints, clarifying questions, expected receipts, and next commands, and keeps route decisions conservative until the repo facts are visible.
- `just versions` checks version and artifact bindings through the Rust auditor.
- `just ux-qa` builds and tests the optional Playwright geometry runtime.
- `just fast` writes a deterministic audit snapshot under `target/jankurai/`.
- `just score` writes local generated audit outputs at `.jankurai/repo-score.json` and `.jankurai/repo-score.md`; these files are ignored and are not accepted ratchet baselines.
- `jankurai copy-code . --json target/jankurai/copy-code.json --md target/jankurai/copy-code.md` writes the copy-code redundancy report used for HLT-043 routing.
- Accepted ratchet and public badge baselines live under `agent/baselines/`. CI copies the reviewed baseline to `target/jankurai/accepted-baseline.json` before the final audit.
- `just conformance` runs the observed seed fixture suite, validates the
  conformance report schema through Rust tests, and regenerates
  `paper/tex/generated/conformance_results_table.tex`.
- `just paper` builds `paper/jankurai.pdf`.
- `just check` runs quality, strict CI-profile security evidence, conformance, final score, and paper build.
- `jankurai doctor` and `jankurai init` write receipts under `target/jankurai/receipts/` for handoff evidence.
- `jankurai kickoff` is the no-write intake step that precedes `context-pack`, `prove`, and `witness`; it does not replace proof lanes and should not claim merge readiness.
- `jankurai migrate verify-prompt` and `jankurai migrate slice-risk` are no-write migration evidence commands. The prompt verifier must classify ambiguous module or call-site claims as `review` instead of inventing certainty; the slice-risk scanner should surface blockers and recommendations without executing cutover behavior.
- `jankurai postmortem record` writes durable TOML records under `.jankurai/postmortems/` only when explicitly asked; `jankurai postmortem list`, `show`, and `read` are no-write views over those records.
- `jankurai doctor` treats severity claims in prose as advisory unless a nearby `Severity-Justified:` or `Blocker-Type:` trailer explains them. It skips TOML postmortem records and only scans explicit prose roots such as `README.md` and `docs/`.
- `jankurai prove` executes a proof-plan JSON. Commands must match `agent/proof-lanes.toml` and `agent/test-map.json` after whitespace normalization, unless `--allow-unsigned-commands` is passed together with `JANKURAI_ALLOW_UNSIGNED_PROOF_COMMANDS=1` (emergency only; keep CI on the default allowlist).
- `jankurai proof-verify` compares a proof plan and evidence index against the current repo state and writes a tamper-evident verification envelope.
- `jankurai proofbind verify` writes `target/jankurai/proofbind/surface-witness.json`, `target/jankurai/proofbind/obligations.json`, and `target/jankurai/proofbind/proofbind.md`. First rollout is advisory: missing semantic proof is reported as repair work unless `--mode required` is used.
- `jankurai proofmark rust` writes `target/jankurai/proofmark/proofmark-receipt.json`, a standard proof receipt at `target/jankurai/proofmark/proof-receipt.json`, and `target/jankurai/proofmark/proofmark.md`. Coverage gaps stay review/advisory; do not fake hard proof when coverage or mutation evidence is unavailable.
- For language bad-behavior audits, the same lane now anchors `HLT-029-RUST-BAD-BEHAVIOR` through `HLT-037-RELEASE-BAD-BEHAVIOR`. Use `cargo test -p jankurai --test language_bad_behavior` to exercise the focused detector pack before broader score reruns; the fixture corpus in `crates/jankurai/tests/fixtures/language_bad_behavior/` documents the `sql`, `typescript`, `docker`, `python`, `ci`, `git`, `gittools`, and `release` families alongside Rust.
- `jankurai witness` writes a merge witness that checks changed-path routing, generated-zone touches, baseline score delta, current audit status, and proof receipt coverage. It may report proof freshness as unknown unless receipts carry git/file digests; it must not claim freshness without evidence.
- `jankurai history latest`, `jankurai history export`, `jankurai history compact`, and `jankurai history restore` validate the bounded score ledger and the mirror recovery path.
- `jankurai score diff` and `jankurai score trend` validate rolling score artifacts so regressions, new findings, caps, and high/critical counts are visible before ratchet gates.
- `jankurai vibe validate --source agent/vibe-coverage.toml --tips tips/vibe_coding` proves every source row is mapped exactly once, matches the source title, is reviewed, references known rules/tools/lanes, has no unjustified `none`, and only claims `detector-backed` with deterministic audit evidence.
- `jankurai vibe coverage --source agent/vibe-coverage.toml --tips tips/vibe_coding --json target/jankurai/vibe-coverage.json --md target/jankurai/vibe-coverage.md --tex paper/tex/generated/vibe_coverage_table.tex` emits the JSON, Markdown, and paper table coverage artifacts.
- New proof and audit automation must be Rust-first. Do not add Python helpers for proof lanes, repo tools, product truth, product services, authorization, general backend glue, or PostgreSQL writes. Python belongs only to rare dated advanced-ML/data exceptions under `python/ai-service`.
- Proof run artifacts: `target/jankurai/proof-receipts/*.json`, `target/jankurai/logs/*.log`, and `target/jankurai/evidence-index.json`, each validated against the matching `schemas/*.schema.json` on write where applicable. Evidence carries plan, command, log, receipt, and artifact digests plus manifest fingerprints.
- `jankurai doctor` validates proof receipts, the evidence index, `target/jankurai/security/evidence.json`, and when present `target/jankurai/context-pack.json` / `target/jankurai/repair-plan.json`; it warns on stale proof `git_head`. Its receipt records typed diagnostics with kind, environment-sensitivity, blocking state, and common fixes. It validates **`agent/boundaries.toml`** and, when present, **`agent/ux-qa.toml`** (`schemas/boundaries.schema.json`, `schemas/ux-qa-policy.schema.json`). Parse/schema failures are **medium** (use `--fail-on medium` to treat as blocking).
- `jankurai context-pack` and `jankurai repair-plan` emit JSON validated against `schemas/context-pack.schema.json` and `schemas/repair-plan.schema.json` on every file write (and validate before printing to stdout when `--out` is omitted).
- `just security` invokes `jankurai security run` and writes `target/jankurai/security/evidence.json`. The envelope includes policy snapshots and per-step policy-blocking metadata. Use `just security-bash` only when debugging the shell script without the evidence envelope.
- Receipts should record the command, exit code, changed paths, artifacts, and the rerun command that the next agent should trust.
- Phase closeouts should cite the exact receipt path instead of relying on chat history.
- Prefer structured errors, telemetry, and repair receipts that tell the next agent where to rerun proof.

Rendered UX QA combines Storybook states, Playwright screenshots, ARIA snapshots, deterministic visual-baseline hashes and optional pixel-diff receipts, axe/WCAG checks, CLS checks, MSW/generated mocks, design tokens, and deterministic DOM geometry rules such as edge clearance, target size, overlap, clipping, wrapping, horizontal overflow, sticky obstruction, focus visibility, form labels, and nested scrollbars.

Critical UI proof must be artifact-backed. A useful receipt names the route or story, browser, viewport, action sequence, screenshot or crop path, ARIA snapshot path when available, sha256 artifact digests, rule IDs, selectors, owner, and merge decision. Deterministic rule violations block; visual baselines compare file bytes and hashes, not pixels or AI/VLM judgment; review only applies to owner-approved baseline changes or ambiguous product calls.

`@jankurai/ux-qa` now emits UX report schema `1.4.0`; validation still accepts existing `1.2.0` and `1.3.0` reports for compatibility. The `1.4.0` contract keeps `artifactCoverage` and `accessibility` summaries, adds artifact `sha256` digests, `state`, and `visualBaseline` summary fields, and keeps older envelopes valid. `agent/ux-qa.toml` policy fields for `readyState`, `timeoutMs`, `screenshotRequired`, `ariaSnapshotRequired`, `accessibilityScanRequired`, `visualBaselineRoot`, `visualDiffRoot`, and `stateQueryParam` drive CLI behavior unless an explicit CLI flag overrides them; route-level overrides can refine baseline paths, owners, and baseline mode.

Tool adoption is scored separately from the core proof lanes. The built-in catalog currently tracks `audit-ci`, `proof-routing`, `security`, `ux-qa`, `db-migration-analyze`, `contract-drift`, `rust-witness`, `vibe-coverage`, `authz-matrix`, `input-boundary`, `agent-tool-supply`, `release-readiness`, `release-bad-behavior`, and `cost-budget`. `agent/tool-adoption.toml` records per-tool mode as `auto`, `required`, `advisory`, or `disabled`. In the audit report, a tool only counts as replaced when CI runs the relevant Jankurai-backed lane and uploads the expected artifact evidence; local config is readiness only.

Tool adoption counters have distinct meanings:

- `configured_count` is the number of applicable catalog tools with an explicit `agent/tool-adoption.toml` entry, even when stronger CI evidence upgrades the item status to `ci_evidence` or `artifact_verified`.
- `ci_evidence_count` is the number of applicable tools whose catalog CI command is present in GitHub Actions.
- `artifact_verified_count` is the number of applicable tools whose CI command and expected uploaded artifact paths are both present.
- `replaced_count` follows CI-backed adoption and currently equals `ci_evidence_count`; local configuration alone does not count as replacement proof.

Item status remains a strongest-observed-evidence label: `configured` for local config only, `ci_evidence` when CI runs the lane, `artifact_verified` when CI also uploads the expected artifacts, `missing` when an applicable tool has no evidence, and `not_applicable` when the tool does not apply to the repository.

The adoption score is intentionally soft-capped. It rewards control-plane presence, configured applicable tools, CI evidence, and artifact verification, then applies a soft cap when a required applicable tool lacks CI-backed evidence. Non-web repos do not get UX QA pressure unless they actually have a web surface.

When required states or required screenshot/ARIA/accessibility artifacts are missing, the UX CLI marks the report `block`. State generation can be driven by `stateQueryParam` so each configured state becomes a concrete URL variant without changing the underlying route contract. Validated `target/jankurai/ux-qa.json` evidence is ingested into repo-score as `ux_qa.artifact`, including artifact counts by kind, missing state names, missing required artifact kinds, and accessibility violation/incomplete/pass totals. The audit adds `HLT-013-RENDERED-UX-GAP` for incomplete state or non-a11y artifact coverage and `HLT-014-A11Y-GAP` for axe violations or missing accessibility artifacts. This slice does not add numeric score caps.

Automated accessibility is evidence, not a complete inclusive testing replacement. Axe catches common machine-detectable WCAG issues; keyboard, screen-reader, cognitive load, localization, motion, and product-context review still need human or domain-specific proof. No AI/VLM or pixel-diff authority replaces deterministic baselines here.

Schema-first work should get a parse smoke test before command wiring lands. For new contract files under `schemas/`, add a Rust test that loads the JSON and checks the required fields or references the contract chain. Keep that proof under `cargo test -p jankurai` so the schema stays machine-readable while the CLI surface is still being planned.

The security lane is wrapper-aware: `tools/security-lane.sh` is the canonical shell entrypoint for secret scanning, dependency review, SBOM, and workflow lint checks. For **per-tool rows** in `target/jankurai/security/evidence.json` `commands[]`, the bundled script emits **`jankurai-security-step=`** JSON lines directly from shell; no Python runtime is required.

Observability repairs should stay typed. The auditor now carries repair-hint surfaces in `crates/jankurai/src/audit/mod.rs` with purpose, reason, common fixes, `docs_url`, and `repair_hint` fields so the next rerun stays local.

The CLI defaults to `domcontentloaded`. Override with `--wait-for` and `--timeout-ms` when the preview server or app shell needs a different readiness contract.

Route-matrix and Storybook commands:

```bash
jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json
jankurai ux storybook --url http://localhost:6006 --config agent/ux-qa.toml
```

Artifacts in reports must be relative to the repo or configured output root.

The full target-stack test doctrine lives in `docs/agent-native-standard.md`
and the canonical TeX paper under `paper/tex/`.
