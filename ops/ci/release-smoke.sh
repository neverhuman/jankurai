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
for suffix in '' .sha256 .sigstore.bundle; do
  curl --proto '=https' --tlsv1.2 -fsSL \
    "https://github.com/$repo/releases/download/$RELEASE_TAG/$name$suffix" \
    -o "$smoke_root/$name$suffix"
done
(cd "$smoke_root" && shasum -a 256 -c "$name.sha256")
gh attestation verify "$smoke_root/$name" --repo "$repo" \
  --cert-identity "$identity" --deny-self-hosted-runners
cosign verify-blob "$smoke_root/$name" --bundle "$smoke_root/$name.sigstore.bundle" \
  --certificate-identity "$identity" --certificate-oidc-issuer https://token.actions.githubusercontent.com
npm install --prefix "$smoke_root/npm" "$smoke_root/$name" playwright@1.59.0
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
if gh attestation verify "$smoke_root/tampered.tgz" --repo "$repo" \
  --cert-identity "$identity" --deny-self-hosted-runners; then
  echo 'tampered package attestation unexpectedly verified' >&2
  exit 1
fi
