#!/usr/bin/env bash
# Verify every tool CI depends on is installed locally so `just ci` can
# reproduce CI failures without pushing.
set -euo pipefail

bold() { printf '\033[1m%s\033[0m' "$1"; }
green() { printf '\033[32m%s\033[0m' "$1"; }
red() { printf '\033[31m%s\033[0m' "$1"; }
yellow() { printf '\033[33m%s\033[0m' "$1"; }

missing=()
warnings=()
ok=()

check_cmd() {
  local name="$1"
  local install_hint="$2"
  if command -v "$name" >/dev/null 2>&1; then
    local version
    version=$("$name" --version 2>&1 | head -1 || true)
    ok+=("$name — ${version:-installed}")
  else
    missing+=("$name — install: $install_hint")
  fi
}

check_cargo_subcommand() {
  local sub="$1"
  local hint="$2"
  if cargo "$sub" --version >/dev/null 2>&1; then
    ok+=("cargo-$sub")
  else
    missing+=("cargo-$sub — install: $hint")
  fi
}

check_npm_package() {
  local pkg="$1"
  if [[ -d "node_modules/$pkg" ]]; then
    ok+=("npm:$pkg")
  else
    warnings+=("npm:$pkg not installed (run \`npm ci\`)")
  fi
}

# Core toolchain
check_cmd cargo "https://rustup.rs"
check_cmd rustc "https://rustup.rs"
check_cmd npm "https://nodejs.org or fnm"
check_cmd node "https://nodejs.org or fnm"
check_cmd just "brew install just"
check_cmd gh "brew install gh"
check_cmd jq "brew install jq"
check_cmd rg "brew install ripgrep"
check_cmd awk "system"
check_cmd python3 "brew install python"

# CI quality gates rely on these
check_cargo_subcommand fmt "rustup component add rustfmt"
check_cargo_subcommand clippy "rustup component add clippy"
check_cargo_subcommand llvm-cov "cargo install cargo-llvm-cov --locked"

# Security lane
check_cmd gitleaks "brew install gitleaks"
check_cargo_subcommand audit "cargo install cargo-audit --locked"
check_cmd zizmor "cargo install zizmor --locked"
check_cmd syft "brew install syft"

# Paper lane
check_cmd latexmk "brew install --cask mactex (or texlive on linux)"

# Playwright (npm-managed; only verify dir if node_modules exists)
if [[ -d node_modules ]]; then
  if [[ ! -d node_modules/@playwright/test ]]; then
    warnings+=("npm:@playwright/test not installed (run \`npm ci\`)")
  fi
  if ! npx --no playwright --version >/dev/null 2>&1; then
    warnings+=("Playwright browsers not installed (run \`npx playwright install chromium\`)")
  fi
else
  warnings+=("node_modules absent (run \`npm ci\`)")
fi

# jankurai binary
if command -v jankurai >/dev/null 2>&1; then
  ok+=("jankurai $(jankurai version 2>/dev/null | head -1 || echo installed)")
else
  warnings+=("jankurai not on PATH (run \`cargo install --path crates/jankurai --locked\`)")
fi

bold "CI-local prerequisites"
printf '\n\n'

if [[ ${#ok[@]} -gt 0 ]]; then
  green "OK:"; printf '\n'
  for entry in "${ok[@]}"; do printf '  ✓ %s\n' "$entry"; done
  printf '\n'
fi

if [[ ${#warnings[@]} -gt 0 ]]; then
  yellow "Warnings:"; printf '\n'
  for entry in "${warnings[@]}"; do printf '  ! %s\n' "$entry"; done
  printf '\n'
fi

if [[ ${#missing[@]} -gt 0 ]]; then
  red "Missing:"; printf '\n'
  for entry in "${missing[@]}"; do printf '  ✗ %s\n' "$entry"; done
  printf '\nInstall the missing tools above, then re-run \`just ci-doctor\`.\n'
  exit 1
fi

printf 'All CI prerequisites installed. \`just ci\` will run the full local equivalent.\n'
