#!/usr/bin/env bash
# Run an ops/ci/*.sh lane inside the pinned local-CI container so the
# OS, toolchain, and tool versions match GitHub Actions exactly.
#
# Usage:
#   ops/ci/run-in-container.sh ops/ci/quality-gates.sh
#   ops/ci/run-in-container.sh ops/ci/audit.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMAGE="${JANKURAI_CI_IMAGE:-jankurai-ci:local}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker not installed; container-based local CI is unavailable" >&2
  exit 2
fi

if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  echo "==> building ${IMAGE} (first run only)"
  docker build -f "${ROOT}/ops/ci/Dockerfile.ci" -t "${IMAGE}" "${ROOT}"
fi

if [[ $# -eq 0 ]]; then
  echo "usage: $0 <ops/ci/script.sh> [args...]" >&2
  exit 2
fi

exec docker run --rm \
  -v "${ROOT}:/work" \
  -w /work \
  -e CARGO_TERM_COLOR=always \
  "${IMAGE}" \
  bash -lc "$*"
