# Phase 04: Init Profiles And Golden Repos

Status: complete
Owner: tools
Last reviewed: 2026-05-04
Parallel MCP candidate: yes after generator contract is locked

## Objective

Turn `jankurai init` from a control-file installer into a profile-driven repo generator. This is the phase where Jankurai starts becoming a creation layer, not only an audit layer.

The exit state is not every possible template. The exit state is a robust generator contract, one excellent default profile, and golden repo fixtures that prove generated repos are audit-ready.

## Current State

Existing implementation:

- `jankurai init` supports `--profile`, **`--profile-file`** (validated `InitProfile` JSON; resolution ignores bundled `--profile` when set), `--ide`, `--mode`, `--ci`, `--issue-backend`, `--ux-qa`, `--dry-run`, `--diff`, `--plan-json`, `--yes`, and `--apply`.
- Adopter templates are external-repo safe: generated workflows, proof lanes, generated-zone commands, profile validation commands, and scaffold `Justfile` recipes call the installed `jankurai` binary instead of `cargo run -p jankurai` or `cargo test -p jankurai`.
- Generated scaffold proof lanes no longer use `true`/noop commands as false-green proof; advisory audit, doctor, score, security, and check recipes are present when referenced by generated maps.
- `jankurai adopt` is the brownfield front door for no-tracked-write adoption planning; `jankurai ci install --github --mode observe --dry-run` previews non-blocking CI.
- Init plan JSON field **`profile`** is always the manifest **`id`** (canonical bundled id or id read from `--profile-file`), not a CLI alias string.
- **`rust-ts-postgres` profile** is loaded from bundled [`crates/jankurai/templates/profiles/rust-ts-postgres.json`](../../crates/jankurai/templates/profiles/rust-ts-postgres.json), validated with **`ArtifactSchema::InitProfile`** before use.
- **Plan and apply** iterate **`generatedPaths`** from that manifest only (sorted); missing templates are a hard error at plan time.
- Unknown profile IDs are rejected with a message listing bundled profile IDs (see `BUNDLED_PROFILE_IDS` in `crates/jankurai/src/init/profiles.rs`).
- Templates live in `crates/jankurai/src/init/templates.rs` (plus `include_str!` agent files under `crates/jankurai/templates/agent/`).
- **Brownfield behavior** is suffix-heuristic in `crates/jankurai/src/init/plan.rs`: existing **`.json`** → additive **`merge-json`**; **`.toml`** → **`merge-toml`**; **`.gitignore`** / **`Justfile`** → **`merge-lines`** (deduped line append); **`AGENTS.md`** and **`agent/JANKURAI_STANDARD.md`** → **`merge-marker`**; other existing paths → **`keep-existing`**. Implementation lives in `crates/jankurai/src/init/merge.rs`; apply path in `crates/jankurai/src/commands/init.rs`.
- Golden tests in `crates/jankurai/tests/init_golden.rs` cover unknown profile, plan/action consistency, greenfield `audit` + `doctor --fail-on high`, preserving an existing `contracts/README.md`, JSON/TOML merge, and `Justfile` line merge where the profile lists `Justfile`.
- Operational handoff log: [`tips/phases/logs/04-init-profiles-golden-repos.log`](../logs/04-init-profiles-golden-repos.log).

Gaps (follow-on):

- Optional: manifest-driven merge policy (instead of hardcoded suffix rules), additional formats, or richer merges when product demand is clear.
- Optional: package-manager/Homebrew/GitHub Action distribution; current public install docs still assume a Cargo-based binary install.

Bundled profiles (2026-05-02): `rust-ts-postgres`, `rust-api`, `react-web`, `b2b-saas`, `ai-product`, `regulated-saas`, `migration-target`, plus aliases (`ai`, `regulated`, `migration`, and existing stack-name aliases for `rust-ts-postgres`).

## Dependencies

Requires Phase 01 stabilization.

Benefits from Phase 03 proof router for generated repo validation.

## Public Interface Changes

Profiles to support, in order:

```bash
jankurai init --profile rust-api
jankurai init --profile react-web
jankurai init --profile rust-ts-postgres
jankurai init --profile b2b-saas
jankurai init --profile ai-product
jankurai init --profile regulated-saas
jankurai init --profile migration-target
```

Start with `rust-ts-postgres` if only one can be completed.

Profile manifest fields:

- profile ID
- display name
- target stack ID
- generated paths
- required lanes
- optional lanes
- included agent adapters
- included CI templates
- included docs
- included security controls
- included UX controls
- included contract system
- included DB policy
- validation commands

## Workstreams

### 1. Generator Contract

Implementation tasks:

- Define a profile manifest schema.
- Separate template metadata from hard-coded Rust constants where practical.
- Keep template rendering deterministic.
- Preserve dry-run and diff behavior.
- Add generated adapter markers where files are safe to refresh.
- Add clear merge-marker policy for user-owned files.

Acceptance:

- `init --dry-run --plan-json` emits all planned actions and selected profile metadata.
- Existing files are not overwritten unless generated and explicitly refreshable.
- New paths appear in owner/test maps.

### 2. Default Profile: `rust-ts-postgres`

Generated repo should include at minimum:

- root `AGENTS.md`
- `agent/JANKURAI_STANDARD.md`
- owner map
- test map
- generated zones
- proof lanes
- audit policy
- standard version
- basic `Justfile`
- GitHub workflow
- Rust workspace skeleton
- TypeScript/Vite app placeholder or documented slot
- `contracts/` skeleton
- `db/migrations/` skeleton
- `docs/architecture/`
- `docs/decisions/`
- `docs/exceptions/`
- security docs or config placeholders
- UX QA config if web surface is included

Acceptance:

- Generated repo can run `jankurai audit` and produce a score report.
- Generated repo has no missing root control files.
- Generated repo docs clearly mark scaffold placeholders that must be replaced before production.

### 3. Golden Repo Fixtures

Implementation tasks:

- Create fixture repos under an allowed test fixture path, not `reference/` unless explicitly treated as source material.
- Add one "minimal generated repo" fixture.
- Add one "existing repo with partial files" fixture.
- Add tests that run init dry-run and apply into tempdirs.
- Assert generated paths and no overwrite behavior.

Acceptance:

- Tests prove `init` is idempotent.
- Tests prove generated adapters can be refreshed when marker and flag allow it.
- Tests prove profile manifests parse.

### 4. Profile Expansion

After default profile is stable, add profiles:

- `rust-api`: Rust API, agent maps, security, contract hooks, no web UX lane by default.
- `react-web`: TypeScript/React/Vite, generated client slots, UX QA, web tests, no DB ownership.
- `b2b-saas`: fullstack stack plus auth/org/audit/admin placeholders and SOC-ready evidence shell.
- `ai-product`: bounded Python service, eval harness docs, prompt/version/eval policy, no product truth.
- `regulated-saas`: stricter evidence shell, PII classification, backup/restore, exception expiry.
- `migration-target`: containment docs, boundary maps, migration evidence slots.

Acceptance:

- Each profile declares which lanes are required.
- Each profile avoids installing irrelevant tools by default.

### 5. Documentation

Implementation tasks:

- Document profile selection.
- Document generated files versus source files.
- Document safe rerun behavior.
- Add examples for greenfield and existing repo adoption.

Acceptance:

- A founder can understand which profile to choose.
- A coding agent can safely rerun init without broad damage.

## Parallel MCP Breakdown

Parallel after manifest schema is locked:

- Agent A: generator/profile manifest core. Owns Rust init modules.
- Agent B: template content. Owns `crates/jankurai/templates/` and generated profile docs.
- Agent C: tests and golden fixtures. Owns init tests.
- Agent D: docs. Owns install/profile docs.

Merge order:

1. Manifest schema and generator contract.
2. Default profile template.
3. Tests.
4. Additional profiles.
5. Docs final pass.

## Validation

Minimum:

```bash
cargo test -p jankurai
just fast
```

Profile smoke:

```bash
jankurai init --profile rust-ts-postgres --dry-run --plan-json target/jankurai/init-plan.json
jankurai init --profile rust-ts-postgres --diff
```

If tempdir apply tests are added, ensure they run under:

```bash
cargo test -p jankurai init
```

## Risks

- Templates can become large and fragile if they are not manifest-driven.
- Generated repos can overpromise by including placeholders that look production-ready.
- Installing too many tools in every profile violates the no-sprawl law.

## Handoff Notes

Leave:

- profile manifest schema
- list of supported profiles
- generated path inventory
- idempotency test names
- known profile limitations
- exact generated repo score for the default profile

## Phase Status Receipt

- Phase status: hardened for first-hour adoption (bundled init profiles 2026-05-02; all seven profiles + golden tests; external-repo-safe generated commands; no-write adoption plan; observe-mode CI)
- Files changed: `crates/jankurai/src/init/profiles.rs`, `crates/jankurai/src/init/templates.rs`, `crates/jankurai/templates/profiles/ai-product.json`, `regulated-saas.json`, `migration-target.json`, `crates/jankurai/tests/init_golden.rs`, `crates/jankurai/src/commands/repair_apply.rs` (ProveArgs plan paths), `tips/phases/04-init-profiles-golden-repos.md`, `tips/phases/logs/04-init-profiles-golden-repos.log`
- Schemas changed: `InitProfile` artifact validation hook (existing `init-profile.schema.json`)
- Public interfaces changed: unknown init profiles error; init plan/actions match `generatedPaths` only
- Routing maps changed: embedded template `agent/owner-map.json`, `agent/test-map.json`, `agent/proof-lanes.toml`
- Validation commands: `cargo test -p jankurai`, `just fast`
- Results: see `tips/phases/logs/04-init-profiles-golden-repos.log`
- Skipped validation: none
- Exceptions created: none; seven bundled profiles plus aliases documented in phase body
- Follow-up phases: 09 reference product platform, 10 reuse registry certified cells
