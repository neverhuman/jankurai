#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../../scripts/node-bootstrap.sh"
exec node "$hub/scripts/publish-family-update.mjs" "$@"
