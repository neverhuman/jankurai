#!/usr/bin/env bash
# Release per-target build: produces signed Linux tarballs or notarized macOS
# pkg artifacts, plus sha256 and Sigstore bundle sidecars.
# Inputs: RELEASE_TAG (vX.Y.Z), TARGET (rust target triple). Locally,
# defaults to host target if TARGET is unset.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

if [[ -z "${RELEASE_TAG:-}" ]]; then
  fail "RELEASE_TAG must be set, e.g. RELEASE_TAG=v1.5.1"
fi

target="${TARGET:-$(rustc -vV | awk '/^host:/ {print $2}')}"
version="${RELEASE_TAG#v}"
expected_version="$(read_version)"
stage="jankurai-${version}-${target}"
dist="${CI_ROOT}/dist"
binary="${CI_ROOT}/target/${target}/release/jankurai"

if [[ "${version}" != "${expected_version}" ]]; then
  fail "RELEASE_TAG (${RELEASE_TAG}) does not match VERSION (${expected_version})"
fi

ensure_dir "${dist}"

step "Install target toolchain"
rustup target add "${target}" >/dev/null 2>&1 || true

step "cargo build --release"
cargo build --release --locked -p jankurai --target "${target}"

case "${target}" in
  *-apple-darwin)
    step "Build notarized macOS pkg"
    bash "${CI_ROOT}/ops/ci/release-macos-sign.sh" \
      --binary "${binary}" \
      --pkg-out "${dist}/${stage}.pkg" \
      --version "${version}" \
      --target "${target}"

    step "sha256 + Sigstore bundle"
    (
      cd "${dist}"
      shasum -a 256 "${stage}.pkg" > "${stage}.pkg.sha256"
    )
    bash "${CI_ROOT}/ops/ci/release-sign-blob.sh" "${dist}/${stage}.pkg"

    assert_nonempty "${dist}/${stage}.pkg"
    assert_nonempty "${dist}/${stage}.pkg.sha256"
    assert_nonempty "${dist}/${stage}.pkg.sigstore.bundle"
    ;;
  *-linux-gnu)
    step "Stage release artifact"
    ensure_dir "${dist}/${stage}"
    cp "${binary}" "${dist}/${stage}/"
    cp "${CI_ROOT}/LICENSE" "${CI_ROOT}/README.md" "${CI_ROOT}/CHANGELOG.md" "${dist}/${stage}/"

    step "Tar + sha256"
    (
      cd "${dist}" && tar -czf "${stage}.tar.gz" "${stage}"
    )
    (
      cd "${dist}" && shasum -a 256 "${stage}.tar.gz" > "${stage}.tar.gz.sha256"
    )
    bash "${CI_ROOT}/ops/ci/release-sign-blob.sh" "${dist}/${stage}.tar.gz"

    assert_nonempty "${dist}/${stage}.tar.gz"
    assert_nonempty "${dist}/${stage}.tar.gz.sha256"
    assert_nonempty "${dist}/${stage}.tar.gz.sigstore.bundle"
    ;;
  *)
    fail "unsupported release target: ${target}"
    ;;
esac
