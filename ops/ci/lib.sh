# Shared helpers for ops/ci/*.sh. Sourced by every job script so the
# workflow and the local runner take identical paths.
# shellcheck shell=bash

set -euo pipefail

# Pinned tool versions. The CI workflow installs from these; local runners
# match by reading the same file via `ops/ci/lib.sh`.
GITLEAKS_VERSION="${GITLEAKS_VERSION:-8.30.0}"
CARGO_AUDIT_VERSION="${CARGO_AUDIT_VERSION:-0.22.1}"
ZIZMOR_VERSION="${ZIZMOR_VERSION:-1.12.0}"
CARGO_LLVM_COV_VERSION="${CARGO_LLVM_COV_VERSION:-0.6.16}"
CARGO_MUTANTS_VERSION="${CARGO_MUTANTS_VERSION:-27.0.0}"
NODE_VERSION="${NODE_VERSION:-22}"
PLAYWRIGHT_BROWSER="${PLAYWRIGHT_BROWSER:-chromium}"

CI_ROOT="${CI_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
ARTIFACT_ROOT="${ARTIFACT_ROOT:-${CI_ROOT}/target/jankurai}"

step() { printf '\n\033[1;36m==> %s\033[0m\n' "$1"; }
note() { printf '\033[0;90m... %s\033[0m\n' "$1"; }
fail() { printf '\n\033[1;31m!! %s\033[0m\n' "$1" >&2; exit 1; }

assert_path() {
  local p="$1"
  if [[ ! -e "$p" ]]; then
    fail "expected artifact not produced: $p"
  fi
  note "artifact present: $p"
}

assert_nonempty() {
  local p="$1"
  assert_path "$p"
  if [[ ! -s "$p" ]]; then
    fail "expected artifact is empty: $p"
  fi
}

ensure_dir() {
  mkdir -p "$1"
}

# Print the contents of VERSION (trimmed). The release workflow uses this
# to assert the pushed tag matches the canonical version.
read_version() {
  tr -d '[:space:]' < "${CI_ROOT}/VERSION"
}

# Install libfuse on Linux so the jankurai-guard `fuse` feature compiles.
# macOS needs nothing here: the `fuser` dependency is gated to
# cfg(target_os = "linux"), so `--all-features` on macOS never links macFUSE.
# Non-Linux hosts and Linux hosts that already have libfuse are a no-op.
ensure_fuse_dev() {
  if [[ "$(uname -s)" != "Linux" ]]; then
    return 0
  fi
  if pkg-config --exists fuse3 2>/dev/null || pkg-config --exists fuse 2>/dev/null; then
    note "libfuse already present"
    return 0
  fi
  step "Install libfuse3-dev"
  sudo apt-get update -qq
  sudo apt-get install -y -qq libfuse3-dev pkg-config
}
