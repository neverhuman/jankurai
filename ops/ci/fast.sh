#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
bash scripts/validate-family.sh
if [ ! -x "${JANKURAI_BIN:-}" ]; then
  bash jankurai-installer.sh
  JANKURAI_BIN="$HOME/.local/bin/jankurai"
fi
export JANKURAI_BIN
export PATH="$(dirname -- "$JANKURAI_BIN"):$PATH"
npm test
node scripts/demo/render-audit-gif.mjs
