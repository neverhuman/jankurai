#!/usr/bin/env bash
# Verify and execute downloaded release products on each supported platform.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_TAG:?release tag required}"
repo="${GITHUB_REPOSITORY:?repository required}"
smoke_root="$(mktemp -d)"
trap 'rm -rf "$smoke_root"' EXIT
identity="https://github.com/$repo/.github/workflows/release.yml@refs/tags/$RELEASE_TAG"
for product in jankurai tuiwright; do
  bash jankurai-installer.sh --repo "$repo" --tag "$RELEASE_TAG" \
    --product "$product" --install-dir "$smoke_root/bin"
done
name="jankurai-ux-qa-${RELEASE_TAG#v}.tgz"
for suffix in '' .sha256 .sigstore.bundle .attestation.jsonl; do
  curl --proto '=https' --tlsv1.2 -fsSL \
    "https://github.com/$repo/releases/download/$RELEASE_TAG/$name$suffix" \
    -o "$smoke_root/$name$suffix"
done
(cd "$smoke_root" && shasum -a 256 -c "$name.sha256")
env -u GH_TOKEN -u GITHUB_TOKEN gh attestation verify "$smoke_root/$name" --repo "$repo" \
  --bundle "$smoke_root/$name.attestation.jsonl" \
  --cert-identity "$identity" --deny-self-hosted-runners \
  --signer-digest "$GITHUB_SHA" --source-digest "$GITHUB_SHA" --source-ref "refs/tags/$RELEASE_TAG"
cosign verify-blob "$smoke_root/$name" --bundle "$smoke_root/$name.sigstore.bundle" \
  --certificate-identity "$identity" --certificate-oidc-issuer https://token.actions.githubusercontent.com
npm install --prefix "$smoke_root/npm" "$smoke_root/$name" playwright@1.59.1
[[ "$("$smoke_root/npm/node_modules/.bin/jankurai-ux-qa" --version)" == "jankurai-ux-qa ${RELEASE_TAG#v}" ]]
(cd "$smoke_root/npm" && npm exec -- playwright install --with-deps chromium --only-shell)
"$smoke_root/npm/node_modules/.bin/jankurai-ux-qa" audit \
  --url 'data:text/html,<html lang="en"><title>Release smoke</title><main><h1>Jankurai release</h1></main></html>' \
  --out "$smoke_root/ux-report.json"
jq -e '.reports | length == 2' "$smoke_root/ux-report.json"
cp "$smoke_root/$name" "$smoke_root/tampered.tgz"
printf 'tampered' >> "$smoke_root/tampered.tgz"
if cosign verify-blob "$smoke_root/tampered.tgz" --bundle "$smoke_root/$name.sigstore.bundle" \
  --certificate-identity "$identity" --certificate-oidc-issuer https://token.actions.githubusercontent.com; then
  echo 'tampered package unexpectedly verified' >&2
  exit 1
fi
if env -u GH_TOKEN -u GITHUB_TOKEN gh attestation verify "$smoke_root/tampered.tgz" --repo "$repo" \
  --bundle "$smoke_root/$name.attestation.jsonl" \
  --cert-identity "$identity" --deny-self-hosted-runners \
  --signer-digest "$GITHUB_SHA" --source-digest "$GITHUB_SHA" --source-ref "refs/tags/$RELEASE_TAG"; then
  echo 'tampered package attestation unexpectedly verified' >&2
  exit 1
fi
