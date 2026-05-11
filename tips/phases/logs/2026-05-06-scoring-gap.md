# 2026-05-06 Scoring Gap Hardening

## Phase 0 Receipts

Commands run before implementation:

- `rtk git status --short`
  - `M crates/jankurai/src/audit/helpers.rs`
  - `M crates/jankurai/src/audit/mod.rs`
  - `M crates/jankurai/src/audit/scan.rs`
  - `M crates/jankurai/tests/audit_smoke.rs`
  - `M docs/audit-rubric.md`
  - `?? crates/jankurai/src/audit/prose.rs`
- `rtk git diff --stat`
  - `crates/jankurai/src/audit/helpers.rs |  92 ++++++++++++++--------------`
  - `crates/jankurai/src/audit/mod.rs     |   5 +-`
  - `crates/jankurai/src/audit/scan.rs    | 113 +++++++++++++++++------------------`
  - `crates/jankurai/tests/audit_smoke.rs | 106 ++++++++++++++++++++++++++++++++`
  - `docs/audit-rubric.md                 |   2 +`
  - `5 files changed, 212 insertions(+), 106 deletions(-)`
- `rtk just fast`
  - `score=100 raw=100 caps=0 findings=0`
- `rtk cargo test -p jankurai --test audit_smoke prose`
  - `2 passed, 40 filtered out`
- `rtk find . -name '._*'`
  - no AppleDouble files found
- `rtk find . -name '.DS_Store'`
  - no `.DS_Store` files found

## Notes

- Existing prose-neutral scan changes are preserved as user-owned work.
- `._*` is now ignored at repo root.

## Supply Chain Pins

Resolved with `rtk git ls-remote`:

- `actions/checkout` `refs/tags/v6`: `de0fac2e4500dabe0009e67214ff5f5447ce83dd`
- `actions/setup-node` `refs/tags/v6`: `48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e`
- `actions/upload-artifact` `refs/tags/v7`: `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a`
- `github/codeql-action` `refs/tags/v3`: `53e96ec3b35fce51c141c0d6f0e31028a448722d`

## Implementation Receipts

- `rtk cargo check --workspace --locked`: pass
- `rtk cargo test -p jankurai --test audit_enforcement_smoke`: pass
- `rtk cargo test -p jankurai --test baseline_ratchet_smoke`: pass
- `rtk cargo test -p jankurai --test security_evidence_smoke`: pass
- `rtk cargo test -p jankurai --test security_evidence_audit_ingest_smoke`: pass
- `rtk cargo test -p jankurai --test security_lane_wrapper_smoke`: pass
- `rtk cargo test -p jankurai --test action_metadata_smoke`: pass
- `rtk cargo test -p jankurai --test language_bad_behavior`: pass
- `rtk cargo test -p jankurai-proofbind`: pass
- `rtk cargo test -p jankurai-proofmark`: pass
- `rtk cargo test -p jankurai`: pass, `381 passed (54 suites)`
- `rtk cargo fmt --all -- --check`: pass
- `rtk cargo check --workspace --locked`: pass
- `rtk just fast`: pass, `score=97 raw=97 caps=0 findings=0`; badge update skipped because `agent/baselines/main.repo-score.json` is intentionally not bootstrapped from the dirty candidate tree.
- `rtk cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: fail on pre-existing workspace lint debt across `tuiwright`, `jankurai`, and proof crates; not completed in this false-green hardening pass.
