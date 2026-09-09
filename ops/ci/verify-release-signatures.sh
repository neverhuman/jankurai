#!/usr/bin/env bash
# Verify collected assets without executing payloads or using API credentials.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_TAG:?release tag required}"
: "${GITHUB_REPOSITORY:?repository required}"
: "${GITHUB_SHA:?source commit required}"
identity="https://github.com/$GITHUB_REPOSITORY/.github/workflows/release.yml@refs/tags/$RELEASE_TAG"
config="$(mktemp -d)"
trap 'rm -rf "$config"' EXIT
for asset in dist/*; do
  case "$asset" in *.sha256|*.sigstore.bundle|*.attestation.jsonl) continue ;; esac
  (cd dist && shasum -a 256 -c "${asset#dist/}.sha256")
  cosign verify-blob "$asset" --bundle "$asset.sigstore.bundle" \
    --certificate-identity "$identity" \
    --certificate-oidc-issuer https://token.actions.githubusercontent.com
  env -u GH_TOKEN -u GITHUB_TOKEN -u GH_ENTERPRISE_TOKEN -u GITHUB_ENTERPRISE_TOKEN \
    GH_CONFIG_DIR="$config" gh attestation verify "$asset" \
    --bundle "$asset.attestation.jsonl" --repo "$GITHUB_REPOSITORY" \
    --cert-identity "$identity" --cert-oidc-issuer https://token.actions.githubusercontent.com \
    --deny-self-hosted-runners --signer-workflow "$GITHUB_REPOSITORY/.github/workflows/release.yml" \
    --signer-digest "$GITHUB_SHA" --source-digest "$GITHUB_SHA" --source-ref "refs/tags/$RELEASE_TAG"
done
