#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
: "${RELEASE_TAG:?release tag required}"
: "${GH_TOKEN:?GitHub publication token required}"
[[ "$RELEASE_TAG" == "v$(cat VERSION)" ]]
python3 ops/ci/verify-release-assets.py dist
# A draft keeps partially uploaded assets out of the public installation path.
gh release create "$RELEASE_TAG" dist/* --repo "$GITHUB_REPOSITORY" \
  --verify-tag --draft --title "Jankurai $RELEASE_TAG" --notes-file docs/release-notes.md
gh release edit "$RELEASE_TAG" --repo "$GITHUB_REPOSITORY" --draft=false
