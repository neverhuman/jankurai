# Jankurai Master Plan

Status: active
Owner: agent
Last reviewed: 2026-05-02

## Purpose

This is the compact router for all agent work that is described as "progress the MASTER_PLAN", "work the phase plan", or "audit and continue". It does not replace the phase roadmap. It tells every agent which files define the current plan, how to pick work, how to prove it, and where to leave phase history.

`MASTER_PLAN` means this file plus:

- `tips/phases/00-phase-index.md`
- the active phase file under `tips/phases/*.md`
- the matching append-only log under `tips/phases/logs/`

## Read Order

Before phase work, read:

1. `agent/JANKURAI_STANDARD.md`
2. `agent/MASTER_PLAN.md`
3. `tips/phases/00-phase-index.md`
4. the selected phase file under `tips/phases/`
5. `agent/owner-map.json`
6. `agent/test-map.json`
7. `agent/generated-zones.toml`
8. `agent/standard-version.toml`

Read `docs/agent-native-standard.md` when policy detail matters.

Reconciliation of `tips/phases_feedback/` notes with shipped behavior lives in `docs/phases-feedback-status.md`. Artifact/schema index: `docs/artifact-contracts.md`. After changing audit output or export flags, run `cargo test -p jankurai --test report_compatibility_guard` (or `just compat`) in addition to the usual crate tests. The security lane proof for this repo is `just security` (wraps `jankurai security run`).

Default to the earliest incomplete or blocked phase whose dependencies can be advanced. If the user names a phase, use that phase unless its dependencies make progress impossible.

Do not regress phase state. A later cleanup may clarify status, add evidence, or mark a smaller slice complete, but it must not erase prior phase receipts or shorten existing phase plans.

Before editing, append a start entry to the matching log in `tips/phases/logs/`.

During long or split work, append progress entries when the ownership scope, validation choice, or residual risk changes.

At handoff, append a finish entry with changed paths, validation, artifacts, git SHA, and residual risk.

## Detailed Planner Protocol

When asked for a planning phase, implementation plan, phase plan, or worker handoff, produce a worker-ready plan for a strong planner delegating execution to weaker agents. The plan must be detailed enough that a fresh agent with minimal context can finish the phase safely without rediscovering the whole repository.

Required plan sections:

- `Objective`: the exact phase outcome, non-goals, assumptions, dependency gates, and completion criteria.
- `Read First`: ordered files to read before implementation, including the active phase file, logs, owner/test maps, generated zones, and target source files.
- `Ownership`: owned paths, forbidden paths, generated/read-only paths, and known concurrent-agent conflict risks.
- `Current State`: what already exists, what is verified, and what should not be rebuilt.
- `Implementation Steps`: small ordered edits, with exact files, functions, structs, schemas, docs, and tests likely to change.
- `Hard Parts`: complex control flow, data-model compatibility, edge cases, and code snippets or pseudocode that steer implementation.
- `Validation`: smallest credible proof lane, focused tests, broad validation, expected artifacts, and how to interpret common failures.
- `Logging And Receipts`: phase log start/progress/finish entries, proof artifacts under `target/jankurai/`, and final handoff content.
- `Parallel Work Packets`: safe packets only for disjoint write scopes, each with owned paths, forbidden paths, expected output, validation, stop conditions, and merge order.

Plans must prefer exact commands and repo-relative paths over generic advice. Include code-shaped guidance for the most error-prone parts, but do not ask worker agents to hand-edit generated artifacts or broaden permissions. If implementation starts in the same session, append the phase start log before edits and update the plan as facts change.

## Proof Routing

Use the smallest credible proof lane for the changed paths.

Before broad validation, prefer:

```bash
cargo run -p jankurai -- lane . --changed <path> --out target/jankurai/<name>.json --md target/jankurai/<name>.md
```

or the equivalent `jankurai proof` command when the phase requires receipt writing.

For audit requests, run:

```bash
cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md
```

Keep proof receipts, command output, SARIF, screenshots, and volatile evidence under `target/jankurai/`. Keep canonical cross-agent phase history under `tips/phases/logs/`.

## Phase Logs

Every phase has one tracked log file:

```text
tips/phases/logs/<phase-slug>.log
```

Use this line format for new entries:

```text
timestamp_utc | actor/tool | phase | action | changed_paths | validation | artifacts | git_sha | residual_risk
```

Use `none`, `pending`, or `not-run` explicitly instead of leaving a field blank.

## Parallel MCP/Agent Work Packet

Use parallel agents only for disjoint write scopes. The parent agent owns consolidation and must append the final phase log entry.

Packet template:

```text
Agent name:
Phase:
Scope:
Owned paths:
Forbidden paths:
Input contracts:
Output contracts:
Log path:
Validation commands:
Expected artifacts:
Stop conditions:
Handoff expectations:
Residual risk:
```

## Handoff Checklist

- Changed paths are owned in `agent/owner-map.json`.
- Changed paths have proof routing in `agent/test-map.json`.
- Generated outputs were not hand-edited.
- The smallest proof lane was run or the skip is logged.
- Phase log has start and finish entries.
- Artifacts under `target/jankurai/` are cited when they matter.
