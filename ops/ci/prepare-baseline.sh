#!/usr/bin/env bash
# Recompute accepted-source evidence with the candidate's locked auditor.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
repo_root="$PWD"
mkdir -p target/jankurai
baseline_root="$(mktemp -d "$repo_root/target/ci-baseline.XXXXXX")"
trap 'rm -rf "$baseline_root"' EXIT
base="$(git rev-parse refs/remotes/origin/main)"
git clone --no-hardlinks --no-checkout "$repo_root" "$baseline_root/repo"
git -C "$baseline_root/repo" checkout --detach "$base"
jankurai audit "$baseline_root/repo" --mode advisory --no-score-history \
  --json "$repo_root/target/jankurai/accepted-baseline.json" \
  --md "$repo_root/target/jankurai/accepted-baseline.md"
