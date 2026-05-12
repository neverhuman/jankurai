#!/usr/bin/env bash
# llvm-cov lane: produce lcov + json coverage artifacts.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

out_dir="${CI_ROOT}/target/coverage"
ensure_dir "$out_dir"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  fail "cargo-llvm-cov not installed (cargo install cargo-llvm-cov --locked --version ${CARGO_LLVM_COV_VERSION})"
fi

step "cargo llvm-cov --workspace"
cargo llvm-cov --workspace --all-features --locked \
  --lcov --output-path "$out_dir/lcov.info"
cargo llvm-cov report --json --output-path "$out_dir/coverage.json"
cargo llvm-cov report --summary-only | tee "$out_dir/summary.txt"

assert_nonempty "$out_dir/lcov.info"
assert_nonempty "$out_dir/coverage.json"
assert_nonempty "$out_dir/summary.txt"
