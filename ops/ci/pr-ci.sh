#!/usr/bin/env bash
# Canonical local-forge PR gate for internal jeryu/jankurai releases.
# The Jeryu host runner checks out the exact PR SHA and publishes
# jankurai/required from this script's real exit status.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

export CARGO_NET_OFFLINE=true
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_NOSYSTEM=1
export GIT_TERMINAL_PROMPT=0
export npm_config_offline=true

if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
  echo "[pr-ci] tracked worktree is dirty at start" >&2
  exit 1
fi

echo "[pr-ci] locked quality gates" >&2
bash ops/ci/quality-gates.sh

echo "[pr-ci] UX build and tests" >&2
npm ci --offline
npm --workspace @jankurai/ux-qa run build
npm --workspace @jankurai/ux-qa run test

echo "[pr-ci] compatibility and conformance" >&2
cargo run -p jankurai --locked -- conformance run \
  --fixtures conformance/fixtures \
  --expected conformance/expected \
  --out target/jankurai/conformance-results.json \
  --md target/jankurai/conformance-results.md \
  --tex target/jankurai/conformance-results.tex
cargo test -p jankurai conformance --locked

echo "[pr-ci] source coverage and mutation evidence" >&2
GITHUB_BASE_REF=main bash ops/ci/coverage-llvm.sh

echo "[pr-ci] exact candidate binary" >&2
cargo build -p jankurai --release --locked
candidate_bin="${CARGO_TARGET_DIR:-$repo_root/target}/release/jankurai"
[[ -x "$candidate_bin" ]] || { echo "missing candidate binary: $candidate_bin" >&2; exit 1; }
expected_version="jankurai $(tr -d '[:space:]' < VERSION)"
actual_version="$($candidate_bin --version)"
[[ "$actual_version" == "$expected_version" ]] || {
  echo "candidate version mismatch: expected '$expected_version', got '$actual_version'" >&2
  exit 1
}
export PATH="$(dirname "$candidate_bin"):$PATH"
export JANKURAI_NO_UPDATE_CHECK=1
export JANKURAI_SECURITY_OFFLINE=1
export JANKURAI_CARGO_AUDIT_DB="${JANKURAI_CARGO_AUDIT_DB:-${CARGO_HOME:-$HOME/.cargo}/advisory-db}"

echo "[pr-ci] relocation proof" >&2
bash ops/ci/relocation-test.sh

echo "[pr-ci] strict offline security" >&2
jankurai security run . --strict --profile ci --out target/jankurai/security/evidence.json

echo "[pr-ci] semantic coverage audit" >&2
jankurai coverage audit . \
  --config agent/coverage-sources.toml \
  --json target/jankurai/coverage/coverage-audit.json \
  --md target/jankurai/coverage/coverage-audit.md

echo "[pr-ci] release identity" >&2
jankurai versions
package_version="$(cargo metadata --locked --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "jankurai") | .version')"
[[ "$package_version" == "$(tr -d '[:space:]' < VERSION)" ]] || {
  echo "Cargo package/root VERSION mismatch: $package_version" >&2
  exit 1
}

echo "[pr-ci] full release audit" >&2
jankurai audit . \
  --full \
  --mode standard \
  --json target/jankurai/repo-score.json \
  --md target/jankurai/repo-score.md \
  --sarif target/jankurai/jankurai.sarif \
  --github-step-summary target/jankurai/summary.md \
  --repair-queue-jsonl target/jankurai/repair-queue.jsonl

if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
  echo "[pr-ci] tracked worktree is dirty at finish" >&2
  git status --short >&2
  exit 1
fi

echo "[pr-ci] jankurai OK" >&2
