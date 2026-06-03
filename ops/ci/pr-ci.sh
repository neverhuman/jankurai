#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

run() {
  printf '[pr-ci] %s\n' "$*" >&2
  "$@"
}

just_has() {
  command -v just >/dev/null 2>&1 || return 1
  [[ -f justfile || -f Justfile || -f .justfile ]] || return 1
  just --summary 2>/dev/null | tr ' ' '\n' | grep -qx "$1"
}

if just_has fast; then
  run just fast
elif just_has check; then
  run just check
elif just_has test; then
  run just test
elif [[ -f Cargo.toml ]]; then
  if cargo nextest --version >/dev/null 2>&1; then
    run cargo nextest run --workspace --no-fail-fast
  else
    run cargo test --workspace --no-fail-fast
  fi
elif [[ -f package.json ]]; then
  if [[ -f package-lock.json ]]; then
    run npm ci --no-audit --no-fund
  fi
  run npm test
else
  printf '[pr-ci] no supported CI entrypoint found\n' >&2
  exit 91
fi
