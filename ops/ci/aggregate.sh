#!/usr/bin/env bash
# Require exact parent results and, in hosted CI, the actual expanded jobs.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
node scripts/ci-aggregate.mjs "$@"
