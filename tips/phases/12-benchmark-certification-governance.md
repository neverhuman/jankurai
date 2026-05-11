# Phase 12: Benchmark Certification And Governance

Status: hardened
Owner: standard
Last reviewed: 2026-05-03
Parallel MCP candidate: yes

## Objective

Prove the Jankurai thesis publicly and make conformance meaningful. This phase builds benchmark corpora, certification artifacts, badges, rule governance, and **machine-readable public evidence**.

The exit state is that “Built with Jankurai” has **schema-valid, CI-publishable** evidence (bundle + badge surfaces) behind it. Hosted dashboards remain optional consumption; they are **not** a completion dependency.

## Current State

Hardened pieces:

- `jankurai bench` emits a schema-valid `BenchmarkReport` with a bundled smoke suite built from `examples/legacy-node-api/` and `examples/perfect-web-api-db/`.
- `jankurai certify` emits a schema-valid `Certification` artifact tied to the live repo score when present and otherwise falls back to an explicit missing-score state.
- `jankurai govern` emits a schema-valid `GovernancePolicy` from the standard manifest.
- **`jankurai publish`** validates the three artifacts above (`--certification`, `--benchmark`, `--governance`), checks `standard_version` against `agent/standard-version.toml`, computes a deterministic **triple-file subject digest**, and emits optional:
  - `PublicEvidenceBundle` JSON (`schemas/public-evidence-bundle.schema.json`)
  - Badge JSON (`schemas/certification-badge.schema.json`)
  - Badge SVG (static shields-style markup)
  - Public Markdown summary
- **Publishability**: the bundle is always schema-valid; `publishable` is `false` when score/governance gates, benchmark failures, inconclusive benchmark tasks, hard caps, or critical/high findings block a “clean” badge (see bundle `blocking_reasons` and `public_status`: `publishable` | `advisory` | `blocked`).
- CI (`.github/workflows/jankurai.yml`) runs Phase 12 after the security lane and uploads evidence under `target/jankurai/` plus `target/jankurai/public/`.
- `just phase12` reproduces the same pipeline locally.

**Deferred (not MVP for Phase 12):** GitHub Artifact Attestations (`actions/attest`), Sigstore/KMS identity signing, keyed attestation `--verify-attestation`, and hosted org dashboards—the bundle is structured so those can wrap the same files later.

## Dependencies

Requires phases 01 through 11 to produce enough real surfaces to benchmark.

## Public interface

```bash
jankurai bench . --out target/jankurai/p12-benchmark-report.json --md target/jankurai/p12-benchmark-report.md
jankurai certify . --out target/jankurai/p12-certification.json --md target/jankurai/p12-certification.md
jankurai govern . --out target/jankurai/p12-governance-policy.json --md target/jankurai/p12-governance-policy.md

jankurai publish . \
  --certification target/jankurai/p12-certification.json \
  --benchmark target/jankurai/p12-benchmark-report.json \
  --governance target/jankurai/p12-governance-policy.json \
  --out target/jankurai/public/p12-public-evidence.json \
  --md target/jankurai/public/p12-public-evidence.md \
  --badge-json target/jankurai/public/jankurai-badge.json \
  --badge-svg target/jankurai/public/jankurai-badge.svg
```

Shortcut:

```bash
just phase12
```

Defaults for `--certification`, `--benchmark`, and `--governance` match `target/jankurai/p12-*.json` when emitted by the canonical paths above.

## Contract slice

Existing:

- `benchmark-suite.schema.json`, `benchmark-report.schema.json`
- `certification.schema.json`
- `governance-policy.schema.json`

New:

- **`public-evidence-bundle.schema.json`**: aggregates identity, summaries, badge, artifact index, validation commands, attestation envelope, blocking reasons.
- **`certification-badge.schema.json`**: shields-style badge JSON (label/message/color enums).

Attestation semantics: `attestation.signature` prefixes `local-sha256-attestation:` over the deterministic subject digest; `signing_key_hint` explains this is triple-file hashing, not asymmetric crypto.

## Validation

Focused:

```bash
cargo test -p jankurai --test phase_12_public_evidence
```

Broad:

```bash
cargo test -p jankurai
just fast
just score
```

After schema or CLI changes:

```bash
just compat
cargo run -p jankurai -- lane . \
  --changed crates/jankurai/src/commands/publish.rs \
  --changed crates/jankurai/src/main.rs \
  --changed crates/jankurai/src/validation.rs \
  --changed schemas/public-evidence-bundle.schema.json \
  --changed schemas/certification-badge.schema.json \
  --out target/jankurai/p12-public-evidence-lane.json \
  --md target/jankurai/p12-public-evidence-lane.md
```

## Closeout artifacts

Expect under `target/jankurai/`:

- `p12-benchmark-report.{json,md}`
- `p12-certification.{json,md}`
- `p12-governance-policy.{json,md}`
- `public/p12-public-evidence.{json,md}`
- `public/jankurai-badge.{json,svg}`

Plus receipts from lanes and audits as documented in MASTER_PLAN.

## Residual risk

- Forks or minimal fixtures may classify as `advisory` or `blocked` while remaining useful for CI (**by design**).
- External signing and hosted dashboards are optional follow-ons, not prerequisites for conformance of the emitted JSON.
