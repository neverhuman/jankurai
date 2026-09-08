#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
for asset in dist/*; do
  [[ "$asset" != *.sha256 && "$asset" != *.sigstore.bundle && "$asset" != *.attestation.jsonl ]] || continue
  cosign sign-blob --yes --bundle "$asset.sigstore.bundle" "$asset"
done
