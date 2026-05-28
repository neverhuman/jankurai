#!/usr/bin/env bash
# Post-main GitHub shadow: after the internal GitLab merge succeeds, mirror the
# accepted main commit through Jeryu's local shadow config and write a receipt.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

ensure_dir "${ARTIFACT_ROOT}"

shadow_json="${ARTIFACT_ROOT}/jeryu-shadow.json"
shadow_md="${ARTIFACT_ROOT}/jeryu-shadow.md"
expected_origin="ssh://git@127.0.0.1:2224/root/jankurai.git"
repo_root="${CI_ROOT}"
branch="${CI_COMMIT_BRANCH:-$(git -C "${repo_root}" branch --show-current 2>/dev/null || true)}"
commit="${CI_COMMIT_SHA:-$(git -C "${repo_root}" rev-parse HEAD 2>/dev/null || echo unknown)}"
origin_url="$(git -C "${repo_root}" remote get-url origin 2>/dev/null || echo unknown)"
shadow_cfg="${repo_root}/.jeryu/local/repos/jankurai.toml"
shadow_command="jeryu repo shadow --repo root/jankurai"

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '%s' "$value"
}

write_receipt() {
  local status="$1"
  local reason="$2"
  cat > "${shadow_json}" <<EOF
{
  "schema_version": "1.0.0",
  "repo": "root/jankurai",
  "branch": "$(json_escape "${branch}")",
  "commit": "$(json_escape "${commit}")",
  "origin_url": "$(json_escape "${origin_url}")",
  "shadow_config": "$(json_escape "${shadow_cfg}")",
  "shadow_command": "$(json_escape "${shadow_command}")",
  "status": "$(json_escape "${status}")",
  "reason": "$(json_escape "${reason}")"
}
EOF
  cat > "${shadow_md}" <<EOF
# Jeryu Shadow Receipt

- Status: \`${status}\`
- Reason: ${reason}
- Branch: \`${branch}\`
- Commit: \`${commit}\`
- Origin: \`${origin_url}\`
- Shadow config: \`${shadow_cfg}\`
- Shadow command: \`${shadow_command}\`
EOF
}

if [[ -z "${branch}" || "${branch}" != "main" ]]; then
  write_receipt "skipped" "shadow lane only runs on main"
  note "shadow skipped on branch ${branch:-unknown}"
  exit 0
fi

if [[ "${origin_url}" != "${expected_origin}" ]]; then
  write_receipt "failed" "origin must point at the internal GitLab remote"
  fail "expected origin ${expected_origin}, got ${origin_url}"
fi

if [[ ! -f "${shadow_cfg}" ]]; then
  write_receipt "failed" "missing local Jeryu shadow config"
  fail "missing ${shadow_cfg}"
fi

if ! command -v jeryu >/dev/null 2>&1; then
  write_receipt "failed" "jeryu is not installed on this runner"
  fail "jeryu is required for the GitHub shadow lane"
fi

if [[ -n "$(git -C "${repo_root}" status --porcelain)" ]]; then
  write_receipt "failed" "worktree must be clean before shadowing main"
  fail "refuse to shadow with a dirty worktree"
fi

step "Jeryu shadow main"
if ! jeryu repo shadow --repo root/jankurai; then
  write_receipt "failed" "jeryu repo shadow returned a non-zero exit status"
  fail "jeryu repo shadow failed"
fi

write_receipt "succeeded" "shadow pushed to GitHub mirror from internal main"
assert_nonempty "${shadow_json}"
assert_nonempty "${shadow_md}"
