# Phase 00: Moonshot Phase Index

Status: complete
Owner: standard
Last reviewed: 2026-05-03
Operational role: active moonshot router
Applies to: all files under `tips/phases/` and `tips/phases/logs/`

## Purpose

This file is the canonical router for the moonshot phase sequence. A fresh agent uses it with `agent/JANKURAI_STANDARD.md`, `agent/MASTER_PLAN.md`, the active phase file, and the matching log to choose the next safe slice, the smallest credible proof lane, and the receipt that closes the work.

## Moonshot Contract

```text
intent -> bounded agents -> proof lanes -> evidence -> expired exceptions -> reusable primitives
```

The router makes that loop executable:

- intent before edits
- bounded write authority
- ownership and proof for every changed path
- generated truth declared by source and command
- durable evidence under `target/jankurai/`
- exceptions with owner and expiry
- repeated fixes turned into reusable primitives

## Source Of Truth

Use this precedence when documents disagree:

1. current session instructions
2. `agent/JANKURAI_STANDARD.md`
3. `agent/MASTER_PLAN.md`
4. `docs/agent-native-standard.md` and `docs/moonshot.md`
5. this index, the active phase file, and the matching log
6. owner/test/proof/generated/version maps
7. older roadmap prose only when not contradicted

If a phase header and receipt disagree, the receipt controls execution state and this index must name the residual risk instead of erasing it.

## Status Taxonomy

| Router state | Meaning | Default selection |
| --- | --- | --- |
| `planned` | No real implementation surface exists yet. | Select only when no safer active or deferred slice exists. |
| `partial` | Material acceptance criteria remain open. | Select when dependencies are satisfied. |
| `complete` | Required v0 surface and receipt exist. | Do not reopen unless named or blocked by downstream evidence. |
| `complete-with-residual` | Complete enough for downstream work, but the docs name hardening worth keeping visible. | Skip unless the residual blocks the next slice. |
| `complete-with-deferred-work` | Required slice is done, and future work is explicitly deferred. | Treat as stable; do not reopen by default. |
| `hardened` | Complete plus an additional validated production-quality slice. | Stable proving ground. |

## Read First

1. `AGENTS.md`
2. `agent/JANKURAI_STANDARD.md`
3. `agent/MASTER_PLAN.md`
4. `docs/moonshot.md`
5. this file
6. the target phase file under `tips/phases/`
7. the matching log under `tips/phases/logs/`
8. `agent/owner-map.json`
9. `agent/test-map.json`
10. `agent/generated-zones.toml`
11. `agent/proof-lanes.toml`
12. `agent/standard-version.toml`

## Ownership And Conflict Risk

| Scope | Rule |
| --- | --- |
| Owned | this file and the append-only Phase 00 log |
| Forbidden | `reference/`, `paper/`, runtime code, schemas, and generated artifacts by hand |
| Conflict risks | two agents touching the same phase log; one agent revising a phase file while another revises its log |

This router change does not require owner-map or test-map edits because the touched `tips/` paths are already routed. `docs/moonshot.md`, receipts, logs, and other phase files stay read-only unless they are the selected phase.

## Phase Dependency Graph

```text
00 -> 01 -> 02 -> 03 -> 04 -> 05 -> 06 -> 07 -> 08
04 + 05 + 06 + 07 + 08 -> 09
04 + 07 + 09 -> 10
02 + 03 + 07 + 08 -> 11
01..11 -> 12
03 + 08 + 10 + 11 + 12 -> 13
```

## Current Ledger

| Phase | State | Router note |
| --- | --- | --- |
| 01 | hardened | standard stabilized and verification envelopes added |
| 02 | hardened | canonical policy contract and rule mapping added |
| 03 | hardened | proof router, evidence ledger, and verification envelope in place |
| 04 | hardened | init profiles, golden repos, external-repo-safe templates, no-write adopt planning, and observe-mode CI in place |
| 05 | complete | UX proof platform in place |
| 06 | complete | security supply chain and compliance evidence in place |
| 07 | hardened | contracts/DB/generated boundaries hardened; all residual gaps closed |
| 08 | complete | agent context and repair scaffolding in place |
| 09 | complete | reference product platform in place |
| 10 | hardened | six certified cells (audit-log, crud-resource, rbac, auth-session, organization-team, background-job); cells remain dry-run/prove evidence surfaces; mutating/provider-backed installs remain deferred; next cell is webhook receiver |
| 11 | hardened | structured inventory, 8-dimension liability scoring, fixture-backed detection tests, slice risk levels; adoption auto-routes far repos to migration-target |
| 12 | hardened | Phase 12 public bundle: `jankurai publish`, badge JSON/SVG, `public-evidence-bundle` schema; CI + `just phase12`; GitHub attest / Sigstore / dashboards remain optional |
| 13 | hardened | dry-run repair, fixture apply, gated real-apply, rollback, draft PR creation, optimize, and exception expiry are live; live GitHub draft PR creation still depends on network access and gh auth; auto-merge intentionally deferred |

## Selection Policy

1. If the user names a phase, use it unless a hard dependency makes progress unsafe.
2. Otherwise, choose the earliest `partial` phase; if none exists, choose the earliest `complete-with-residual` or `complete-with-deferred-work` slice that improves proof, boundaries, or mutation safety.
3. Prefer work in this order when nothing is named: Phase 13 only for regressions in repair gating, rollback, or external GitHub PR receipts; then Phase 10 next certified cell (webhook receiver / mutating installs / next reusable cell); Phase 12 is **hardened**—revisit only on public-evidence regressions or optional signing/dashboard follow-ons.
4. Before edits, append a `start` entry, run the smallest credible proof lane, then append a `finish` entry with changed paths, validation, artifacts, git SHA, and residual risk.
5. Never hand-edit generated artifacts. Fix the source and regenerate.

## Validation

For Phase 00 router changes:

- `rtk git diff --check -- tips/phases/00-phase-index.md tips/phases/logs/00-phase-index.log`
- `rtk cargo run -p jankurai -- lane . --changed tips/phases/00-phase-index.md --out target/jankurai/phase00-index-lane.json --md target/jankurai/phase00-index-lane.md`
- `rtk cargo run -p jankurai -- lane . --changed tips/phases/logs/00-phase-index.log --out target/jankurai/phase00-log-lane.json --md target/jankurai/phase00-log-lane.md`
- `rtk just fast`
- `rtk just score`
- `rtk just check`

Expected artifacts:

- `target/jankurai/phase00-index-lane.{json,md}`
- `target/jankurai/phase00-log-lane.{json,md}`
- `target/jankurai/fast-score.{json,md}`
- `agent/repo-score.{json,md}`

Failure interpretation:

- if the phase-index lane fails, fix the router text or selection logic
- if the log lane fails, fix the append-only log row format
- if `just score` or `just check` fails for unrelated repo state, do not mark Phase 00 complete and record the exact failing command

## Logging And Receipts

Phase logs are append-only and use:

```text
timestamp_utc | actor/tool | phase | action | changed_paths | validation | artifacts | git_sha | residual_risk
```

Write `start` before edits, `progress` only when scope or risk changes, and `finish` after validation. The phase receipt lives in this file; the log is the durable cross-agent history.

## Parallel Work Packets

Use parallel workers only when write scopes are disjoint. Template: `Agent`, `Phase`, `Scope`, `Owned paths`, `Forbidden paths`, `Validation`, `Artifacts`, `Stop`, `Merge order`. Do not let two workers edit the same phase log or the same phase file.

## Stop Conditions

- Stop if ownership/proof routing is missing, a generated or read-only file would be hand-edited, validation is skipped, a finish receipt is missing, or parallel work overlaps. Do not update the paper until all phase plans are complete; do not rewrite append-only logs; treat dirty worktree changes as project state.

## Phase Completion Receipt

- Phase completed: 00 phase index
- Files changed: `tips/phases/00-phase-index.md`, `tips/phases/logs/00-phase-index.log`
- Validation: `rtk git diff --check -- tips/phases/00-phase-index.md tips/phases/00-phase-index.log`; `rtk cargo run -p jankurai -- lane . --changed tips/phases/00-phase-index.md --out target/jankurai/phase00-index-lane.json --md target/jankurai/phase00-index-lane.md`; `rtk cargo run -p jankurai -- lane . --changed tips/phases/logs/00-phase-index.log --out target/jankurai/phase00-log-lane.json --md target/jankurai/phase00-log-lane.md`; `rtk just fast`; `rtk just score`; `rtk just check`
- Results: pending this turn's validation
- Follow-up phases unblocked: 01 standard stabilization, plus the documented downstream phase order
