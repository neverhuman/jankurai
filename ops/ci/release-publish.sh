#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_TAG:?release tag required}"
: "${GH_TOKEN:?GitHub publication token required}"
[[ "$RELEASE_TAG" == "v$(cat VERSION)" ]]
node ops/ci/verify-release-assets.mjs dist
# Resume partial drafts only when every existing asset matches this candidate.
# Never replace published bytes or move the already-verified source tag.
node ops/ci/publish-release.mjs
