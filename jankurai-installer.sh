#!/usr/bin/env bash
# Installer-first release entrypoint.
# Downloads the immutable release artifact for the current OS/arch, verifies
# the GitHub release, GitHub attestation, sha256 checksum, and Sigstore bundle,
# then installs the binary.
# Keep structured JSON diagnostics and telemetry in receipts so the next
# agent can rerun the release proof without guessing at hidden state.

set -euo pipefail

fail() {
  printf '\n!! %s\n' "$1" >&2
  exit 1
}

step() {
  printf '\n==> %s\n' "$1"
}

note() {
  printf '... %s\n' "$1"
}

ensure_dir() {
  mkdir -p "$1"
}

usage() {
  cat <<'EOF'
usage: jankurai-installer.sh [--repo owner/name] [--tag vX.Y.Z] [--install-dir path]

Environment variables:
  JANKURAI_RELEASE_REPO   release repository, default: neverhuman/jankurai
  JANKURAI_RELEASE_TAG    release tag, required if --tag is omitted
  JANKURAI_INSTALL_DIR    Linux install prefix, default: ~/.local/bin
EOF
}

repo="${JANKURAI_RELEASE_REPO:-neverhuman/jankurai}"
tag="${JANKURAI_RELEASE_TAG:-${RELEASE_TAG:-}}"
install_dir="${JANKURAI_INSTALL_DIR:-${HOME}/.local/bin}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) repo="${2:?missing repo value}"; shift 2 ;;
    --tag) tag="${2:?missing tag value}"; shift 2 ;;
    --install-dir) install_dir="${2:?missing install-dir value}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fail "unknown argument: $1" ;;
  esac
done

[[ -n "${tag}" ]] || fail "set JANKURAI_RELEASE_TAG=vX.Y.Z or pass --tag vX.Y.Z"

case "$(uname -s)" in
  Darwin) os="darwin" ;;
  Linux) os="linux" ;;
  *) fail "unsupported operating system: $(uname -s)" ;;
esac

case "$(uname -m)" in
  x86_64|amd64) arch="x86_64" ;;
  arm64|aarch64) arch="aarch64" ;;
  *) fail "unsupported architecture: $(uname -m)" ;;
esac

release_url="https://github.com/${repo}/releases/download/${tag}"
version="${tag#v}"
case "${os}" in
  darwin)
    artifact_name="jankurai-${version}-${arch}-apple-darwin.pkg"
    ;;
  linux)
    artifact_name="jankurai-${version}-${arch}-unknown-linux-gnu.tar.gz"
    ;;
esac

artifact_url="${release_url}/${artifact_name}"
checksum_url="${artifact_url}.sha256"
bundle_url="${artifact_url}.sigstore.bundle"
workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT

download() {
  local url="$1"
  local out="$2"
  curl -fsSLo "${out}" "${url}"
}

require_tool() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 is required for release verification; use cargo install --path crates/jankurai --locked if you only need a local source install"
}

require_tool curl
require_tool gh
require_tool cosign

step "Verify immutable release"
gh release verify "${tag}" -R "${repo}"

step "Download release assets"
download "${artifact_url}" "${workdir}/${artifact_name}"
download "${checksum_url}" "${workdir}/${artifact_name}.sha256"
download "${bundle_url}" "${workdir}/${artifact_name}.sigstore.bundle"

step "Verify GitHub artifact attestation"
gh attestation verify "${workdir}/${artifact_name}" -R "${repo}"

step "Verify sha256"
if command -v shasum >/dev/null 2>&1; then
  (
    cd "${workdir}"
    shasum -a 256 -c "${artifact_name}.sha256"
  )
else
  (
    cd "${workdir}"
    sha256sum -c "${artifact_name}.sha256"
  )
fi

step "Verify Sigstore bundle"
cosign verify-blob "${workdir}/${artifact_name}" \
  --bundle "${workdir}/${artifact_name}.sigstore.bundle" \
  --certificate-identity "https://github.com/${repo}/.github/workflows/release.yml@refs/tags/${tag}" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com"

case "${os}" in
  darwin)
    step "Install notarized pkg"
    sudo installer -pkg "${workdir}/${artifact_name}" -target /
    note "Installed to the system location provided by the pkg payload"
    ;;
  linux)
    step "Install tarball payload"
    payload_dir="${workdir}/payload"
    ensure_dir "${payload_dir}"
    tar -xzf "${workdir}/${artifact_name}" -C "${payload_dir}"

    binary="$(find "${payload_dir}" -type f -name jankurai -perm -u+x | head -n 1)"
    [[ -n "${binary}" ]] || fail "release archive did not contain an executable jankurai binary"

    ensure_dir "${install_dir}"
    install -m 0755 "${binary}" "${install_dir}/jankurai"
    note "Installed to ${install_dir}/jankurai"
    ;;
esac
