#!/usr/bin/env bash
# Quality gates lane: fmt, clippy, workspace tests, README test-surface.
# Used by both jankurai.yml#test-matrix and `just ci-quick`.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

ensure_fuse_dev

step "cargo fmt"
cargo fmt --all -- --check

step "cargo clippy"
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

step "cargo test --workspace"
cargo test --workspace --exclude tuiwright --all-targets --all-features --locked

step "cargo test -p tuiwright --serial"
cargo test -p tuiwright --all-targets --locked -- --test-threads=1

step "README test surface in sync"
bash "${CI_ROOT}/scripts/render-test-surface.sh" --check
