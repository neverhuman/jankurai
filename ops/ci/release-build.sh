#!/usr/bin/env bash
# Build only the public CLI products, then package provenance with each tarball.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
version="$(cat VERSION)"
[[ "${RELEASE_TAG:-v$version}" == "v$version" ]] || { echo 'release tag differs from VERSION' >&2; exit 1; }
: "${TARGET:?TARGET is required}"
case "$TARGET" in
  x86_64-unknown-linux-gnu|aarch64-apple-darwin) ;;
  *) echo "unsupported release target: $TARGET" >&2; exit 1 ;;
esac
# Exercise the native exchange and recovery primitives on both supported hosts.
node --test scripts/family-operation-review.test.mjs scripts/family-recovery.test.mjs
bash scripts/family.sh build --release --target "$TARGET"
node ops/ci/package-release.mjs "$TARGET"
