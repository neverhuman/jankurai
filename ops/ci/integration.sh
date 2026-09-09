#!/usr/bin/env bash
# Combined conformance, proof, and security evidence from the locked auditor.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
hub="$PWD"
export PATH="$hub/.fusion/target/debug:$PATH"
bash scripts/validate-family.sh --checkouts
mkdir -p target/jankurai
jankurai conformance run \
  --fixtures ../jankurai-conformance/conformance/fixtures \
  --expected ../jankurai-conformance/conformance/expected \
  --out target/jankurai/conformance.json \
  --md target/jankurai/conformance.md \
  --tex target/jankurai/conformance.tex
bash ops/ci/security.sh
bash ops/ci/audit.sh
bash ops/ci/tool-adoption.sh
