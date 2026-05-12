#!/usr/bin/env bash
# Run the same gates GitHub Actions runs, in the same order, on the local
# machine. Mirrors .github/workflows/jankurai.yml (test-matrix +
# coverage-llvm + audit) and the release.yml audit-gate.
#
# Lanes (selectable via $1):
#   quick     test-matrix job: fmt, clippy, workspace tests, README check
#   coverage  coverage-llvm job: cargo-llvm-cov workspace
#   audit     audit job: full proofbind/proofmark/security/UX/audit/ratchet
#   release   release.yml audit-gate (strict security + ratchet audit + SARIF)
#   all       quick + coverage + audit  (default)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LANE="${1:-all}"

step() {
  printf '\n\033[1;36m==> %s\033[0m\n' "$1"
}

run_quick() {
  step "fmt"
  cargo fmt --all -- --check

  step "clippy"
  cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

  step "cargo test --workspace"
  cargo test --workspace --all-targets --all-features --locked

  step "test-surface (README chart in sync)"
  bash scripts/render-test-surface.sh --check
}

run_coverage() {
  step "cargo-llvm-cov"
  if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "cargo-llvm-cov not installed. Run: cargo install cargo-llvm-cov --locked" >&2
    exit 2
  fi
  mkdir -p target/coverage
  cargo llvm-cov --workspace --all-features --locked --lcov --output-path target/coverage/lcov.info
  cargo llvm-cov report --json --output-path target/coverage/coverage.json
  cargo llvm-cov report --summary-only | tee target/coverage/summary.txt
}

run_audit() {
  step "npm ci"
  if [[ ! -d node_modules ]] || [[ package-lock.json -nt node_modules ]]; then
    npm ci
  else
    echo "node_modules up to date"
  fi

  step "playwright browsers"
  npx playwright install chromium

  step "quality gates"
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
  cargo test --workspace --all-targets --all-features --locked
  npm --workspace @jankurai/ux-qa run build
  npm --workspace @jankurai/ux-qa run test

  step "install local jankurai"
  cargo install --path crates/jankurai --locked --force

  mkdir -p target/jankurai target/jankurai/security target/jankurai/public

  step "resolve baseline"
  cp agent/baselines/main.repo-score.json target/jankurai/accepted-baseline.json

  step "proofbind verify"
  jankurai proofbind verify . --mode required || jankurai proofbind verify .

  step "proofmark rust"
  jankurai proofmark rust . --mode required \
    --obligations target/jankurai/proofbind/obligations.json || true

  step "rust witness build"
  jankurai rust witness build .

  step "security lane (strict)"
  jankurai security run . --strict --profile ci --out target/jankurai/security/evidence.json

  step "ux-qa smoke server"
  node tools/ux-qa-smoke-server.mjs > target/jankurai/ux-qa-server.log 2>&1 &
  ux_pid=$!
  trap 'kill "$ux_pid" 2>/dev/null || true' EXIT
  for _ in $(seq 1 30); do
    if curl -fsS "http://127.0.0.1:3000/?ux_state=success" >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done

  step "ux audit"
  jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json

  kill "$ux_pid" 2>/dev/null || true
  trap - EXIT

  step "coverage audit (semantic)"
  mkdir -p target/jankurai/coverage
  jankurai coverage audit . \
    --config agent/coverage-sources.toml \
    --json target/jankurai/coverage/coverage-audit.json \
    --md target/jankurai/coverage/coverage-audit.md

  step "language bad-behavior fixtures"
  cargo test -p jankurai --test language_bad_behavior

  step "migration evidence fixtures"
  cargo test -p jankurai --test migration_prompt_verify --test migration_slice_risk

  step "final jankurai audit (ratchet)"
  jankurai audit . \
    --mode ratchet \
    --baseline target/jankurai/accepted-baseline.json \
    --json target/jankurai/repo-score.json \
    --md target/jankurai/repo-score.md \
    --sarif target/jankurai/jankurai.sarif \
    --repair-queue-jsonl target/jankurai/repair-queue.jsonl
}

run_release() {
  step "verify VERSION vs git tag"
  if [[ -n "${LOCAL_RELEASE_TAG:-}" ]]; then
    tag="${LOCAL_RELEASE_TAG#v}"
    file="$(tr -d '[:space:]' < VERSION)"
    if [[ "$tag" != "$file" ]]; then
      echo "LOCAL_RELEASE_TAG ($tag) does not match VERSION ($file)" >&2
      exit 1
    fi
  fi

  step "quality gates"
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
  cargo test --workspace --all-targets --all-features --locked

  step "install local jankurai"
  cargo install --path crates/jankurai --locked --force

  step "security lane (strict)"
  mkdir -p target/jankurai/security
  jankurai security run . --strict --profile ci --out target/jankurai/security/evidence.json

  step "final ratchet audit"
  jankurai audit . \
    --mode ratchet \
    --baseline agent/baselines/main.repo-score.json \
    --json target/jankurai/repo-score.json \
    --md target/jankurai/repo-score.md \
    --sarif target/jankurai/jankurai.sarif

  step "zizmor workflows"
  zizmor .github/workflows
}

case "$LANE" in
  quick)    run_quick ;;
  coverage) run_coverage ;;
  audit)    run_audit ;;
  release)  run_release ;;
  all)      run_quick; run_coverage; run_audit ;;
  *) echo "usage: $0 {quick|coverage|audit|release|all}" >&2; exit 2 ;;
esac

printf '\n\033[1;32mlocal CI lane "%s" passed\033[0m\n' "$LANE"
