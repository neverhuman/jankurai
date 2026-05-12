#!/usr/bin/env bash
# Regenerate the Test Surface block in README.md.
# Usage:
#   scripts/render-test-surface.sh            # rewrite README between markers
#   scripts/render-test-surface.sh --check    # exit non-zero if README would change
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "render-test-surface: ripgrep (rg) is required" >&2
  exit 2
fi

count_tests() {
  local pattern="$1"; shift
  local files=("$@")
  [[ ${#files[@]} -eq 0 ]] && { printf 0; return; }
  local total=0
  for f in "${files[@]}"; do
    [[ -e "$f" ]] || continue
    local n
    n=$(rg -c "$pattern" "$f" 2>/dev/null || true)
    [[ -z "$n" ]] && n=0
    total=$((total + n))
  done
  printf '%d' "$total"
}

collect_rust() {
  local subpath="$1"
  local files=()
  while IFS= read -r f; do files+=("$f"); done < <(rg -l '#\[test\]' -g "$subpath" --type rust 2>/dev/null || true)
  printf '%s\n' "${files[@]:-}"
}

# Workspace-wide totals
TOTAL_TESTS=$(rg -c '#\[test\]' --type rust crates 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')
INTEGRATION_FILES=$(find crates -type f -name '*.rs' 2>/dev/null \
  | awk -F/ 'index($0, "/tests/") > 0 && index($0, "/fixtures/") == 0 && index($0, "/expected/") == 0' \
  | awk -F/ '{ for (i=1;i<=NF;i++) if ($i=="tests" && i==NF-1) print; }' \
  | wc -l | tr -d ' ')
PLAYWRIGHT_TESTS=$(rg -c "^\s*test\(" packages/ux-qa/tests 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')

# Category buckets — each file mapped to exactly one bucket
TESTS_DIR=crates/jankurai/tests
LR_DIR=crates/jankurai/src/audit/language_rules

rust=$(count_tests '#\[test\]' \
  "$LR_DIR/rust.rs" \
  "$TESTS_DIR/rust_foundation_smoke.rs")

python=$(count_tests '#\[test\]' \
  "$LR_DIR/python.rs")

comments=$(count_tests '#\[test\]' \
  "$LR_DIR/comments.rs" \
  "$TESTS_DIR/comment_hygiene_bad_behavior.rs")

docker=$(count_tests '#\[test\]' \
  "$LR_DIR/docker.rs")

typescript=$(count_tests '#\[test\]' \
  "$LR_DIR/typescript.rs")

sql=$(count_tests '#\[test\]' \
  "$LR_DIR/sql.rs" \
  "$LR_DIR/sql_migration.rs" \
  "$TESTS_DIR/sql_migration_bad_behavior.rs")

ci=$(count_tests '#\[test\]' \
  "$LR_DIR/ci.rs" \
  "$TESTS_DIR/action_metadata_smoke.rs")

git=$(count_tests '#\[test\]' \
  "$LR_DIR/git.rs" \
  "$LR_DIR/gittools.rs")

release=$(count_tests '#\[test\]' \
  "$LR_DIR/release.rs" \
  "$TESTS_DIR/versions_smoke.rs")

# language_bad_behavior.rs spans many detectors; split per test fn prefix so the
# bar chart credits each language family instead of a synthetic "detectors" bucket.
LBB="$TESTS_DIR/language_bad_behavior.rs"
LBB_OTHER=0
if [[ -f "$LBB" ]]; then
  while IFS= read -r fn; do
    case "$fn" in
      python_*)      python=$((python + 1)) ;;
      sql_*)         sql=$((sql + 1)) ;;
      typescript_*)  typescript=$((typescript + 1)) ;;
      docker_*)      docker=$((docker + 1)) ;;
      ci_*)          ci=$((ci + 1)) ;;
      gittools_*|git_*) git=$((git + 1)) ;;
      release_*)     release=$((release + 1)) ;;
      *)             LBB_OTHER=$((LBB_OTHER + 1)) ;;
    esac
  done < <(awk '/^#\[test\]/{flag=1; next} flag && /^[[:space:]]*fn / { line=$0; sub(/.*fn /, "", line); sub(/\(.*$/, "", line); print line; flag=0 }' "$LBB")
fi

security=$(count_tests '#\[test\]' \
  "$TESTS_DIR/security_evidence_smoke.rs" \
  "$TESTS_DIR/security_evidence_audit_ingest_smoke.rs" \
  "$TESTS_DIR/security_lane_wrapper_smoke.rs" \
  "$TESTS_DIR/web_security_and_repo_rot.rs")

boundaries=$(count_tests '#\[test\]' \
  "$TESTS_DIR/boundaries_audit_ingest_smoke.rs" \
  "$TESTS_DIR/boundaries_manifest_smoke.rs" \
  "$TESTS_DIR/generated_zones_manifest_smoke.rs" \
  "$TESTS_DIR/contract_source_smoke.rs" \
  crates/jankurai/src/boundaries/manifest.rs)

migration=$(count_tests '#\[test\]' \
  "$TESTS_DIR/migration_safety_audit_smoke.rs" \
  "$TESTS_DIR/migration_prompt_verify.rs" \
  "$TESTS_DIR/migration_slice_risk.rs" \
  "$TESTS_DIR/migrate_smoke.rs" \
  "$TESTS_DIR/phase_11_migration_hardening.rs")

uxqa=$(count_tests '#\[test\]' \
  "$TESTS_DIR/ux_qa_audit_ingest_smoke.rs" \
  "$TESTS_DIR/ux_qa_policy_smoke.rs" \
  "$TESTS_DIR/ux_qa_report_smoke.rs")
uxqa=$((uxqa + PLAYWRIGHT_TESTS))

proof=$(count_tests '#\[test\]' \
  "$TESTS_DIR/proof_surface_smoke.rs" \
  "$TESTS_DIR/proofbind_proofmark_smoke.rs" \
  crates/jankurai-proofbind/tests/proofbind_smoke.rs \
  crates/jankurai-proofmark/tests/proofmark_smoke.rs)

audit_bucket=$(count_tests '#\[test\]' \
  "$TESTS_DIR/audit_smoke.rs" \
  "$TESTS_DIR/audit_enforcement_smoke.rs" \
  "$TESTS_DIR/baseline_ratchet_smoke.rs" \
  "$TESTS_DIR/coverage_audit_smoke.rs" \
  "$TESTS_DIR/score_history.rs" \
  "$TESTS_DIR/badge_schema_smoke.rs" \
  "$TESTS_DIR/zyal_audit.rs")
audit_bucket=$((audit_bucket + LBB_OTHER))

conformance=$(count_tests '#\[test\]' \
  "$TESTS_DIR/conformance_runner.rs" \
  "$TESTS_DIR/conformance_fixture_inventory.rs" \
  "$TESTS_DIR/schema_contracts.rs" \
  "$TESTS_DIR/report_compatibility_guard.rs")

vibe=$(count_tests '#\[test\]' \
  "$TESTS_DIR/vibe_coverage_smoke.rs" \
  "$TESTS_DIR/vibe_coverage_semantic.rs" \
  "$TESTS_DIR/vibe_detector_fixtures.rs")

phases=$(count_tests '#\[test\]' \
  "$TESTS_DIR/phase_02_rule_contracts.rs" \
  "$TESTS_DIR/phase_08_09_smoke.rs" \
  "$TESTS_DIR/phase_12_public_evidence.rs" \
  "$TESTS_DIR/phase_13_auto_pr_draft.rs" \
  "$TESTS_DIR/phase_13_optimization_and_exceptions.rs" \
  "$TESTS_DIR/phase_13_patch_execution.rs" \
  "$TESTS_DIR/phase_13_real_mutation.rs" \
  "$TESTS_DIR/phase_13_repair_safety.rs" \
  "$TESTS_DIR/phase10_auth_session_cell_smoke.rs" \
  "$TESTS_DIR/phase10_background_job_cell_smoke.rs" \
  "$TESTS_DIR/phase10_billing_subscription_cell_smoke.rs" \
  "$TESTS_DIR/phase10_notification_shell_cell_smoke.rs" \
  "$TESTS_DIR/phase10_org_team_cell_smoke.rs" \
  "$TESTS_DIR/phase10_periodic_cron_cell_smoke.rs" \
  "$TESTS_DIR/phase10_webhook_receiver_cell_smoke.rs")

# Render a horizontal bar chart. Bars normalised to MAX_BAR characters of '█'.
MAX_BAR=24
LABEL_WIDTH=14

render_block() {
  local categories=(
    "rust"        "$rust"
    "python"      "$python"
    "typescript"  "$typescript"
    "docker"      "$docker"
    "sql"         "$sql"
    "comments"    "$comments"
    "security"    "$security"
    "boundaries"  "$boundaries"
    "ci"          "$ci"
    "git"         "$git"
    "release"     "$release"
    "migration"   "$migration"
    "ux-qa"       "$uxqa"
    "proof"       "$proof"
    "audit"       "$audit_bucket"
    "conformance" "$conformance"
    "vibe"        "$vibe"
    "phases"      "$phases"
  )

  local max=0
  local i
  for ((i=1; i<${#categories[@]}; i+=2)); do
    local v="${categories[i]}"
    (( v > max )) && max=$v
  done
  (( max == 0 )) && max=1

  {
    echo '<!-- TEST_SURFACE_START -->'
    echo '_Generated by `scripts/render-test-surface.sh` — do not edit by hand._'
    echo
    echo "- **Total \`#[test]\` functions:** $TOTAL_TESTS across the Rust workspace"
    echo "- **Integration test files:** $INTEGRATION_FILES"
    echo "- **Playwright tests (\`@jankurai/ux-qa\`):** $PLAYWRIGHT_TESTS"
    echo
    echo '```'
    for ((i=0; i<${#categories[@]}; i+=2)); do
      local name="${categories[i]}"
      local count="${categories[i+1]}"
      local width=$(( count * MAX_BAR / max ))
      (( count > 0 && width == 0 )) && width=1
      local bar=""
      local pad=""
      local b
      for ((b=0; b<width; b++)); do bar+="█"; done
      for ((b=width; b<MAX_BAR; b++)); do pad+=" "; done
      printf -- '%-14s %s%s %s\n' "$name" "$bar" "$pad" "$count"
    done
    echo '```'
    echo '<!-- TEST_SURFACE_END -->'
  }
}

README="$ROOT/README.md"

if ! grep -q '<!-- TEST_SURFACE_START -->' "$README"; then
  echo "render-test-surface: README.md is missing the <!-- TEST_SURFACE_START --> marker" >&2
  exit 3
fi

BLOCK_FILE="$(mktemp)"
trap 'rm -f "$BLOCK_FILE" "$README.new"' EXIT
render_block > "$BLOCK_FILE"

awk -v blockfile="$BLOCK_FILE" '
  BEGIN {
    while ((getline line < blockfile) > 0) {
      block[++n] = line
    }
    close(blockfile)
    in_block = 0
    printed = 0
  }
  /<!-- TEST_SURFACE_START -->/ {
    if (!printed) {
      for (i = 1; i <= n; i++) print block[i]
      printed = 1
    }
    in_block = 1
    next
  }
  /<!-- TEST_SURFACE_END -->/ {
    in_block = 0
    next
  }
  { if (!in_block) print }
' "$README" > "$README.new"

if [[ "${1:-}" == "--check" ]]; then
  if ! diff -u "$README" "$README.new" >/dev/null; then
    diff -u "$README" "$README.new" || true
    rm -f "$README.new"
    echo "render-test-surface: README.md is out of date — run scripts/render-test-surface.sh" >&2
    exit 1
  fi
  rm -f "$README.new"
  echo "render-test-surface: README.md is up to date"
  exit 0
fi

mv "$README.new" "$README"
echo "render-test-surface: README.md updated"
