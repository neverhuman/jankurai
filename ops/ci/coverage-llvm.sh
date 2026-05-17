#!/usr/bin/env bash
# Coverage lane: produce llvm-cov and cargo-mutants evidence artifacts.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

ensure_fuse_dev

out_dir="${CI_ROOT}/target/coverage"
ensure_dir "$out_dir"
ensure_dir "${CI_ROOT}/target/llvm-cov"
ensure_dir "${ARTIFACT_ROOT}/coverage"
ensure_dir "${CI_ROOT}/target/mutants"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  fail "cargo-llvm-cov not installed (cargo install cargo-llvm-cov --locked --version ${CARGO_LLVM_COV_VERSION})"
fi

if ! command -v cargo-mutants >/dev/null 2>&1; then
  fail "cargo-mutants not installed (cargo install cargo-mutants --locked --version ${CARGO_MUTANTS_VERSION})"
fi

ensure_llvm_tool_env() {
  local host
  host="$(rustc -vV | awk '/^host:/ { print $2 }')"
  local sysroot
  sysroot="$(rustc --print sysroot)"
  local candidate
  while IFS= read -r candidate; do
    if [[ -x "${candidate}/llvm-cov" && -x "${candidate}/llvm-profdata" ]]; then
      export LLVM_COV="${LLVM_COV:-${candidate}/llvm-cov}"
      export LLVM_PROFDATA="${LLVM_PROFDATA:-${candidate}/llvm-profdata}"
      note "llvm tools: ${candidate}"
      return 0
    fi
  done < <(
    printf '%s\n' "${sysroot}/lib/rustlib/${host}/bin"
    find "${HOME}/.rustup/toolchains" -path "*/lib/rustlib/${host}/bin" -type d 2>/dev/null || true
  )
}

mutation_diff="${ARTIFACT_ROOT}/coverage/mutation.diff"
mutation_list="${ARTIFACT_ROOT}/coverage/mutants-list.json"
mutation_outcomes="${CI_ROOT}/target/mutants/mutants.out/outcomes.json"

write_empty_mutation_outcomes() {
  ensure_dir "$(dirname "$mutation_outcomes")"
  printf '{\n  "outcomes": []\n}\n' > "$mutation_outcomes"
}

write_mutation_diff() {
  : > "$mutation_diff"
  if ! git -C "$CI_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    note "not a git worktree; mutation evidence records an empty diff"
    return 0
  fi

  if [[ -n "${GITHUB_BASE_REF:-}" ]]; then
    git -C "$CI_ROOT" fetch --no-tags --depth=1 origin "$GITHUB_BASE_REF" >/dev/null 2>&1 || true
    if git -C "$CI_ROOT" rev-parse --verify "origin/${GITHUB_BASE_REF}" >/dev/null 2>&1; then
      if git -C "$CI_ROOT" diff --no-ext-diff --binary "origin/${GITHUB_BASE_REF}...HEAD" > "$mutation_diff"; then
        return 0
      fi
      git -C "$CI_ROOT" diff --no-ext-diff --binary HEAD > "$mutation_diff"
      return 0
    fi
  fi

  if [[ -n "${GITHUB_SHA:-}" ]] && git -C "$CI_ROOT" rev-parse --verify HEAD^ >/dev/null 2>&1; then
    git -C "$CI_ROOT" diff --no-ext-diff --binary HEAD^ HEAD > "$mutation_diff"
    return 0
  fi

  git -C "$CI_ROOT" diff --no-ext-diff --binary HEAD > "$mutation_diff"
}

step "cargo llvm-cov --workspace"
ensure_llvm_tool_env
cargo llvm-cov --workspace --all-features --locked \
  --lcov --output-path "$out_dir/lcov.info"
cargo llvm-cov report --json --output-path "$out_dir/coverage.json"
cargo llvm-cov report --summary-only | tee "$out_dir/summary.txt"
cp "$out_dir/lcov.info" "${CI_ROOT}/target/llvm-cov/lcov.info"
cp "$out_dir/lcov.info" "${ARTIFACT_ROOT}/coverage/rust-lcov.info"

assert_nonempty "$out_dir/lcov.info"
assert_nonempty "$out_dir/coverage.json"
assert_nonempty "$out_dir/summary.txt"
assert_nonempty "${CI_ROOT}/target/llvm-cov/lcov.info"
assert_nonempty "${ARTIFACT_ROOT}/coverage/rust-lcov.info"

step "cargo mutants --in-diff"
write_mutation_diff
if [[ ! -s "$mutation_diff" ]]; then
  note "no git diff found; writing empty mutation outcomes"
  write_empty_mutation_outcomes
else
  cargo mutants --list --json --in-diff "$mutation_diff" --output "${CI_ROOT}/target/mutants" \
    --workspace --all-features > "$mutation_list"
  mutant_count=$(
    python3 - "$mutation_list" <<'PY'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as fh:
    data = json.load(fh)
print(len(data) if isinstance(data, list) else 0)
PY
  )
  if [[ "$mutant_count" -eq 0 ]]; then
    note "diff produced no Rust mutants; writing empty mutation outcomes"
    write_empty_mutation_outcomes
  else
    note "testing ${mutant_count} changed-code mutants"
    cargo mutants --in-diff "$mutation_diff" --output "${CI_ROOT}/target/mutants" \
      --workspace --all-features --baseline skip \
      --jobs "${CARGO_MUTANTS_JOBS:-4}" --no-times \
      --timeout "${CARGO_MUTANTS_TIMEOUT:-120}" -- --test-threads=1
  fi
fi
assert_nonempty "$mutation_outcomes"
