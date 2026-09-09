#!/usr/bin/env bash
# Jankurai self-audit lane: writes the repo-score artifacts that CI uploads.
# The same lane runs locally via `just audit`.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT"

mkdir -p .jankurai
log "audit lane: jankurai audit . -> .jankurai/repo-score.{json,md}" --full
jankurai audit . --no-score-history --json .jankurai/repo-score.json --md .jankurai/repo-score.md --full

assert_artifact .jankurai/repo-score.json
assert_artifact .jankurai/repo-score.md

if [[ -f agent/badge.toml ]]; then
  log "audit lane: jankurai badge --check"
  grep -q 'jankurai-badge:start' README.md
  test -s agent/jankurai-badge.svg
  test -s agent/jankurai-badge.json
  jankurai badge --check \
    --score agent/baselines/main.repo-score.json \
    --out agent/jankurai-badge.svg \
    --json-out agent/jankurai-badge.json \
    --readme README.md \
    --link agent/jankurai-badge.json \
    --update-readme \
    --label jankurai
fi
