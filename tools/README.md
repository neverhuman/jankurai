# jankurai Repo Scorer

Use the installed `jankurai` command or `cargo run -p jankurai --` to score a repository against jankurai `0.3.0`.

Target stack only: Rust core + TypeScript/React/Vite + PostgreSQL + generated contracts + exception-only Python AI/data service. This is not a generic linter.

## Contract

- Fast Rust binary with low-overhead filesystem traversal
- No Python runtime dependency for the audit lane
- Agents must not add Python to repo tools, proof lanes, product services, or backend glue
- Runs on arbitrary checkouts without bootstrap drama
- Emits JSON and Markdown
- Reads repo structure and local evidence, not build artifacts
- Does not mutate the target repo
- Produces actionable `agent_fix_queue` items
- Carries hard caps for severe agent-native failures

## Usage

```bash
jankurai /path/to/repo --json agent/repo-score.json --md agent/repo-score.md
cargo run -p jankurai -- /path/to/repo --json agent/repo-score.json --md agent/repo-score.md
cargo run -p jankurai -- /path/to/repo --changed src/foo.rs contracts/api.yaml
```

Install the checkout-local command with:

```bash
cargo install --path crates/jankurai --locked
```

Or install the package entrypoint:

```bash
cargo install --git https://github.com/jeppsontaylor/jankurai --package jankurai --locked
jankurai /path/to/repo --json agent/repo-score.json --md agent/repo-score.md
```

## CI

Run the scorer in every PR:

```bash
cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md
```

Upload both files. The JSON is the machine contract; the Markdown is the human review surface. Teams can fail CI on score, caps, or selected severities.

## Output

- `standard_version`
- `auditor_version`
- `schema_version`
- `paper_edition`
- `target_stack_id`
- `target_stack`
- `score`
- `raw_score`
- `caps_applied`
- `hard_rules`
- `dimensions`
- `findings`
- `agent_fix_queue`

## Strict Checks

The scorer flags known vibe-coding failure modes:

| Category | Hard Signals |
| --- | --- |
| stack drift | unnecessary runtime languages, any new Python without a dated advanced-ML/data exception |
| code shape | duplication, mega files, mega functions, weak names, junk drawers |
| placeholders | TODO/FIXME/HACK/XXX, stubs, placeholders, unimplemented/unreachable/TODO panics |
| fallbacks | fallback soup, best-effort retries, broad catch/except, null/undefined fallbacks |
| contracts | handwritten DTO/API types, handwritten web API clients, missing generated clients, drift untested |
| generated zones | missing generated-zone manifest, missing do-not-edit markers, TODOs in generated code |
| data | direct DB access from web/API/domain/application/Python product code |
| tests | no Playwright/e2e for web, no rendered UX receipt, no Rust property tests, skipped/focused/tautological/snapshot-only proof |
| security | no security lane, secret-like content, prompt injection, overbroad agent agency, no dependency/SBOM/provenance scan |
| migrations | destructive SQL without rollback/backfill/lock safety evidence |
| docs | missing root instructions, missing architecture/boundary/testing docs |
| exceptions | no agent-friendly errors with name/code/purpose/reason/common fixes/docs URL |

See `docs/audit-rubric.md` for the full scoring contract.
