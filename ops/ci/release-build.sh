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
bash scripts/family.sh build --release --target "$TARGET"
python3 ops/ci/package-release.py "$TARGET"
