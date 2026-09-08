#!/usr/bin/env bash
# Export the action's signed bundle alongside each subject for anonymous installs.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${ATTESTATION_BUNDLE:?attestation action bundle required}"
test -s "$ATTESTATION_BUNDLE"
for asset in dist/*; do
  case "$asset" in *.sha256|*.sigstore.bundle|*.attestation.jsonl) continue ;; esac
  cp "$ATTESTATION_BUNDLE" "$asset.attestation.jsonl"
done
