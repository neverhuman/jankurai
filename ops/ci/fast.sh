#!/usr/bin/env bash
# Deterministic fast lane: the narrowest proof loop for agent iteration.
# Validates required split metadata, Jeryu mirror config, lockfile pins, branch
# dependencies, committed cross-repo path dependencies, and action pinning.
# The identical command is exposed locally via `just fast` and
# `bash scripts/ci-local.sh fast`.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT"

log "fast lane: bash scripts/validate-family.sh"
bash scripts/validate-family.sh
