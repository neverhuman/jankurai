#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

lane="${1:-required}"
case "$lane" in
  required) bash ops/ci/required.sh ;;
  *) echo "usage: $0 {required}" >&2; exit 2 ;;
esac
