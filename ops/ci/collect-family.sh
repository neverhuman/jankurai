#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
bash scripts/family.sh pull
if git diff --quiet -- family.lock Cargo.lock; then
  echo 'changed=false' >> "$GITHUB_OUTPUT"
else
  mkdir -p target/candidate
  cp family.lock Cargo.lock target/candidate/
  echo 'changed=true' >> "$GITHUB_OUTPUT"
fi
