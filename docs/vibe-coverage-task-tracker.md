# v0.6.1 Vibe-Coding Coverage Tracker

Last updated: 2026-05-04
Owner: agent
Release: 0.6.1

This tracker governs the v0.6 vibe-coding coverage release. It does not
advance `agent/MASTER_PLAN.md`, does not select phase work, and does not append
phase log entries. `agent/MASTER_PLAN.md` and `tips/phases/*` apply only when a
user explicitly asks for MASTER_PLAN or phase work.

## Tasks

| ID | Area | Status | Changed paths | Validation | Artifacts | Residual risk |
| --- | --- | --- | --- | --- | --- | --- |
| VCOV-00 | Release tracker | done | `docs/vibe-coverage-task-tracker.md` | `git diff --check` | tracker | none |
| VCOV-01 | Conditional phase routing | done | `AGENTS.md`, adapters, init templates/tests | `cargo run -p jankurai -- adapters verify` | synced adapter files | adapter wording drift |
| VCOV-02 | Canonical coverage source | done | `agent/vibe-coverage.toml` | `jankurai vibe validate` | source registry | issue-specific evidence remains partial for 243 rows |
| VCOV-03 | Source/report schemas | done | `schemas/vibe-coverage-*.schema.json`, validation code/tests | `cargo test -p jankurai --test schema_contracts` | schemas | schema evolution must remain backward compatible |
| VCOV-04 | Vibe CLI parser/reporter | done | `crates/jankurai/src/commands/vibe.rs`, CLI wiring | `cargo test -p jankurai --test vibe_coverage_smoke` | JSON/Markdown/TeX reports | table length still depends on TeX page geometry |
| VCOV-05 | Audit integration | done | `model.rs`, `audit/mod.rs`, `render.rs`, repo-score schema | `cargo test -p jankurai --test report_compatibility_guard` | `.jankurai/repo-score.*` | optional summary must not invalidate old reports |
| VCOV-06 | Rule registry extension | done | `audit/rules.rs`, scan/findings, tests, docs | `cargo test -p jankurai --test rule_registry_smoke --test vibe_detector_fixtures` | rule registry export | detectors remain heuristic outside fixture families |
| VCOV-07 | Tool adoption and CI | done | `agent/tool-adoption.toml`, workflow, docs | `just score` | uploaded vibe coverage artifacts | CI artifact proof requires hosted run |
| VCOV-08 | Paper table integration | done | `paper/tex/*`, generated table | `just paper` | `paper/tex/generated/vibe_coverage_table.tex` | TeX table layout |
| VCOV-09 | Version sweep | done | version manifests, changelog, docs | `just versions` | 0.6.1 / 1.4.2 bindings | historical v0.6.0 release notes remain intentionally unchanged |
| VCOV-10 | Generated audit artifacts | open | `.jankurai/repo-score.json`, `.jankurai/repo-score.md`, score history | `just score` | audit JSON/MD/history | dirty worktree affects report fingerprint |
| VCOV-11 | Full validation | open | all owned paths | `just check` | target proof artifacts | environment-specific failures |
| VCOV-12 | Final handoff | open | final response | n/a | changed path list and counts | none |

## Stop Conditions

Stop if any `tips/vibe_coding/tipN.txt` source row is unmapped, if the coverage
source or emitted report cannot be schema-validated, if generated TeX drifts
from `agent/vibe-coverage.toml`, if existing dirty work would be overwritten, or
if release work starts executing MASTER_PLAN/phase tasks.
