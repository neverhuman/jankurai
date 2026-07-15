#!/usr/bin/env bash
# Prove the release binary does not read schemas from its build checkout.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

scratch="$(mktemp -d "${TMPDIR:-/tmp}/jankurai-relocation.XXXXXX")"
trap 'rm -rf "${scratch}"' EXIT
source_dir="${scratch}/source"
target_dir="${scratch}/target"
run_dir="${scratch}/run"
fixture="${scratch}/fixture"
mkdir -p "${source_dir}" "${target_dir}" "${run_dir}/bin" "${fixture}"

git -C "${CI_ROOT}" archive HEAD | tar -x -C "${source_dir}"
CARGO_NET_OFFLINE=true cargo build \
  --locked \
  --release \
  --manifest-path "${source_dir}/Cargo.toml" \
  --package jankurai \
  --target-dir "${target_dir}"
install -m 0755 "${target_dir}/release/jankurai" "${run_dir}/bin/jankurai"

rm -rf "${source_dir}" "${target_dir}"
[[ ! -e "${source_dir}/schemas" ]] || fail "relocation source schemas still exist"

git -C "${fixture}" init -q -b main
printf '# relocation fixture\n' >"${fixture}/README.md"
GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null \
  git -C "${fixture}" -c user.name=relocation -c user.email=relocation@example.invalid \
  add README.md
GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null \
  git -C "${fixture}" -c user.name=relocation -c user.email=relocation@example.invalid \
  commit -q -m base
printf '# release notes\n\n- relocated\n' >"${fixture}/CHANGELOG.md"
GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null git -C "${fixture}" add CHANGELOG.md

expected_semver="$(read_version)"
version="$(${run_dir}/bin/jankurai --version)"
[[ "${version}" == "jankurai ${expected_semver}" ]] \
  || fail "relocated binary version mismatch: ${version}"

(cd "${fixture}" && "${run_dir}/bin/jankurai" version) \
  | grep -q "CLI version: \`${expected_semver}\`" \
  || fail "relocated binary could not report installed release identity"

JANKURAI_NO_UPDATE_CHECK=1 \
  "${run_dir}/bin/jankurai" bench "${fixture}" \
    --out "${fixture}/target/jankurai/relocated-benchmark.json"

JANKURAI_NO_UPDATE_CHECK=1 \
  "${run_dir}/bin/jankurai" certify "${fixture}" \
    --out "${fixture}/target/jankurai/relocated-certification.json"

JANKURAI_NO_UPDATE_CHECK=1 \
GIT_CONFIG_GLOBAL=/dev/null \
GIT_CONFIG_SYSTEM=/dev/null \
GIT_TERMINAL_PROMPT=0 \
  "${run_dir}/bin/jankurai" diff-audit "${fixture}" \
    --base-ref main \
    --skip-proof \
    --advisory-only \
    --out-dir "${fixture}/target/jankurai/diff"

JANKURAI_NO_UPDATE_CHECK=1 \
GIT_CONFIG_GLOBAL=/dev/null \
GIT_CONFIG_SYSTEM=/dev/null \
GIT_TERMINAL_PROMPT=0 \
  "${run_dir}/bin/jankurai" gate "${fixture}" --staged-only

assert_nonempty "${fixture}/target/jankurai/diff/diff-score.json"
assert_nonempty "${fixture}/target/jankurai/relocated-benchmark.json"
assert_nonempty "${fixture}/target/jankurai/relocated-certification.json"
jq -e --arg version "${expected_semver}" '.runner_version == $version' \
  "${fixture}/target/jankurai/relocated-benchmark.json" >/dev/null \
  || fail "relocated benchmark did not use embedded release identity"
jq -e --arg version "${expected_semver}" '.auditor_version == $version' \
  "${fixture}/target/jankurai/relocated-certification.json" >/dev/null \
  || fail "relocated certification did not use embedded release identity"
note "relocated schema and release-data commands passed after source removal"
