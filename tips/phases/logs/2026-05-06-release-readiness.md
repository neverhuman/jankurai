# 2026-05-06 Release Readiness

## Start Receipts

- `rtk jankurai update --client-start --quiet`: pass
- `rtk git status --short`: dirty tree with scoring/security/proof hardening, CI/badge/docs updates, user-owned prose scan work, and untracked baseline/prose smoke files.
- `rtk git diff --stat`: 42 tracked files changed, 1179 insertions, 440 deletions before baseline bootstrap.
- `rtk cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: failed on 3 mechanical lints:
  - `crates/tuiwright/src/input.rs`: manual ASCII range check
  - `crates/tuiwright/src/render.rs`: `len() > 0`
  - `crates/jankurai-proofmark/src/coverage.rs`: manual `unwrap_or_default`

## Dirty Tree Classification

- Prose scan preservation: `crates/jankurai/src/audit/prose.rs`, `helpers.rs`, `scan.rs`, `audit_smoke.rs`, `docs/audit-rubric.md`.
- Audit enforcement and ratchet: `crates/jankurai/src/main.rs`, `model.rs`, `audit/mod.rs`, `audit/baseline.rs`, enforcement and baseline smoke tests, `schemas/repo-score.schema.json`.
- Security and proof evidence: `commands/security.rs`, `audit/security_artifact.rs`, proofbind/proofmark crates and tests, `schemas/security-evidence.schema.json`, `agent/security-policy.toml`, `agent/proof-lanes.toml`.
- CI, badge, docs: `.github/workflows/jankurai.yml`, `action.yml`, `commands/ci.rs`, `commands/badge.rs`, `agent/badge.toml`, `README.md`, `docs/testing.md`, `docs/artifact-contracts.md`, `Justfile`.
- Clippy cleanup: `crates/tuiwright/src/input.rs`, `crates/tuiwright/src/render.rs`, `crates/jankurai-proofmark/src/coverage.rs`.

## Tip Reconciliation

- Evidence-terminal scoring: CI now runs quality, proof, strict security, UX, and fixture evidence before final audit.
- No candidate self-baseline: CI resolves `agent/baselines/main.repo-score.json` from protected main or committed baseline.
- Fail-closed ratchet: baseline parsing and ratchet mode are covered by new audit enforcement tests.
- Strict security/proof lanes: CI uses `--strict --profile ci` and proof lanes use `--mode required`.
- CI hardening: workflow has explicit permissions, concurrency, timeouts, SHA-pinned official actions, and SARIF upload.
- Badge integrity: badge config points at the accepted baseline, not ignored local score output.
- Archive hygiene: `._*` is ignored and final cleanup will scan for sidecars before handoff.

## Implementation Receipts

- Clippy lint fixes applied to the three reported mechanical issues. Full clippy rerun pending.
- `rtk cargo fmt --all -- --check`: pass
- `rtk cargo check --workspace --locked`: pass
- `rtk cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass
- Focused scoring/security/badge/CI tests: pass, 51 tests across 8 suites
- `rtk cargo test -p jankurai-proofbind`: pass
- `rtk cargo test -p jankurai-proofmark`: pass
- `rtk cargo test -p jankurai`: pass, 381 tests
- `rtk cargo test --workspace --all-targets --all-features --locked`: pass, 403 tests
- `rtk npm ci`: pass
- `rtk npm --workspace @jankurai/ux-qa run build`: pass
- `rtk npm --workspace @jankurai/ux-qa run test`: pass, 20 tests
- `rtk just conformance`: pass
- `rtk just security-strict`: initially failed closed on missing/failing `zizmor`; installed `zizmor` 1.24.1 and fixed checkout credential persistence; pass
- `rtk just paper`: pass
- `rtk just check`: pass
- `rtk git diff --check`: pass
- `rtk cargo install --path crates/jankurai --locked --force`: pass; refreshed installed `jankurai` 0.8.9 before commit hook
- Commit `be636f3` (`Harden scoring integrity for release`) created from the green tree.

## Baseline Receipts

- Clean-tree guard before baseline: `rtk git diff --quiet` pass; `rtk git diff --cached --quiet` pass.
- Standard audit to `target/jankurai/repo-score.json`: pass, `score=97 raw=97 caps=0 findings=0`, `dirty_worktree=false`.
- Accepted baseline report fingerprint: `sha256:34e19fd99e94166a1fffbf52bcca2cf7e1d2454fb8e739f0638e0f929771b1b9`.
- Accepted baseline input fingerprint: `sha256:1e6d95b4c1d43a8aba27d2defd1dfb799d5279d746417951beb8759a0f6646a7`.
- Accepted baseline policy fingerprint: `sha256:4cada2563bc061cb649c364949b0bb3e2460a6702c088681bc2eb6a31f9b482a`.
- `agent/baselines/main.repo-score.json` sha256: `89bf2b6f962fbc85bd321539d83736aa89cf02b33977e35cf6d38163610d2d6d`.
- `agent/jankurai-badge.json` sha256: `af71981056f26b40fa2ae0513783e67ddf38fe4e52c89df974069c2855d49955`.
- `agent/jankurai-badge.svg` sha256: `e750b114e3928338da9a8c480a94aac356e062a1882b95bb1c74b083f7b36b2d`.
- `rtk cargo run -p jankurai -- badge ... --check`: pass, badge current against `agent/baselines/main.repo-score.json`.
- Commit `d87b208` (`Exclude accepted baselines from secret scanning`) fixed baseline self-scan recursion before this final accepted baseline was regenerated.

## Final Release Receipts

- Final commits:
  - `be636f3` `Harden scoring integrity for release`
  - `878f317` `Bootstrap accepted scoring baseline`
  - `d87b208` `Exclude accepted baselines from secret scanning`
  - `117b181` `Refresh accepted baseline after scanner fix`
- Final ratchet audit: `rtk jankurai audit . --mode ratchet --baseline agent/baselines/main.repo-score.json ... --no-score-history`, pass with `score=97 raw=97 caps=0 findings=0`, `dirty_worktree=false`, `score_delta=0`.
- UX evidence generated with a temporary localhost fixture server and matching visual baselines under `target/jankurai/ux-qa-baselines`; final `target/jankurai/ux-qa.json` reports 10 matched visual baselines.
- Final artifact presence check: pass for repo score JSON/Markdown, SARIF, summary, repair queue, audit timings, strict security evidence, proofbind/proofmark receipts, UX evidence, conformance results, and `paper/jankurai.pdf`.
- Final cleanup: `rtk git diff --check` pass; no `._*`, `.DS_Store`, swap, temp, or backup sidecars outside `target/`; `rtk git status --short` clean before this receipt update.
