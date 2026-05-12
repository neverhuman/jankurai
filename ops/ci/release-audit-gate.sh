#!/usr/bin/env bash
# Release audit gate: tag-vs-VERSION check, quality gates, strict security
# lane, ratchet audit, SARIF artifact. Mirrors release.yml#audit-gate.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

ensure_dir "${ARTIFACT_ROOT}"
ensure_dir "${ARTIFACT_ROOT}/security"

if [[ -n "${RELEASE_TAG:-}" ]]; then
  step "Verify RELEASE_TAG matches VERSION"
  tag="${RELEASE_TAG#v}"
  file="$(read_version)"
  if [[ "$tag" != "$file" ]]; then
    fail "RELEASE_TAG (${RELEASE_TAG} -> ${tag}) does not match VERSION (${file})"
  fi
fi

step "Security toolchain"
bash "$(dirname "${BASH_SOURCE[0]}")/security-tools.sh"

step "Quality gates"
bash "$(dirname "${BASH_SOURCE[0]}")/quality-gates.sh"

step "Install local jankurai"
cargo install --path crates/jankurai --locked --force

step "Security lane (strict, ci profile)"
jankurai security run . --strict --profile ci --out "${ARTIFACT_ROOT}/security/evidence.json"

step "Ratchet audit"
jankurai audit . \
  --mode ratchet \
  --baseline "${CI_ROOT}/agent/baselines/main.repo-score.json" \
  --json "${ARTIFACT_ROOT}/repo-score.json" \
  --md "${ARTIFACT_ROOT}/repo-score.md" \
  --sarif "${ARTIFACT_ROOT}/jankurai.sarif"

assert_nonempty "${ARTIFACT_ROOT}/repo-score.json"
assert_nonempty "${ARTIFACT_ROOT}/repo-score.md"
assert_nonempty "${ARTIFACT_ROOT}/jankurai.sarif"
assert_nonempty "${ARTIFACT_ROOT}/security/evidence.json"
