#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/node-bootstrap.sh"
exec node "$hub/scripts/project-family-manifest.mjs" "$@"
