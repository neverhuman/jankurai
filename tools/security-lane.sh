#!/usr/bin/env bash
set -euo pipefail

strict="${JANKURAI_SECURITY_STRICT:-0}"
security_profile="${JANKURAI_SECURITY_PROFILE:-local}"
case "$security_profile" in
  local|ci|release) ;;
  *)
    echo "unsupported JANKURAI_SECURITY_PROFILE: $security_profile" >&2
    exit 2
    ;;
esac

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

run_tool() {
  local label="$1"
  local tool="$2"
  local cmd="$3"
  local advisory_flag="$4"

  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "skip: $tool not installed; advisory outside strict mode"
    emit_step "$label" "$tool" "$cmd" "skipped" "$advisory_flag" ""
    return 0
  fi

  local out err code
  out="$(mktemp)"
  err="$(mktemp)"
  if bash -c "$cmd" >"$out" 2>"$err"; then
    emit_step "$label" "$tool" "$cmd" "ran" "$advisory_flag" "0"
  else
    code=$?
    emit_step "$label" "$tool" "$cmd" "failed" "$advisory_flag" "$code"
  fi

  if [ -s "$out" ]; then
    cat "$out"
  fi
  if [ -s "$err" ]; then
    cat "$err" >&2
  fi
  rm -f "$out"
  rm -f "$err"

  if [ "${code:-0}" -ne 0 ] && [ "$strict" = "1" ] && [ "$advisory_flag" = "0" ]; then
    exit "$code"
  fi
  return 0
}

run_required() {
  local tool="$1"
  local cmd="$2"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "skip: $tool not installed; advisory outside strict mode"
    emit_step "$tool" "$tool" "$cmd" "skipped" "0" ""
    if [ "$strict" = "1" ]; then
      echo "missing required security tool: $tool" >&2
      exit 1
    fi
    return 0
  fi
  local out err code
  out="$(mktemp)"
  err="$(mktemp)"
  if bash -c "$cmd" >"$out" 2>"$err"; then
    emit_step "$tool" "$tool" "$cmd" "ran" "0" "0"
  else
    code=$?
    emit_step "$tool" "$tool" "$cmd" "failed" "0" "$code"
  fi
  if [ -s "$out" ]; then
    cat "$out"
  fi
  if [ -s "$err" ]; then
    cat "$err" >&2
  fi
  rm -f "$out" "$err"
  if [ "${code:-0}" -ne 0 ]; then
    exit "$code"
  fi
  return 0
}

required_tool_names=(gitleaks)
required_commands=(
  "echo 'security-lane:gitleaks:start' >&2; if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then scan_dir=\"\$(mktemp -d)\"; trap 'rm -rf \"\$scan_dir\"' EXIT; git archive --format=tar HEAD | tar -xf - -C \"\$scan_dir\"; (cd \"\$scan_dir\" && gitleaks detect --no-git --source . --config .gitleaks.toml --gitleaks-ignore-path .gitleaksignore --redact --no-banner); else gitleaks detect --no-git --source . --config .gitleaks.toml --gitleaks-ignore-path .gitleaksignore --redact --no-banner; fi; status=\$?; echo 'security-lane:gitleaks:done' >&2; exit \$status"
)

cargo_audit_command='db="${JANKURAI_CARGO_AUDIT_DB:-${JAIN_PINNED_ADVISORY_DB:-target/jankurai/security/advisory-db}}"; offline="${JANKURAI_SECURITY_OFFLINE:-${CARGO_NET_OFFLINE:-false}}"; case "$offline" in 1|true|TRUE) [ -d "$db" ] || { echo "offline advisory database missing: $db" >&2; exit 1; } ;; *) if [ -d "$db/.git" ]; then git -C "$db" pull --ff-only --depth 1; else git clone --depth 1 https://github.com/RustSec/advisory-db.git "$db"; fi ;; esac; cargo audit --db "$db" --no-fetch --stale'

required_tool_names_ci=(cargo-audit npm zizmor)
required_commands_ci=(
  "$cargo_audit_command"
  "npm audit --audit-level=high"
  "zizmor .github/workflows"
)

if [ "$security_profile" = "release" ]; then
  required_tool_names_ci=(cargo-audit zizmor syft grype)
  required_commands_ci=(
    "$cargo_audit_command"
    "zizmor .github/workflows"
    "syft dir:. -o spdx-json=target/jankurai/sbom.spdx.json"
    "syft dir:. --override-default-catalogers javascript-lock-cataloger -o syft-json=target/jankurai/security/javascript-packages.syft.json && GRYPE_DB_AUTO_UPDATE=false GRYPE_CHECK_FOR_APP_UPDATE=false grype sbom:target/jankurai/security/javascript-packages.syft.json --fail-on high"
  )
fi

for i in "${!required_tool_names[@]}"; do
  run_required "${required_tool_names[$i]}" "${required_commands[$i]}"
done

for i in "${!required_tool_names_ci[@]}"; do
  if [ "$strict" = "1" ]; then
    advisory_flag="0"
  else
    advisory_flag="1"
  fi
  run_tool "${required_tool_names_ci[$i]}" "${required_tool_names_ci[$i]}" "${required_commands_ci[$i]}" "$advisory_flag"
done

if [ "$security_profile" != "release" ]; then
  run_tool "syft" "syft" "syft . -o spdx-json=target/jankurai/sbom.spdx.json" "1"
fi
