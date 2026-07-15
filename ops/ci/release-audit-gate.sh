#!/usr/bin/env bash
# Release audit gate: tag-vs-VERSION check, coverage/mutation evidence,
# quality gates, strict security lane, ratchet audit, and SARIF artifact.
# Mirrors release.yml#audit-gate.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

ensure_dir "${ARTIFACT_ROOT}"
ensure_dir "${ARTIFACT_ROOT}/security"

if [[ -n "${RELEASE_TAG:-}" ]]; then
  step "Verify RELEASE_TAG matches VERSION"
  expected_tag="$(sed -n 's/^release_tag = "\([^"]*\)"/\1/p' "${CI_ROOT}/agent/standard-version.toml")"
  [[ -n "${expected_tag}" ]] || fail "agent/standard-version.toml is missing release_tag"
  [[ "${RELEASE_TAG}" == "${expected_tag}" ]] \
    || fail "RELEASE_TAG (${RELEASE_TAG}) does not match release_tag (${expected_tag})"
  tag="${RELEASE_TAG#v}"
  tag="${tag%%-*}"
  file="$(read_version)"
  if [[ "$tag" != "$file" ]]; then
    fail "RELEASE_TAG (${RELEASE_TAG} -> ${tag}) does not match VERSION (${file})"
  fi
fi

step "Installed-binary relocation"
bash "$(dirname "${BASH_SOURCE[0]}")/relocation-test.sh"

step "Security toolchain"
bash "$(dirname "${BASH_SOURCE[0]}")/security-tools.sh"

step "Quality gates"
bash "$(dirname "${BASH_SOURCE[0]}")/quality-gates.sh"

step "Coverage and mutation evidence"
bash "$(dirname "${BASH_SOURCE[0]}")/coverage-llvm.sh"

step "Install local jankurai"
cargo install --path crates/jankurai --locked --force

step "Security lane (strict, ci profile)"
jankurai security run . --strict --profile ci --out "${ARTIFACT_ROOT}/security/evidence.json"

step "Ratchet audit"
jankurai audit . \
  --full \
  --mode ratchet \
  --baseline "${CI_ROOT}/agent/baselines/main.repo-score.json" \
  --json "${ARTIFACT_ROOT}/repo-score.json" \
  --md "${ARTIFACT_ROOT}/repo-score.md" \
  --sarif "${ARTIFACT_ROOT}/jankurai.sarif"

assert_nonempty "${ARTIFACT_ROOT}/repo-score.json"
assert_nonempty "${ARTIFACT_ROOT}/repo-score.md"
assert_nonempty "${ARTIFACT_ROOT}/jankurai.sarif"
assert_nonempty "${ARTIFACT_ROOT}/security/evidence.json"
