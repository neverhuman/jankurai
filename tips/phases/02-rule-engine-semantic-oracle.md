# Phase 02: Rule Engine And Semantic Oracle

Status: hardened
Owner: tools
Last reviewed: 2026-05-02
Parallel MCP candidate: yes

## Objective

Move Jankurai from mostly heuristic scanning toward a versioned rule engine with semantic understanding. The audit should know why a pattern violates ownership, boundary, generated-zone, contract, DB, security, UX, or agent-context policy.

The exit state is not a complete compiler-grade analyzer. The exit state is a clean rule architecture that can support AST and graph checks incrementally without breaking the existing audit.

## Current State

The current audit uses dependency-light Rust code and pattern-based scanning:

- `crates/jankurai/src/audit/mod.rs` defines weights, caps, pattern lists, dimensions, and findings.
- `crates/jankurai/src/boundaries/` has early boundary checks for Rust, TypeScript, Python, SQL, streaming, and manifest loading.
- `agent/boundaries.toml` exists.
- Report findings already carry rule-like fields: severity, category, path, evidence, `check_id`, `rule_id`, `lane`, `docs_url`, `owner`, line, matched term, reason, fingerprint, and `agent_fix`.
- **`HLT-014-A11Y-GAP`** is registered in `crates/jankurai/src/audit/rules.rs` (aligned with `agent/JANKURAI_STANDARD.md`); registry uniqueness and lookup tests in `crates/jankurai/tests/rule_registry_smoke.rs`.

The implementation is useful but centralized. Future growth needs a rule registry and analyzers with clear contracts.

## Dependencies

Must follow Phase 01 report compatibility decisions.

Inputs:

- `docs/agent-native-standard.md`
- `docs/audit-rubric.md`
- `docs/boundary-oracle.md`
- `agent/boundaries.toml`
- existing findings in `crates/jankurai/src/audit/mod.rs`

## Public Interface Changes

Allowed:

- Add internal rule structs and registry modules.
- Add machine-readable rule metadata.
- Add semantic analyzer modules behind stable audit output.
- Add new rule IDs if docs and tests are included.
- Add confidence and evidence kind refinements without removing existing fields.

Avoid:

- Removing existing caps before equivalent rules exist.
- Requiring heavyweight parsers in `just fast` until performance is measured.
- Producing findings without repair instructions.

## Target Architecture

Create a clear split:

```text
inventory -> analyzers -> rule registry -> findings -> caps -> score -> reports
```

Suggested modules:

- `audit/rules.rs`: rule metadata and registry.
- `audit/analyzers/`: optional future home for language and artifact analyzers.
- `boundaries/`: boundary-specific checks remain here or become analyzers.
- `audit/finding_builder.rs`: helper for consistent finding construction.
- `audit/caps.rs`: hard cap mapping separate from scan implementation.

Rule metadata should include:

- stable ID
- name
- category
- TLR
- default severity
- default lane
- owner routing hint
- docs URL
- evidence kind
- autofix or repair-plan eligibility
- confidence rules
- cap impact where applicable

## Workstreams

### 1. Rule Registry

Implementation tasks:

- Extract stable rule metadata from hard-coded pattern checks.
- Keep legacy caps stable while mapping them to rule metadata.
- Add docs for each stable HLT rule or a generated rule index.
- Add tests that every emitted `rule_id` exists in the registry.
- Add tests that every high/critical finding has `agent_fix`, lane, and rerun command.

Acceptance:

- A new rule can be added without editing unrelated report rendering code.
- Unknown rule IDs fail tests.
- Rule docs and metadata stay aligned.

### 2. Boundary Oracle

Implementation tasks:

- Load `agent/boundaries.toml` as a first-class audit input.
- Normalize boundary policies for Rust, TypeScript, Python, SQL, and streaming.
- Detect direct DB access from wrong layer using path plus import/content evidence.
- Detect Rust domain impurity using path plus forbidden import/function markers.
- Detect TypeScript web wrong-layer access using import and dependency markers.
- Detect Python product truth leakage using path plus DB/auth/product API markers.
- Detect streaming clients outside declared adapter paths.

Acceptance:

- Boundary findings include path, line when available, owner, lane, evidence, and exact repair.
- Kafka/streaming exceptions require owner, expiry, brownfield reason or classification, and migration path.
- False positives in docs/reference/generated/vendor paths are allowlisted.

### 3. Generated Zone Oracle

Implementation tasks:

- Parse `agent/generated-zones.toml`.
- Detect generated files lacking required headers where policy applies.
- Detect hand-edited generated zones by comparing source path, command metadata, or checksum when available.
- Detect public API contract changes without generated outputs or drift proof.
- Keep generated-output checks advisory until reproducibility data exists.

Acceptance:

- Generated findings tell agents to edit source contracts and regenerate, not patch generated files.
- Generated-zone docs explain source, command, and ownership.

### 4. AST And Graph Pilot

Implementation tasks:

- Select one low-risk semantic parser pilot.
- Prefer dependency-light parsing first: manifest/YAML/TOML/JSON plus shallow Rust/TS import extraction.
- If tree-sitter or another parser is added, measure compile time and document why.
- Build an import/dependency edge model for Rust modules and TypeScript imports.
- Add fixtures for one known violation per supported analyzer.

Acceptance:

- Semantic checks improve at least one rule with fewer false positives than text-only scanning.
- Parser failures degrade to advisory diagnostics, not audit crashes.
- Performance remains compatible with `just fast`.

### 5. Finding Quality

Implementation tasks:

- Standardize `problem`, `reason`, `evidence`, and `agent_fix` wording.
- Ensure every finding answers: what failed, why it matters, where to fix, how to prove.
- Add fingerprints that remain stable across unrelated line shifts when possible.
- Add confidence scoring conventions.

Acceptance:

- Findings are actionable without reading the whole standard.
- Repair queue ordering remains deterministic.

## Parallel MCP Breakdown

This phase is a strong parallel candidate after the rule metadata contract is sketched.

Parallel agents:

- Agent A: rule registry and metadata tests. Owns registry modules and rule docs.
- Agent B: boundary oracle. Owns `boundaries/`, fixtures, boundary docs.
- Agent C: generated-zone oracle. Owns generated-zone parser/checks and fixtures.
- Agent D: AST/import pilot. Owns analyzer module and parser fixture tests.

Shared constraints:

- No agent changes the report JSON shape without coordinating with Agent A.
- Each agent adds fixtures for its own checks.
- All agents use the same finding builder once available.

Merge order:

1. Rule registry and finding builder.
2. Boundary and generated-zone checks.
3. AST/import pilot.
4. Final report compatibility validation.

## Validation

Minimum:

```bash
cargo test -p jankurai
just fast
```

Add fixture-specific commands if new tests are created.

## Risks

- Parser dependencies can slow builds or increase maintenance burden.
- Over-eager semantic checks can create noisy false positives.
- A rule registry can become bureaucracy if it does not improve repair quality.

## Handoff Notes

Leave:

- list of rule IDs added or remapped
- analyzer inputs and outputs
- known false-positive allowlists
- performance delta for `cargo check -p jankurai`
- examples of improved findings

## Phase Status Receipt

- Phase status: partial rule engine and semantic oracle
- Files changed: `crates/jankurai/src/audit/mod.rs`, `crates/jankurai/src/audit/rules.rs`, `crates/jankurai/tests/audit_smoke.rs`, and `target/jankurai/phase-logs/02-rule-engine-semantic-oracle.md.log`
- Schemas changed: rule metadata and finding compatibility surfaces under `schemas/`
- Public interfaces changed: audit now routes stable `rule_id`, lane, TLR, docs URL, and owner hints through the registry
- Generated artifacts: none
- Routing maps changed: none in this slice
- Validation commands: `cargo test -p jankurai`, `just fast`
- Results: validation passed; analyzer split remains partial
- Skipped validation: none
- Exceptions created: analyzer split and AST/import pilot remain gated follow-on work
- Follow-up phases: 07 contracts/db/generated boundaries, 08 agent context and repair, 11 migration engine
