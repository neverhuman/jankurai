#!/usr/bin/env bash
# Local-CI entrypoint. Delegates to the same ops/ci/*.sh scripts that
# .github/workflows/*.yml invoke, so local and remote runs are identical
# (modulo runner-environment differences listed in docs/ci-local.md).
#
# Lanes (selectable via $1):
#   quick     ops/ci/quality-gates.sh       (jankurai.yml#test-matrix)
#   coverage  ops/ci/coverage-llvm.sh       (jankurai.yml#coverage-llvm)
#   audit     ops/ci/audit.sh               (jankurai.yml#audit)
#   release   ops/ci/release-audit-gate.sh  (release.yml#audit-gate)
#   container ops/ci/run-in-container.sh    (audit lane inside ubuntu image)
#   all       quick + coverage + audit
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LANE="${1:-all}"

case "$LANE" in
  quick)     bash ops/ci/quality-gates.sh ;;
  coverage)  bash ops/ci/coverage-llvm.sh ;;
  audit)     bash ops/ci/audit.sh ;;
  release)   RELEASE_TAG="${LOCAL_RELEASE_TAG:-}" bash ops/ci/release-audit-gate.sh ;;
  container) bash ops/ci/run-in-container.sh "bash ops/ci/${2:-audit.sh}" ;;
  all)
    bash ops/ci/quality-gates.sh
    bash ops/ci/coverage-llvm.sh
    bash ops/ci/audit.sh
    ;;
  *) echo "usage: $0 {quick|coverage|audit|release|container|all}" >&2; exit 2 ;;
esac

printf '\n\033[1;32mlocal CI lane "%s" passed\033[0m\n' "$LANE"
