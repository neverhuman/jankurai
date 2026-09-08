#!/usr/bin/env bash
# Verify signed staged assets before publication, then execute native products.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_TAG:?release tag required}"
: "${GITHUB_REPOSITORY:?repository required}"
: "${GITHUB_SHA:?source commit required}"
identity="https://github.com/$GITHUB_REPOSITORY/.github/workflows/release.yml@refs/tags/$RELEASE_TAG"
for asset in dist/*; do
  case "$asset" in *.sha256|*.sigstore.bundle|*.attestation.jsonl) continue ;; esac
  (cd dist && shasum -a 256 -c "${asset#dist/}.sha256")
  cosign verify-blob "$asset" --bundle "$asset.sigstore.bundle" \
    --certificate-identity "$identity" \
    --certificate-oidc-issuer https://token.actions.githubusercontent.com
  env -u GH_TOKEN -u GITHUB_TOKEN gh attestation verify "$asset" \
    --bundle "$asset.attestation.jsonl" --repo "$GITHUB_REPOSITORY" \
    --cert-identity "$identity" --deny-self-hosted-runners \
    --signer-workflow "$GITHUB_REPOSITORY/.github/workflows/release.yml" \
    --signer-digest "$GITHUB_SHA" --source-digest "$GITHUB_SHA" \
    --source-ref "refs/tags/$RELEASE_TAG"
done
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
for product in jankurai tuiwright; do
  env -u GH_TOKEN -u GITHUB_TOKEN bash jankurai-installer.sh \
    --tag "$RELEASE_TAG" --product "$product" --assets-dir "$PWD/dist" \
    --install-dir "$stage/bin"
done
"$stage/bin/jankurai" audit . --mode advisory --full --no-score-history \
  --json "$stage/audit.json" --md "$stage/audit.md"
if [[ -f "dist/jankurai-ux-qa-${RELEASE_TAG#v}.tgz" ]]; then
  npm install --prefix "$stage/ux" "$PWD/dist/jankurai-ux-qa-${RELEASE_TAG#v}.tgz" playwright@1.59.1
  [[ "$("$stage/ux/node_modules/.bin/jankurai-ux-qa" --version)" == "jankurai-ux-qa ${RELEASE_TAG#v}" ]]
fi
