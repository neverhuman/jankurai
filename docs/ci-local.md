# Running CI locally

Every GitHub Actions job has a matching `just` recipe so failures are caught
on the developer's machine, not first on GitHub. Use this as the contract
between local work and the remote merge gate.

## Quick reference

| Recipe           | Mirrors GitHub job                          | Typical runtime |
| ---------------- | ------------------------------------------- | --------------- |
| `just ci-doctor` | prereq check — no CI equivalent             | < 1 s           |
| `just ci-quick`  | `jankurai / test (ubuntu-latest, macos-latest)` | 5–10 min    |
| `just ci-coverage` | `jankurai / coverage (llvm-cov)`          | 5–10 min        |
| `just ci-audit`  | `jankurai / audit`                          | 15–25 min       |
| `just ci-release`| `release / audit-gate`                      | 10–20 min       |
| `just ci`        | quick + coverage + audit                    | 25–40 min       |
| `just zizmor`    | zizmor scan portion of the security lane    | < 5 s           |

All recipes call `scripts/ci-local.sh` so the exact step sequence stays in
one place. The script halts on the first failing step (`set -euo pipefail`).

## First-time setup

```bash
just ci-doctor          # report missing tools with install hints
```

Typical installs the doctor will ask for:

```bash
rustup component add rustfmt clippy llvm-tools-preview
cargo install cargo-llvm-cov --locked
cargo install cargo-audit --locked
cargo install zizmor --locked
brew install gitleaks syft just gh jq ripgrep
brew install --cask mactex            # macOS paper build (linux: texlive)
npm ci                                # workspace dev deps
npx playwright install chromium
cargo install --path crates/jankurai --locked
```

## What each lane runs

### `just ci-quick`
The test-matrix job:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
3. `cargo test --workspace --all-targets --all-features --locked`
4. `bash scripts/render-test-surface.sh --check` — README chart is in sync

### `just ci-coverage`
The coverage-llvm job. Needs `cargo-llvm-cov` installed.

```bash
cargo llvm-cov --workspace --all-features --locked --lcov --output-path target/coverage/lcov.info
cargo llvm-cov report --json --output-path target/coverage/coverage.json
cargo llvm-cov report --summary-only
```

### `just ci-audit`
The full audit job:

1. `npm ci` and `npx playwright install chromium`
2. Quality gates (fmt, clippy, workspace tests, ux-qa build/test)
3. `cargo install --path crates/jankurai --locked --force`
4. `jankurai proofbind verify` + `jankurai proofmark rust`
5. `jankurai rust witness build`
6. `jankurai security run --strict --profile ci`
7. UX QA smoke server + `jankurai ux audit`
8. `jankurai coverage audit`
9. `cargo test -p jankurai --test language_bad_behavior`
10. `cargo test -p jankurai --test migration_prompt_verify --test migration_slice_risk`
11. `jankurai audit . --mode ratchet --baseline agent/baselines/main.repo-score.json`

### `just ci-release`
The release.yml `audit-gate` job. Optionally set `LOCAL_RELEASE_TAG=v1.0.0`
to also assert `VERSION` matches the tag.

### `just zizmor`
Static analysis of GitHub workflows. Run before pushing any `.github/workflows/`
change — caught two cache-poisoning issues in the v1.0.0 PR before they
reached CI.

## Editing the lane

`scripts/ci-local.sh` is the source of truth. When a CI workflow step
changes, mirror the change in the same script so `just ci` stays accurate.
The pre-commit hook does not yet run `just ci` (too slow); use it manually
before opening or updating a pull request.
