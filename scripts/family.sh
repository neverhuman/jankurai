#!/usr/bin/env bash
set -euo pipefail
hub="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${1:-}" == "recover" ]]; then
  exec node "$hub/scripts/family.mjs" "$@"
fi
source "$(dirname "${BASH_SOURCE[0]}")/node-bootstrap.sh"
exec node "$hub/scripts/family.mjs" "$@"
