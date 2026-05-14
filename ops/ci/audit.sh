#!/usr/bin/env bash
# Full audit lane: quality gates, proofbind, proofmark, security, UX,
# coverage, language fixtures, and the ratchet audit. Mirrors the
# jankurai / audit job exactly.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

ensure_fuse_dev

ensure_dir "${ARTIFACT_ROOT}"
ensure_dir "${ARTIFACT_ROOT}/security"
ensure_dir "${ARTIFACT_ROOT}/public"
ensure_dir "${ARTIFACT_ROOT}/coverage"

step "npm ci"
npm ci

step "Playwright browsers"
npx playwright install "${PLAYWRIGHT_BROWSER}" --with-deps || \
  npx playwright install "${PLAYWRIGHT_BROWSER}"

step "Security toolchain"
bash "$(dirname "${BASH_SOURCE[0]}")/security-tools.sh"

step "Quality gates"
bash "$(dirname "${BASH_SOURCE[0]}")/quality-gates.sh"

step "npm @jankurai/ux-qa build + test"
npm --workspace @jankurai/ux-qa run build
npm --workspace @jankurai/ux-qa run test

step "Install local jankurai"
cargo install --path crates/jankurai --locked --force

baseline="${ARTIFACT_ROOT}/accepted-baseline.json"
cp "${CI_ROOT}/agent/baselines/main.repo-score.json" "$baseline"
assert_nonempty "$baseline"

step "Proofbind verify"
jankurai proofbind verify .

step "Proofmark rust"
jankurai proofmark rust . --obligations "${ARTIFACT_ROOT}/proofbind/obligations.json" || \
  jankurai proofmark rust .

step "Rust witness build"
jankurai rust witness build .

step "Security lane (strict, ci profile)"
jankurai security run . --strict --profile ci --out "${ARTIFACT_ROOT}/security/evidence.json"
assert_nonempty "${ARTIFACT_ROOT}/security/evidence.json"

step "UX QA smoke server"
node "${CI_ROOT}/tools/ux-qa-smoke-server.mjs" > "${ARTIFACT_ROOT}/ux-qa-server.log" 2>&1 &
ux_pid=$!
trap 'kill "$ux_pid" 2>/dev/null || true' EXIT
for _ in $(seq 1 30); do
  if curl -fsS "http://127.0.0.1:3000/?ux_state=success" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

step "UX audit"
jankurai ux audit --config "${CI_ROOT}/agent/ux-qa.toml" --out "${ARTIFACT_ROOT}/ux-qa.json"
assert_nonempty "${ARTIFACT_ROOT}/ux-qa.json"

kill "$ux_pid" 2>/dev/null || true
trap - EXIT

step "Coverage audit (semantic)"
jankurai coverage audit . \
  --config "${CI_ROOT}/agent/coverage-sources.toml" \
  --json "${ARTIFACT_ROOT}/coverage/coverage-audit.json" \
  --md "${ARTIFACT_ROOT}/coverage/coverage-audit.md"
assert_nonempty "${ARTIFACT_ROOT}/coverage/coverage-audit.json"

step "Language bad-behavior fixtures"
cargo test -p jankurai --test language_bad_behavior

step "Migration evidence fixtures"
cargo test -p jankurai --test migration_prompt_verify --test migration_slice_risk

step "Final jankurai audit (ratchet)"
jankurai audit . \
  --mode ratchet \
  --baseline "$baseline" \
  --json "${ARTIFACT_ROOT}/repo-score.json" \
  --md "${ARTIFACT_ROOT}/repo-score.md" \
  --sarif "${ARTIFACT_ROOT}/jankurai.sarif" \
  --github-step-summary "${ARTIFACT_ROOT}/summary.md" \
  --repair-queue-jsonl "${ARTIFACT_ROOT}/repair-queue.jsonl" \
  --proof-receipts "${ARTIFACT_ROOT}/proofmark/proof-receipt.json"

assert_nonempty "${ARTIFACT_ROOT}/repo-score.json"
assert_nonempty "${ARTIFACT_ROOT}/repo-score.md"
assert_nonempty "${ARTIFACT_ROOT}/jankurai.sarif"
