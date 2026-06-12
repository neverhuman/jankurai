#!/usr/bin/env bash
# Jankurai self-audit lane: writes the repo-score artifacts that CI uploads.
# The same lane runs locally via `just audit`.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT"

mkdir -p .jankurai
log "audit lane: jankurai audit . -> .jankurai/repo-score.{json,md}"
jankurai audit . --no-score-history --json .jankurai/repo-score.json --md .jankurai/repo-score.md

assert_artifact .jankurai/repo-score.json
assert_artifact .jankurai/repo-score.md
