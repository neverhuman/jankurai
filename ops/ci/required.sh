#!/usr/bin/env bash
# Required lane: the lightweight gate that must pass on every push.
# Runs the family validator that proves split metadata, lock pins, and action
# pinning posture stay coherent. Same command as `just fast`.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT"

log "required lane: bash scripts/validate-family.sh"
bash scripts/validate-family.sh
