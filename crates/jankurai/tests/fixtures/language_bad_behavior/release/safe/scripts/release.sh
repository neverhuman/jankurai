#!/usr/bin/env bash
set -euo pipefail

version="${1:?version}"
tag="v${version}"

cargo test --workspace --locked
cargo build --release --locked
git tag -s "${tag}" -m "Release ${tag}"
git tag -v "${tag}"
git push origin "${tag}"

mkdir -p target/release-artifacts
cp target/release/jankurai target/release-artifacts/jankurai
sha256sum target/release-artifacts/jankurai > target/release-artifacts/jankurai.sha256
syft packages dir:. -o spdx-json > target/release-artifacts/sbom.spdx.json
cosign attest --predicate target/release-artifacts/sbom.spdx.json target/release-artifacts/jankurai
gh release create "${tag}" --verify-tag --notes-file CHANGELOG.md \
  target/release-artifacts/jankurai \
  target/release-artifacts/jankurai.sha256 \
  target/release-artifacts/sbom.spdx.json
