#!/usr/bin/env bash
# Quality gates lane: fmt, clippy, workspace tests, README test-surface.
# Used by both jankurai.yml#test-matrix and `just ci-quick`.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

step "cargo fmt"
cargo fmt --all -- --check

step "cargo clippy"
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

step "cargo test --workspace"
cargo test --workspace --all-targets --all-features --locked

step "README test surface in sync"
bash "${CI_ROOT}/scripts/render-test-surface.sh" --check
