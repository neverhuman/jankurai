#!/usr/bin/env bash
set -euo pipefail

strict="${JANKURAI_SECURITY_STRICT:-0}"

# One JSON object per line after the prefix; parsed by `jankurai security run`.
emit_step() {
  local label="$1"
  local tool="$2"
  local scmd="$3"
  local status="$4"
  local advisory_flag="$5"
  local ec="${6-}"
  local advisory_json="false"
  if [ "$advisory_flag" = "1" ]; then
    advisory_json="true"
  fi

  printf 'jankurai-security-step={"label":"%s","tool":"%s","shell_command":"%s","status":"%s","advisory":%s' \
    "$(json_escape "$label")" \
    "$(json_escape "$tool")" \
    "$(json_escape "$scmd")" \
    "$(json_escape "$status")" \
    "$advisory_json"
  if [ -n "$ec" ]; then
    printf ',"exit_code":%s' "$ec"
  fi
  printf '}\n'
}

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '%s' "$value"
}

run_required() {
  local tool="$1"
  local cmd="$2"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "skip: $tool not installed; advisory outside strict mode"
    emit_step "$tool" "$tool" "$cmd" "skipped" "1" ""
    if [ "$strict" = "1" ]; then
      echo "missing required security tool: $tool" >&2
      exit 1
    fi
    return 0
  fi
  local err code
  err="$(mktemp)"
  if bash -c "$cmd" 2>"$err"; then
    emit_step "$tool" "$tool" "$cmd" "ran" "0" "0"
    if [ -s "$err" ]; then
      cat "$err" >&2
    fi
    rm -f "$err"
    return 0
  else
    code=$?
    emit_step "$tool" "$tool" "$cmd" "failed" "0" "$code"
    if [ -s "$err" ]; then
      cat "$err" >&2
    fi
    rm -f "$err"
    exit "$code"
  fi
}

run_advisory() {
  local tool="$1"
  local cmd="$2"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "skip: $tool not installed; advisory outside strict mode"
    emit_step "$tool" "$tool" "$cmd" "skipped" "1" ""
    return 0
  fi
  local err code
  err="$(mktemp)"
  if bash -c "$cmd" 2>"$err"; then
    emit_step "$tool" "$tool" "$cmd" "ran" "1" "0"
    if [ -s "$err" ]; then
      cat "$err" >&2
    fi
    rm -f "$err"
    return 0
  else
    code=$?
    emit_step "$tool" "$tool" "$cmd" "failed" "1" "$code"
    if [ -s "$err" ]; then
      cat "$err" >&2
    fi
    rm -f "$err"
    if [ "$strict" = "1" ]; then
      exit "$code"
    fi
    return 0
  fi
}

required_tool_names=(gitleaks)
required_commands=(
  "gitleaks detect --source . --redact --no-banner"
)

advisory_tool_names=(cargo-audit npm syft zizmor)
advisory_commands=(
  "db=\"\${JANKURAI_CARGO_AUDIT_DB:-target/jankurai/security/advisory-db}\"; if [ -d \"\$db/.git\" ]; then git -C \"\$db\" pull --ff-only --depth 1; else git clone --depth 1 https://github.com/RustSec/advisory-db.git \"\$db\"; fi; cargo audit --db \"\$db\" --no-fetch --stale"
  "npm audit --audit-level=high"
  "syft . -o spdx-json=target/jankurai/sbom.spdx.json"
  "zizmor .github/workflows"
)

for i in "${!required_tool_names[@]}"; do
  run_required "${required_tool_names[$i]}" "${required_commands[$i]}"
done

for i in "${!advisory_tool_names[@]}"; do
  run_advisory "${advisory_tool_names[$i]}" "${advisory_commands[$i]}"
done
