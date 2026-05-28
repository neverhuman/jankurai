#!/usr/bin/env bash
# Install a pinned Node.js toolchain when the runner does not already provide
# the requested major version. GitLab jobs use this to mirror GitHub's
# setup-node step without relying on runner image state.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

node_major="${NODE_VERSION%%.*}"

if command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
  have_major="$(node -p 'process.versions.node.split(".")[0]' 2>/dev/null || true)"
  if [[ "${have_major}" == "${node_major}" ]]; then
    note "node ${have_major} already installed"
    exit 0
  fi
fi

step "Install Node.js ${NODE_VERSION}"
case "$(uname -s)" in
  Linux)
    apt_cmd="apt-get"
    command -v sudo >/dev/null 2>&1 && apt_cmd="sudo apt-get"
    $apt_cmd update -qq
    $apt_cmd install -y -qq ca-certificates curl gnupg
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    curl -fsSL "https://deb.nodesource.com/setup_${node_major}.x" -o "${tmp}/nodesource.sh"
    bash "${tmp}/nodesource.sh"
    $apt_cmd install -y -qq nodejs
    ;;
  Darwin)
    if command -v brew >/dev/null 2>&1; then
      if ! brew list "node@${node_major}" >/dev/null 2>&1; then
        brew install "node@${node_major}" || brew install node
      fi
      export PATH="/opt/homebrew/opt/node@${node_major}/bin:/usr/local/opt/node@${node_major}/bin:${PATH}"
    else
      fail "node ${NODE_VERSION} is required; install it before running the security lane"
    fi
    ;;
  *)
    fail "unsupported host for node bootstrap: $(uname -s)"
    ;;
esac

if ! command -v node >/dev/null 2>&1 || ! command -v npm >/dev/null 2>&1; then
  fail "node ${NODE_VERSION} bootstrap did not make node/npm available"
fi
