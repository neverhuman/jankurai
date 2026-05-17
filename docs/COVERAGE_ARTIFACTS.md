# Coverage Evidence Artifacts

The default config is `agent/coverage-sources.toml`. The audit outputs are:

- `target/jankurai/coverage/coverage-audit.json`
- `target/jankurai/coverage/coverage-audit.md`

The audit parses artifacts only. It never runs external coverage, mutation, browser, Docker, AI, hosted dashboard, or network tools.

For this repository, `just ci-coverage` is the producer lane. It writes:

- Rust LCOV: `target/llvm-cov/lcov.info` and `target/jankurai/coverage/rust-lcov.info`
- Cargo-mutants outcomes: `target/mutants/mutants.out/outcomes.json`
- Cargo-mutants diff/list receipts: `target/jankurai/coverage/mutation.diff` and `target/jankurai/coverage/mutants-list.json`

Those artifact paths are intentionally part of the local-CI contract so a fresh
clone can run the producer lane before `jankurai coverage audit` without
hand-building missing evidence files.

## Config Shape

```toml
version = 1

[[source]]
id = "rust-lcov"
kind = "line_coverage"
format = "lcov"
mode = "required"
owner = "tools"
lane = "coverage-audit"
artifacts = ["target/llvm-cov/lcov.info"]
applies_to = ["crates/**/*.rs"]
rules = ["HLT-008-FALSE-GREEN-RISK"]
hard_changed_line_coverage = 0.90
soft_total_line_coverage = 0.75

[[source]]
id = "rust-mutation"
kind = "mutation"
format = "cargo-mutants-json"
mode = "auto"
owner = "tools"
lane = "coverage-audit"
artifacts = ["target/mutants/mutants.out/outcomes.json"]
applies_to = ["crates/**/*.rs"]
rules = ["HLT-008-FALSE-GREEN-RISK"]
hard_survivors_on_changed_paths = 1
```

Allowed modes are `required`, `advisory`, `disabled`, and `auto`. `auto` is enabled only when matching paths or configured artifacts exist.

Allowed kinds are `line_coverage`, `mutation`, `property_fuzz`, `api_contract`, `ui_e2e`, `db_migration`, `container`, `supply_chain`, `dead_code`, `type_coverage`, and `jankurai_artifact`.

Allowed formats in v1 are `lcov`, `cargo-mutants-json`, `stryker-json`, `trivy-json`, `hadolint-json`, `jankurai-json`, and `generic-json-summary`.

## Audit JSON

`coverage-audit.json` contains `schema_version`, command metadata, a summary, source results, and normalized findings. Each finding carries `rule_id`, `severity`, `confidence`, `source_id`, `kind`, `artifact`, `path`, `line`, `message`, `evidence`, `repair`, `owner`, and `lane`.

Example finding:

```json
{
  "rule_id": "HLT-008-FALSE-GREEN-RISK",
  "severity": "high",
  "confidence": 0.95,
  "source_id": "rust-lcov",
  "kind": "line_coverage",
  "artifact": "target/llvm-cov/lcov.info",
  "path": "crates/app/src/lib.rs",
  "line": 42,
  "message": "uncovered changed line is reachable but not proven",
  "evidence": ["DA:42,0", "changed_from=origin/main"],
  "repair": "add or strengthen behavior tests for this changed line, rerun the producer lane, then rerun `jankurai coverage audit`",
  "owner": "tools",
  "lane": "coverage-audit"
}
```

## Repair Workflow

Fix the behavior gap in the test or proof lane that owns the source. Rerun the producer tool, then rerun `jankurai coverage audit`. For this repo, `just ci-coverage` is the canonical producer for Rust line coverage and Rust mutation evidence. For mutation survivors, strengthen assertions around the changed path and rerun the mutation lane. For supply-chain or Docker findings, fix the package, image, or Dockerfile behavior and rerun the scanner.

## Freshness And Security

Missing freshness metadata is a soft warning unless strict release policy explicitly requires it. Coverage artifacts can leak local paths, source structure, package names, and vulnerability details; keep them local unless the project explicitly publishes them.
