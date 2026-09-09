#!/usr/bin/env bash
set -euo pipefail
hub="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ ! -f "$hub/node_modules/@iarna/toml/package.json" ]]; then
  (cd "$hub" && npm ci --ignore-scripts)
fi
