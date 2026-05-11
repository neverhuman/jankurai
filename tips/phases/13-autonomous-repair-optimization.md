# Phase 13: Autonomous Repair And Optimization

Status: hardened
Owner: agent
Last reviewed: 2026-05-04
Parallel MCP candidate: yes

## Objective

Add constrained autonomous repair and optimization after the proof, context, registry, migration, and certification systems are mature. The current slice hardens risk-gated dry-run repair planning, gated real-apply mutation, and draft PR receipts on top of fixture-only bounded patch execution.

This is intentionally the last phase. Autonomous repair without strong proof and permission boundaries would recreate vibe coding under a new name.

## Current State

Existing and planned prerequisites:

- Audit findings and repair queues exist.
- Phase 03 adds proof receipts.
- Phase 08 adds context packs, permission profiles, and repair packets.
- Phase 10 adds certified cells.
- Phase 11 adds migration slices.
- Phase 12 adds benchmark and certification evidence.

The implemented repair surface is dry-run by default with explicit fixture-only and gated real-apply modes. Repair packets and repair plans carry explicit eligibility, risk, planned edits, planned proof commands, rollback guidance, human approval requirements, and structured patch fields. Repair runs evaluate whether an auto-PR request would be blocked or eligible, emit a draft-only PR evidence package, and can execute bounded `append-text`, `replace-exact`, and `create-file` edits. Real repository patch execution requires `--apply` plus `JANKURAI_ALLOW_REPAIR_APPLY=1`, and clean git worktree plus proof-after-mutation plus automatic rollback on failure. Git commit requires `--git-commit` plus `JANKURAI_ALLOW_GIT_MUTATION=1`. GitHub draft PR creation requires `--github-pr` plus `JANKURAI_ALLOW_GITHUB_PR=1`. The optimizer reports token, performance, dependency, and dead-code candidates without mutating the tree. Exception expiry scans dated numbered docs under `docs/exceptions/`, emits a machine-valid report (`complete` versus `blocked` for expired or invalid entries), and supports `--strict` so CI can exit non-zero when expired or invalid exceptions persist. Expiring-soon entries keep status `complete` (warning counts only). `just phase13` and the shipped GitHub audit workflow run `optimize` plus `exceptions expire --strict`. Auto-merge is intentionally never introduced.

## Dependencies

Hard dependencies:

- Phase 03 proof router
- Phase 08 repair packets and permission profiles
- Phase 10 cell registry
- Phase 11 migration engine
- Phase 12 certification and benchmark governance

## Public Interface Changes

Implemented command surface:

```bash
jankurai repair-plan . --from target/jankurai/repo-score.json --out target/jankurai/repair-plan.json --md target/jankurai/repair-plan.md
jankurai repair . --plan target/jankurai/repair-plan.json --dry-run --out target/jankurai/repair-run.json --md target/jankurai/repair-run.md
jankurai repair . --plan target/jankurai/repair-plan.json --dry-run --auto-pr --max-risk low
jankurai repair . --plan target/jankurai/repair-plan.json --dry-run --auto-pr --max-risk medium --pr-draft-out target/jankurai/repair-pr-draft.json --pr-draft-md target/jankurai/repair-pr-draft.md
jankurai repair target/jankurai/p13-fixture-repo --plan target/jankurai/p13-fixture-repo/target/jankurai/repair-plan.json --fixture-apply --max-risk medium --out target/jankurai/p13-fixture-repair-run.json --md target/jankurai/p13-fixture-repair-run.md
jankurai repair . --plan target/jankurai/repair-plan.json --apply --max-risk medium --out target/jankurai/repair-run.json --md target/jankurai/repair-run.md
jankurai repair . --plan target/jankurai/repair-plan.json --apply --git-commit --auto-pr --github-pr --max-risk medium --out target/jankurai/repair-run.json --md target/jankurai/repair-run.md
jankurai optimize . --mode all --out target/jankurai/optimization-report.json --md target/jankurai/optimization-report.md
jankurai exceptions expire . --warning-days 7 --out target/jankurai/exception-expiry-report.json --md target/jankurai/exception-expiry-report.md
jankurai exceptions expire . --warning-days 7 --strict --out target/jankurai/exception-expiry-report.json --md target/jankurai/exception-expiry-report.md
```

Gated real-apply flow:

- Real repository patch execution requires `--apply` plus `JANKURAI_ALLOW_REPAIR_APPLY=1`.
- Git commit requires `--git-commit` plus `JANKURAI_ALLOW_GIT_MUTATION=1`.
- GitHub draft PR creation requires `--github-pr` plus `JANKURAI_ALLOW_GITHUB_PR=1`.
- Auto-PR remains draft-only; auto-merge is intentionally never introduced.

Exception expiry:

- Advisory by default (`status` reflects `blocked` when any exception is expired or invalid, but exit code stays zero unless `--strict`).
- `--strict` exits non-zero when `status` is `blocked` (does not affect `complete` repos with only current or expiring-soon dated exceptions).

## Contract Slice

`repair-plan.schema.json` stays non-destructive. Minimum fields:

- `schema_version`, `source_report`, `generated_at`, `target_stack_id`
- `plan_mode` fixed to `dry-run`
- `planned_edits[]` with `path`, `operation`, `reason`, `finding_fingerprint`, `rule_id`, `apply_strategy`, and optional patch text fields
- packet `repair_eligibility`, `risk_level`, and `eligibility_reason`
- `planned_commands`
- `proof_lanes`
- `rollback_guidance`
- `human_approval_requirements`
- `packets[]`

`repair-run.schema.json` records execution mode, repair execution status, auto-PR dry-run eligibility, optional auto-PR draft summary, max risk, blocked packets, risk summary, proof lanes, applied edits, skipped edits, files written, optional proof evidence index, and notes.

`repair-pr-draft.schema.json` records the draft-only PR evidence package with branch name, titles, planned paths, eligible and blocked packets, proof lanes, artifact links, residual risk, and mutation flags.

The current implementation supports dry-run planning, dry-run auto-PR eligibility reporting, fixture-only patch execution, gated real repository apply, gated git commit, gated GitHub draft PR creation, and proof-backed rollback on failure. It does not support auto-merge.

## Safety Principles

Autonomous repair may only act when:

- finding has stable fingerprint
- rule has repair eligibility
- allowed paths are explicit
- forbidden paths are explicit
- generated zones are protected
- permission profile allows edits
- proof lanes are executable
- rollback or revert plan exists
- human review requirement is clear
- max risk threshold is not exceeded

Autonomous repair must not:

- perform destructive migrations
- rotate secrets
- change production infrastructure credentials
- edit generated files by hand
- broaden agent permissions
- rewrite architecture broadly
- merge without proof
- hide failed tests
- create undocumented exceptions

## Workstreams

### 1. Repair Eligibility

Implementation tasks:

- Add eligibility metadata to rules:
  - auto-safe
  - agent-assisted
  - human-required
  - never-auto
- Define risk levels.
- Add tests that high-risk rules cannot auto-run.

Acceptance:

- Every auto-repairable rule declares why it is safe.
- Destructive or ambiguous work is human-required.
- Secret sprawl is never-auto and critical.

### 2. Dry-Run Repair Planner

Implementation tasks:

- Convert repair packets into patch plans.
- Include expected file edits, commands, proof, and rollback.
- Do not write files in first iteration.
- Emit Markdown and JSON plans.

Acceptance:

- A human can approve or reject plan before edits.
- Plans are deterministic for the same input report.
- Plans validate against `schemas/repair-plan.schema.json`.

### 3. Bounded Patch Execution

Implementation tasks:

- Apply edits only within allowed paths.
- Refuse generated zones unless running declared generator.
- Refuse changes outside task scope.
- Run proof lanes after patch.
- Emit repair receipt.

Acceptance:

- Patch cannot escape allowed paths.
- Failed proof stops repair and records evidence.

Status: complete. Fixture-only execution is supported for sandboxed testing. Real repository patch execution is supported behind explicit CLI, environment, and proof gates.

### 4. Auto-PR Workflow

Implementation tasks:

- Generate branch, commit, PR body, and artifact links.
- Include report fingerprint, proof receipts, risk level, and residual risk.
- Keep PR draft by default.
- Require human review for medium/high risk.

Acceptance:

- Auto-PR draft packages are transparent and auditable.
- Draft body includes exact proof lanes, artifact links, and residual risk.

Status: complete. Draft-only evidence packages are emitted for dry-run mode. Real branch, commit, push, and GitHub draft PR creation are supported behind `--apply --git-commit --github-pr` and corresponding environment gates.

### 5. Optimization Commands

Implementation tasks:

- Token reduction:
  - shorten root docs
  - move durable detail into routed docs
  - update context maps
  - remove duplicate agent instructions
- Performance:
  - detect budget regressions
  - suggest targeted fixes
  - require benchmark proof
- Dependency cleanup:
  - identify unused dependencies
  - require build/test proof
- Dead code:
  - identify orphan code
  - require reachability and tests

Acceptance:

- Optimization never removes behavior without proof.
- Token reduction reports before/after context size.

Status: complete. The `optimize` command reports token reduction, benchmark, dependency, and dead-code candidates without mutating the tree.

### 6. Exception Expiry Loop

Implementation tasks:

- Detect expired exceptions.
- Generate repair plans:
  - remove exception by fixing violation
  - renew with owner and justification
  - escalate to human
- Add dashboard/report summary.

Acceptance:

- Expired or invalid exceptions cannot silently persist where CI enables `--strict`.
- Repair options are explicit.

Status: complete. The `exceptions expire` command scans numbered docs under `docs/exceptions/` and reports expired, expiring-soon, current, and invalid records. Passing `--strict` fails the process when the report would be `blocked`.

## Parallel MCP Breakdown

Partial parallel candidate:

- Agent A: eligibility and risk model.
- Agent B: dry-run planner.
- Agent C: patch execution sandbox.
- Agent D: auto-PR integration.
- Agent E: optimization subcommands.
- Agent F: exception expiry loop.

Do not parallelize patch execution and permission model changes until the permission profile schema is locked.

Merge order:

1. Eligibility/risk model.
2. Dry-run planner.
3. Patch execution.
4. Auto-PR.
5. Optimization and exception loops.

## Validation

Minimum:

```bash
cargo test -p jankurai
just fast
```

Repair dry-run smoke:

```bash
jankurai repair . --plan target/jankurai/repair-plan.json --dry-run
```

Patch execution must use fixture repos before touching real projects.

Phase 13 public lane (optimization + strict exception expiry, matches CI):

```bash
just phase13
```

Optimization, real-mutation, and exception-expiry smoke:

```bash
cargo test -p jankurai --test phase_13_optimization_and_exceptions
cargo test -p jankurai --test phase_13_real_mutation
cargo test -p jankurai --test schema_contracts
```

## Risks

- Autonomous repair can become vibe coding if proof is weak.
- Agents can overfit to tests and miss product intent.
- Auto-PRs can spam maintainers if repair queue prioritization is poor.
- Optimization can remove useful context if token budget is valued over clarity.

## Handoff Notes

Leave:

- eligibility matrix
- risk policy
- dry-run examples
- fixture repair results
- proof receipts
- auto-PR template
- known never-auto rules
- residual risk that live GitHub draft PR creation still depends on network access and `gh` authentication

## Phase Status Receipt

- Phase status: hardened autonomous repair and optimization.
- Files changed in this slice: `crates/jankurai/src/commands/repair_apply.rs`, `crates/jankurai/tests/phase_13_real_mutation.rs`, `crates/jankurai/tests/schema_contracts.rs`, `docs/phases-feedback-status.md`, `tips/phases/00-phase-index.md`, `tips/phases/13-autonomous-repair-optimization.md`, `tips/phases/logs/13-autonomous-repair-optimization.log`.
- Generated artifacts: `target/jankurai/p13-final-hardening-lane.json`, `target/jankurai/p13-final-hardening-lane.md`, `target/jankurai/p13-final-source-score.json`, `target/jankurai/p13-final-source-score.md`, `target/jankurai/p13-final-repair-plan.json`, `target/jankurai/p13-final-repair-plan.md`, `target/jankurai/p13-final-repair-run.json`, `target/jankurai/p13-final-repair-run.md`, `target/jankurai/p13-final-repair-pr-draft.json`, `target/jankurai/p13-final-repair-pr-draft.md`, `target/jankurai/fast-score.json`, `target/jankurai/fast-score.md`, `agent/repo-score.json`, `agent/repo-score.md`.
- Validation: `cargo test -p jankurai`; `cargo test -p jankurai --test phase_13_real_mutation`; `cargo test -p jankurai --test schema_contracts`; `cargo test -p jankurai --test phase_13_patch_execution --test phase_13_auto_pr_draft --test phase_13_optimization_and_exceptions --test command_surface_smoke`; `just fast`; `just score`; `just security`; `cargo run -p jankurai -- versions`; `npm --workspace @jankurai/ux-qa run build`; `git diff --check`; `cargo run -p jankurai -- lane . --changed crates/jankurai/src/commands/repair_apply.rs --changed crates/jankurai/tests/phase_13_real_mutation.rs --changed crates/jankurai/tests/schema_contracts.rs --changed docs/phases-feedback-status.md --changed tips/phases/00-phase-index.md --changed tips/phases/13-autonomous-repair-optimization.md --changed tips/phases/logs/13-autonomous-repair-optimization.log --out target/jankurai/p13-final-hardening-lane.json --md target/jankurai/p13-final-hardening-lane.md`; `cargo run -p jankurai -- . --json target/jankurai/p13-final-source-score.json --md target/jankurai/p13-final-source-score.md`; `cargo run -p jankurai -- repair-plan . --from target/jankurai/p13-final-source-score.json --out target/jankurai/p13-final-repair-plan.json --md target/jankurai/p13-final-repair-plan.md`; `cargo run -p jankurai -- repair . --plan target/jankurai/p13-final-repair-plan.json --dry-run --auto-pr --max-risk medium --out target/jankurai/p13-final-repair-run.json --md target/jankurai/p13-final-repair-run.md --pr-draft-out target/jankurai/p13-final-repair-pr-draft.json --pr-draft-md target/jankurai/p13-final-repair-pr-draft.md`.
- Residual risk: live GitHub draft PR creation depends on network access and `gh` authentication; auto-merge is intentionally never introduced.
- Results: real-apply rollback, branch cleanup, GitHub PR failure receipts, and schema example coverage are now exercised by tests; the live gate surface remains explicit.
- Follow-up phases: none; Phase 13 is hardened.
