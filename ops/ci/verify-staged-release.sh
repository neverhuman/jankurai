#!/usr/bin/env bash
# Verify signed staged assets before publication, then execute native products.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_TAG:?release tag required}"
bash ops/ci/verify-release-signatures.sh
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
