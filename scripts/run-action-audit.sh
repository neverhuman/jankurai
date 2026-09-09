#!/usr/bin/env bash
set -euo pipefail
# Arguments remain arrays/quoted strings. No repository scripts are evaluated.
bin="$1"
audit_path="$2"
mode="$3"
baseline="$4"
floor="$5"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
node "$script_dir/action-gate.mjs" --validate-floor "$floor"
case "$mode" in standard|advisory|ratchet|release) ;; *) echo 'unsupported audit mode' >&2; exit 1 ;; esac
: "${RUNNER_TEMP:?RUNNER_TEMP must name the runner temporary directory}"
umask 077
report_dir="$(mktemp -d "$RUNNER_TEMP/jankurai-action.XXXXXXXX")"
if [ -n "${GITHUB_OUTPUT:-}" ]; then
  printf 'report-json=%s\nreport-md=%s\nreport-directory=%s\n' "$report_dir/repo-score.json" "$report_dir/repo-score.md" "$report_dir" >> "$GITHUB_OUTPUT"
fi
args=(audit "$audit_path" --full --mode "$mode" --no-score-history
  --json "$report_dir/repo-score.json" --md "$report_dir/repo-score.md"
  --sarif "$report_dir/jankurai.sarif" --github-step-summary "$report_dir/summary.md"
  --repair-queue-jsonl "$report_dir/repair-queue.jsonl")
if [ "$mode" = ratchet ] || [ "$mode" = release ]; then args+=(--baseline "$baseline"); fi
# The additional Action floor never replaces a stronger repository policy.
audit_status=0
"$bin" "${args[@]}" || audit_status=$?
gate_status=0
node "$script_dir/action-gate.mjs" "$report_dir/repo-score.json" "$floor" "$mode" || gate_status=$?
if [ -n "${GITHUB_STEP_SUMMARY:-}" ] && [ -f "$report_dir/summary.md" ] && [ ! -L "$report_dir/summary.md" ]; then
  cat "$report_dir/summary.md" >> "$GITHUB_STEP_SUMMARY"
fi
# Preserve scanner, policy, cancellation and execution failures even if a report passes.
if [ "$audit_status" -ne 0 ]; then exit "$audit_status"; fi
exit "$gate_status"
