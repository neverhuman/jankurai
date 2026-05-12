#!/usr/bin/env bash
# Release per-target build: produces a signed tarball + sha256.
# Inputs: RELEASE_TAG (vX.Y.Z), TARGET (rust target triple). Locally,
# defaults to host target if TARGET is unset.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

if [[ -z "${RELEASE_TAG:-}" ]]; then
  fail "RELEASE_TAG must be set, e.g. RELEASE_TAG=v1.1.0"
fi

target="${TARGET:-$(rustc -vV | awk '/^host:/ {print $2}')}"
version="${RELEASE_TAG#v}"
stage="jankurai-${version}-${target}"
dist="${CI_ROOT}/dist"

ensure_dir "${dist}/${stage}"

step "Install target toolchain"
rustup target add "${target}" >/dev/null 2>&1 || true

step "cargo build --release"
cargo build --release --locked -p jankurai --target "${target}"

step "Stage release artifact"
cp "${CI_ROOT}/target/${target}/release/jankurai" "${dist}/${stage}/"
cp "${CI_ROOT}/LICENSE" "${CI_ROOT}/README.md" "${CI_ROOT}/CHANGELOG.md" "${dist}/${stage}/"

step "Tar + sha256"
( cd "${dist}" && tar -czf "${stage}.tar.gz" "${stage}" )
( cd "${dist}" && shasum -a 256 "${stage}.tar.gz" > "${stage}.tar.gz.sha256" )

assert_nonempty "${dist}/${stage}.tar.gz"
assert_nonempty "${dist}/${stage}.tar.gz.sha256"
