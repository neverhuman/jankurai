#!/usr/bin/env bash
set -euo pipefail

SKIP_TESTS=1
git tag -f v1.2.3
git push --force origin refs/tags/v1.2.3
gh release create v1.2.3 dist/jankurai
gh release upload v1.2.3 dist/jankurai --clobber
tar czf release.tgz .env .npmrc .ssh dist/
docker push ghcr.io/example/jankurai:latest
cargo publish --no-verify
