#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
if [[ "$(uname -s)" == Linux ]]; then
  sudo apt-get update
  sudo apt-get install -y pkg-config libssl-dev libfuse3-dev latexmk texlive-latex-extra texlive-fonts-recommended
fi
if [[ "${CI_INSTALL_SECURITY:-1}" == 1 && "$(uname -s)" == Linux ]]; then
  GOBIN="${CARGO_HOME:-$HOME/.cargo}/bin" go install github.com/gitleaks/gitleaks/v8@v8.21.2
  GOBIN="${CARGO_HOME:-$HOME/.cargo}/bin" go install github.com/rhysd/actionlint/cmd/actionlint@v1.7.8
  GOBIN="${CARGO_HOME:-$HOME/.cargo}/bin" go install github.com/anchore/grype/cmd/grype@v0.99.0
fi
bash scripts/family.sh setup
if [[ "$(uname -s)" == Linux ]]; then
  (cd ../jankurai-tools-ux && npm exec -- playwright install --with-deps chromium --only-shell)
fi
