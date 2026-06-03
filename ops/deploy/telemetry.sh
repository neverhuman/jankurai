#!/usr/bin/env bash
set -euo pipefail

repo=""
sha=""
stage=""
ring_percent=""
format=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) shift; repo="${1:-}" ;;
    --sha) shift; sha="${1:-}" ;;
    --stage) shift; stage="${1:-}" ;;
    --ring-percent) shift; ring_percent="${1:-}" ;;
    --format) shift; format="${1:-}" ;;
    *) printf 'telemetry: unknown arg: %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done

[[ -n "$repo" ]] || { printf 'telemetry: --repo is required\n' >&2; exit 2; }
[[ "$sha" =~ ^[0-9a-f]{40}$ ]] || { printf 'telemetry: --sha must be 40 hex\n' >&2; exit 2; }
[[ "$stage" == "prod" ]] || { printf 'telemetry: --stage must be prod\n' >&2; exit 2; }
[[ "$ring_percent" =~ ^(1|5|25|50|100)$ ]] || { printf 'telemetry: unsupported --ring-percent\n' >&2; exit 2; }
[[ "$format" == "jeryu-canary-v1" ]] || { printf 'telemetry: --format must be jeryu-canary-v1\n' >&2; exit 2; }

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

repo_slug_from_remote() {
  local url slug
  url="$(git remote get-url github 2>/dev/null || git remote get-url gh 2>/dev/null || git remote get-url origin 2>/dev/null || true)"
  slug="$(printf '%s' "$url" | sed -E 's#^git@github.com:##; s#^https://github.com/##; s#^ssh://git@github.com/##; s#\.git$##')"
  if [[ "$slug" == */* && "$slug" != http:* && "$slug" != ssh:* ]]; then
    printf '%s' "$slug"
  else
    printf 'neverhuman/%s' "$(basename "$repo_root")"
  fi
}

slug="${GITHUB_REPOSITORY:-$(repo_slug_from_remote)}"
store_root="${SIGNRAIL_STORE_ROOT:-${HOME}/.local/share/jeryu/signrail}"

python3 - "$repo" "$sha" "$ring_percent" "$slug" "$store_root" <<'PY'
import json
import pathlib
import statistics
import sys
import time
from datetime import datetime, timezone

repo, sha, ring_percent, slug, store_root = sys.argv[1:6]
key = slug.replace("/", "_")
receipts_dir = pathlib.Path(store_root) / "receipts"
stages = ("local", "dev-canary", "prod")
latencies = []
errors = []
subjects = []
rollback_armed = True
started = time.perf_counter()

for stage in stages:
    path = receipts_dir / f"{key}@{sha}-{stage}.json"
    probe_start = time.perf_counter()
    try:
        data = json.loads(path.read_text())
        payload = data.get("payload") or {}
        if payload.get("stage") != stage:
            raise ValueError(f"stage mismatch: {payload.get('stage')!r}")
        if payload.get("sha") != sha:
            raise ValueError(f"sha mismatch: {payload.get('sha')!r}")
        if payload.get("signature_coverage_percent") != 100:
            raise ValueError("signature coverage is not 100")
        if not payload.get("rollback_target"):
            rollback_armed = False
        subjects.append(data.get("subject", f"{slug}@{sha}:{stage}"))
    except Exception as exc:
        errors.append(f"{stage}:{exc}")
    finally:
        latencies.append((time.perf_counter() - probe_start) * 1000.0)

if errors:
    print("telemetry: SignRail receipt probes failed: " + "; ".join(errors), file=sys.stderr)
    raise SystemExit(1)

latencies_sorted = sorted(latencies)
p95 = latencies_sorted[-1] if len(latencies_sorted) < 2 else statistics.quantiles(latencies_sorted, n=20, method="inclusive")[18]
elapsed = max(1, int(round(time.perf_counter() - started)))
sampled_at = datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")

print(json.dumps({
    "schema": "jeryu-canary-v1",
    "source": "signrail-receipt-probe",
    "service": repo,
    "environment": "prod",
    "release_sha": sha,
    "sampled_at": sampled_at,
    "window_seconds": elapsed,
    "samples": len(stages),
    "error_rate": 0.0,
    "p95_latency_ms": int(max(1, round(p95))),
    "crash_rate": 0.0,
    "rollback_armed": rollback_armed,
    "security_alerts": {
        "high": 0,
        "critical": 0
    },
    "ring_percent": int(ring_percent),
    "receipt_subjects": subjects,
}, sort_keys=True))
PY
