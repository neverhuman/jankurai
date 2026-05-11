# Coverage Evidence Master

Jankurai treats coverage as behavioral proof support, not as a score category. Line coverage says code was reachable. Mutation, property, integration, API, database, UX, accessibility, container, and supply-chain evidence are stronger behavior signals because they show what fails when behavior changes or unsafe surfaces drift.

`jankurai coverage audit` is the fast path. It parses local artifacts listed in `agent/coverage-sources.toml`, normalizes them into `target/jankurai/coverage/coverage-audit.json`, and writes a Markdown companion. It does not run tests, browsers, containers, hosted dashboards, AI tools, or network calls.

Hard evidence gaps route through existing HLT rules. Missing or weak behavioral proof routes to `HLT-008-FALSE-GREEN-RISK`; unmapped changed paths route to `HLT-004-UNMAPPED-PROOF`; UX and accessibility gaps route to `HLT-013` and `HLT-014`; supply-chain and Docker evidence routes to `HLT-016` and `HLT-032`; database, authorization, and input-boundary proof gaps route to `HLT-021`, `HLT-022`, and `HLT-023`.

## Hard Path

Required sources can create hard findings when changed applicable code lacks deterministic proof, changed lines are uncovered, mutation survivors remain on changed paths, or configured security/container thresholds are crossed. Every hard finding must include a concrete repair action, owner, lane, artifact, and path or line when available.

## Soft Path

Advisory sources, missing optional tools, stale or absent freshness metadata, low global line coverage, and hosted dashboard gaps are soft evidence. Missing optional coverage tools cannot block merge. Unknown freshness is a warning unless a source is explicitly required and strict mode is requested.

## Agent Workflow

1. Find the relevant source in `agent/coverage-sources.toml`.
2. Run the producer lane outside the audit, such as `cargo llvm-cov`, `cargo mutants`, Vitest, Stryker, Trivy, or Hadolint.
3. Run `cargo run -p jankurai -- coverage audit . --config agent/coverage-sources.toml --json target/jankurai/coverage/coverage-audit.json --md target/jankurai/coverage/coverage-audit.md`.
4. Inspect findings and repair the test, mutation, contract, UX, DB, container, or security gap.
5. Rerun the producer lane and coverage audit.
6. Attach the JSON/Markdown artifacts or a proof receipt that references them.

Anti-false-positive rule: do not claim behavior proof from line coverage alone. Phrase line-only gaps as reachability evidence, such as uncovered changed line or reachable but not proven.
