#!/usr/bin/env bash
# Install the pinned security toolchain. Idempotent: skips tools that are
# already on PATH at the required version.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

step "Node.js toolchain ${NODE_VERSION}"
bash "$(dirname "${BASH_SOURCE[0]}")/node-tools.sh"

want_version() {
  local cmd="$1" want="$2"
  if ! command -v "$cmd" >/dev/null 2>&1; then return 1; fi
  local have
  have="$("$cmd" --version 2>&1 | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || true)"
  [[ "$have" == "$want" ]]
}

step "cargo-audit ${CARGO_AUDIT_VERSION}"
if ! want_version "cargo-audit" "$CARGO_AUDIT_VERSION"; then
  cargo install cargo-audit --version "$CARGO_AUDIT_VERSION" --locked
fi

step "zizmor ${ZIZMOR_VERSION}"
if ! want_version "zizmor" "$ZIZMOR_VERSION"; then
  cargo install zizmor --version "$ZIZMOR_VERSION" --locked
fi

step "gitleaks ${GITLEAKS_VERSION}"
if ! want_version "gitleaks" "$GITLEAKS_VERSION"; then
  uname_s="$(uname -s | tr '[:upper:]' '[:lower:]')"
  uname_m="$(uname -m)"
  case "$uname_s/$uname_m" in
    linux/x86_64)  asset="gitleaks_${GITLEAKS_VERSION}_linux_x64.tar.gz" ;;
    linux/aarch64) asset="gitleaks_${GITLEAKS_VERSION}_linux_arm64.tar.gz" ;;
    darwin/arm64)  asset="gitleaks_${GITLEAKS_VERSION}_darwin_arm64.tar.gz" ;;
    darwin/x86_64) asset="gitleaks_${GITLEAKS_VERSION}_darwin_x64.tar.gz" ;;
    *) fail "unsupported os/arch $uname_s/$uname_m for gitleaks install" ;;
  esac
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  ( cd "$tmp"
    curl -fsSLO "https://github.com/gitleaks/gitleaks/releases/download/v${GITLEAKS_VERSION}/${asset}"
    curl -fsSLO "https://github.com/gitleaks/gitleaks/releases/download/v${GITLEAKS_VERSION}/gitleaks_${GITLEAKS_VERSION}_checksums.txt"
    grep " ${asset}\$" "gitleaks_${GITLEAKS_VERSION}_checksums.txt" | sha256sum -c -
    tar -xzf "${asset}" gitleaks
    local_bin="${HOME}/.local/bin"
    mkdir -p "$local_bin"
    if install -m 0755 gitleaks "${local_bin}/gitleaks" 2>/dev/null; then
      export PATH="${local_bin}:${PATH}"
    elif command -v sudo >/dev/null 2>&1; then
      sudo install -m 0755 gitleaks /usr/local/bin/gitleaks
    else
      fail "could not install gitleaks without sudo or a writable ${local_bin}"
    fi
  )
fi
