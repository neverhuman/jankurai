#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
source scripts/node-bootstrap.sh
: "${RELEASE_TAG:?release tag required}"
: "${GH_TOKEN:?GitHub publication token required}"
[[ "$RELEASE_TAG" == "v$(cat VERSION)" ]]
node ops/ci/verify-release-assets.mjs dist
# Verify checksum inventory and Sigstore bundles before release publication.
# GitHub build attestations bind these same assets to the release workflow.
# A draft keeps partially uploaded assets out of the public installation path.
gh release create "$RELEASE_TAG" dist/* --repo "$GITHUB_REPOSITORY" \
  --verify-tag --draft --title "Jankurai $RELEASE_TAG" --notes-file docs/release-notes.md
gh release edit "$RELEASE_TAG" --repo "$GITHUB_REPOSITORY" --draft=false
