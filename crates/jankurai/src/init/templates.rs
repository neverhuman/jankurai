pub struct Template {
    pub path: &'static str,
    pub body: &'static str,
}

pub fn template_for_path(path: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.path == path)
}

pub fn body_for_path(path: &str, level: &str, cargo_repo: bool) -> Option<&'static str> {
    if path == "Justfile" {
        if cargo_repo && level == "full" {
            return Some(RUST_FULL_JUSTFILE);
        }
        if matches!(level, "agents" | "score") {
            return Some(MINIMAL_JUSTFILE);
        }
    }
    template_for_path(path).map(|template| template.body)
}

const ADAPTER_POINTER: &str = "<!-- jankurai generated adapter -->\n<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->\nRead `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\nWhen a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\nFor explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.\nFor explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.\nIf jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.\n";
const PROOF_ADAPTER_POINTER: &str = "---\nname: jankurai\ndescription: Jankurai workspace guidance for Codex. Read repo instructions and standard first; phase files are only for explicit phase work.\n---\n\n# jankurai\n\n<!-- jankurai generated adapter -->\n<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->\nRead `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\nWhen a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\nFor explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.\nFor explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.\nRun the proof lane in `agent/test-map.json` for changed paths.\nIf jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.\n";
macro_rules! workflow_adapter {
    ($title:literal, $usage:literal, $receipts:literal, $next:literal, $stop:literal) => {
        concat!(
            "# ",
            $title,
            "\n\n",
            "<!-- jankurai generated adapter -->\n",
            "<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->\n",
            "Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\n",
            "When a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\n",
            "For explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.\n",
            "For explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.\n",
            $usage,
            "\nExpected receipts: ",
            $receipts,
            ".\n",
            "Next command: ",
            $next,
            ".\n",
            "Stop: ",
            $stop,
            ".\n",
            "If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.\n"
        )
    };
}
macro_rules! cell_agents_template {
    ($title:literal, $owner:literal, $forbidden:literal, $proof_lane:literal) => {
        concat!(
            "# ",
            $title,
            "\n\n",
            "<!-- jankurai generated adapter -->\n",
            "<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->\n",
            "Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\n",
            "When a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\n",
            "Owns `",
            $owner,
            "`.\n",
            "Forbidden: ",
            $forbidden,
            ".\n",
            "Proof lane: `",
            $proof_lane,
            "`.\n",
            "If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.\n"
        )
    };
}
const WEB_AGENTS: &str = cell_agents_template!(
    "apps/web/AGENTS.md",
    "apps/web/",
    "product truth, backend authority, and direct DB writes",
    "rendered UX / Playwright"
);
const API_AGENTS: &str = cell_agents_template!(
    "apps/api/AGENTS.md",
    "apps/api/",
    "UI-only concerns, direct DB writes, and contract generation",
    "edge handler / contract tests"
);
const DOMAIN_AGENTS: &str = cell_agents_template!(
    "crates/domain/AGENTS.md",
    "crates/domain/",
    "I/O glue, transport routing, and persistence code",
    "unit / property tests"
);
const APPLICATION_AGENTS: &str = cell_agents_template!(
    "crates/application/AGENTS.md",
    "crates/application/",
    "transport handlers, persistence code, and UI concerns",
    "use-case / authz tests"
);
const ADAPTERS_AGENTS: &str = cell_agents_template!(
    "crates/adapters/AGENTS.md",
    "crates/adapters/",
    "domain policy, web UI, and direct persistence truth",
    "adapter integration tests"
);
const WORKERS_AGENTS: &str = cell_agents_template!(
    "crates/workers/AGENTS.md",
    "crates/workers/",
    "request handling, UI behavior, and direct user flow ownership",
    "workflow / replay tests"
);
const CONTRACTS_AGENTS: &str = cell_agents_template!(
    "contracts/AGENTS.md",
    "contracts/",
    "generated clients, handwritten transport glue, and product truth",
    "generation / drift checks"
);
const DB_AGENTS: &str = cell_agents_template!(
    "db/AGENTS.md",
    "db/",
    "application logic, transport routing, and UI concerns",
    "migration / constraint tests"
);
const OPS_AGENTS: &str = cell_agents_template!(
    "ops/AGENTS.md",
    "ops/",
    "product feature code, domain policy, and direct DB writes",
    "security lane / workflow lint"
);
const PYTHON_AI_AGENTS: &str = cell_agents_template!(
    "python/ai-service/AGENTS.md",
    "python/ai-service/",
    "product truth, authorization, repo tooling, and direct DB writes",
    "eval / contract tests"
);
const KICKOFF_WORKFLOW: &str = workflow_adapter!(
    "jankurai kickoff",
    "Use `jankurai kickoff . --intent \"<change request>\" --out target/jankurai/kickoff.json --md target/jankurai/kickoff.md` to turn user intent into a no-write handoff. If changed paths are missing, keep the result planning-safe and ask bounded questions before any mutable command runs.",
    "`target/jankurai/kickoff.json`, `target/jankurai/kickoff.md`",
    "`jankurai context-pack`",
    "the task crosses owners, touches generated zones without source regeneration, or needs a broader proof lane than the receipt can justify"
);
const CONTEXT_PACK_WORKFLOW: &str = workflow_adapter!(
    "jankurai context-pack",
    "Use `jankurai context-pack . --changed <path> --max-tokens 6000 --out target/jankurai/context-pack.json --md target/jankurai/context-pack.md` to turn a bounded change set into a repo-aware context bundle.",
    "`target/jankurai/context-pack.json`, `target/jankurai/context-pack.md`",
    "`jankurai prove`",
    "the task is too broad, owner/test routing is unclear, or generated-zone work needs source regeneration first"
);
const PROVE_WORKFLOW: &str = workflow_adapter!(
    "jankurai prove",
    "Use `jankurai prove . --changed <path> --plan-out target/jankurai/proof-plan.json --plan-md target/jankurai/proof-plan.md` to build a proof plan, then run the proof receipts and evidence index under `target/jankurai/`.",
    "`target/jankurai/proof-plan.json`, `target/jankurai/proof-plan.md`, `target/jankurai/proof-receipts/`, `target/jankurai/evidence-index.json`",
    "`jankurai witness`",
    "commands are unsigned, not in proof lanes or the test map, or the plan would mutate generated zones without allowlisted proof"
);
const WITNESS_WORKFLOW: &str = workflow_adapter!(
    "jankurai witness",
    "Use `jankurai witness . --changed-from origin/main --baseline agent/baselines/main.repo-score.json --out target/jankurai/merge-witness.json --md target/jankurai/merge-witness.md` to compare the current branch against the accepted baseline.",
    "`target/jankurai/merge-witness.json`, `target/jankurai/merge-witness.md`",
    "`jankurai repair-plan`",
    "changed-path routing, generated-zone touches, baseline score delta, or proof coverage cannot be justified"
);
const REPAIR_PLAN_WORKFLOW: &str = workflow_adapter!(
    "jankurai repair-plan",
    "Use `jankurai repair-plan . --from agent/repo-score.json --out target/jankurai/repair-plan.json --md target/jankurai/repair-plan.md` to turn the latest report into bounded repair packets.",
    "`target/jankurai/repair-plan.json`, `target/jankurai/repair-plan.md`",
    "`jankurai repair`",
    "the repair broadens scope, touches generated zones without a source contract, or requires a migration, secret rotation, or external service change"
);
const MINIMAL_JUSTFILE: &str = "# jankurai scaffold Justfile\n\nfast:\n\tjankurai doctor --fail-on critical\n\nscore:\n\tjankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md --score-history agent/score-history.jsonl --score-history-csv agent/score-history.csv\n\ndoctor:\n\tjankurai doctor --fail-on high\n\ncheck: fast score\n";
const RUST_FULL_JUSTFILE: &str = "# jankurai scaffold Justfile\n\nfast:\n\tjankurai doctor --fail-on critical\n\nscore:\n\tjankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md --score-history agent/score-history.jsonl --score-history-csv agent/score-history.csv\n\ndoctor:\n\tjankurai doctor --fail-on high\n\nsecurity:\n\tjankurai security run . --out target/jankurai/security/evidence.json\n\nrust-map:\n\tjankurai rust map .\n\nrust-witness:\n\tjankurai rust witness build .\n\nrust-diagnose:\n\tjankurai rust diagnose .\n\ncheck: fast score security rust-map rust-witness rust-diagnose\n";
pub const PRE_COMMIT_HOOK: &str = r#"#!/usr/bin/env bash
# JANKURAI MANAGED HOOK: pre-commit
set -euo pipefail

if [ "${JANKURAI_SKIP_HOOKS:-}" = "1" ]; then
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
git_dir="$(git rev-parse --git-dir 2>/dev/null || printf '%s/.git' "$repo_root")"
case "$git_dir" in
  /*) ;;
  *) git_dir="$repo_root/$git_dir" ;;
esac
jankurai_dir="$git_dir/jankurai"
mkdir -p "$jankurai_dir"

env_file="$jankurai_dir/env"
if [ -f "$env_file" ]; then
  # shellcheck disable=SC1090
  . "$env_file"
fi

if [ -n "${JANKURAI_PRE_COMMIT_CHAIN:-}" ] && [ -x "$JANKURAI_PRE_COMMIT_CHAIN" ] && [ -z "${JANKURAI_CHAINED_HOOK:-}" ]; then
  JANKURAI_CHAINED_HOOK=1 "$JANKURAI_PRE_COMMIT_CHAIN" "$@"
fi

if [ -n "${JANKURAI_BIN:-}" ] && [ -x "$JANKURAI_BIN" ]; then
  jankurai_cmd="$JANKURAI_BIN"
else
  jankurai_cmd="${JANKURAI_FALLBACK_BIN:-jankurai}"
fi

cd "$repo_root"

report_dir="${JANKURAI_HOOK_REPORT_DIR:-target/jankurai/hooks}"
mkdir -p "$report_dir"
report_json="$report_dir/pre-commit-score.json"
report_md="$report_dir/pre-commit-score.md"
report_history_jsonl="$report_dir/pre-commit-score-history.jsonl"
report_history_csv="$report_dir/pre-commit-score-history.csv"

audit_args=(
  audit .
  --mode advisory
  --json "$report_json"
  --md "$report_md"
  --score-history "$report_history_jsonl"
  --score-history-csv "$report_history_csv"
)

if [ -n "${JANKURAI_HISTORY_MIRROR:-}" ]; then
  audit_args+=(--score-history-mirror "$JANKURAI_HISTORY_MIRROR")
fi
if [ "${JANKURAI_HISTORY_MIRROR_REQUIRED:-}" = "1" ]; then
  audit_args+=(--score-history-mirror-required)
fi
if [ -n "${JANKURAI_SCORE_HISTORY_MAX_ROWS:-}" ]; then
  audit_args+=(--score-history-max-rows "$JANKURAI_SCORE_HISTORY_MAX_ROWS")
fi
if [ -n "${JANKURAI_SCORE_HISTORY_MAX_BYTES:-}" ]; then
  audit_args+=(--score-history-max-bytes "$JANKURAI_SCORE_HISTORY_MAX_BYTES")
fi

if ! "$jankurai_cmd" "${audit_args[@]}"; then
  echo "jankurai pre-commit audit failed; set JANKURAI_SKIP_HOOKS=1 to bypass local hooks" >&2
  exit 1
fi

if [ "${JANKURAI_HOOK_STAGE_ARTIFACTS:-}" = "1" ]; then
  git add -- "$report_json" "$report_md" "$report_history_jsonl" "$report_history_csv" 2>/dev/null || true
fi

report_path="$report_json"
json_int() {
  sed -n "s/^[[:space:]]*\"$1\":[[:space:]]*\([-0-9][0-9]*\).*/\1/p" "$report_path" | head -n 1
}
score="$(json_int score)"
raw_score="$(json_int raw_score)"
minimum_score="$(json_int minimum_score)"
hard_findings="$(json_int hard_findings)"
finding_count="$(grep -c '^[[:space:]]*"check_id":' "$report_path" || true)"

score="${score:-0}"
raw_score="${raw_score:-$score}"
minimum_score="${minimum_score:-85}"
hard_findings="${hard_findings:-0}"
decision="pass"
if [ "$hard_findings" -gt 0 ] || [ "$score" -lt "$minimum_score" ]; then
  decision="fail"
fi

cat > "$jankurai_dir/last-score.env" <<EOF
JANKURAI_SCORE='$score'
JANKURAI_RAW_SCORE='$raw_score'
JANKURAI_FINDINGS='$finding_count'
JANKURAI_HARD_FINDINGS='$hard_findings'
JANKURAI_DECISION='$decision'
JANKURAI_REPORT='${report_json}'
EOF
"#;
pub const PREPARE_COMMIT_MSG_HOOK: &str = r#"#!/usr/bin/env bash
# JANKURAI MANAGED HOOK: prepare-commit-msg
set -euo pipefail

if [ "${JANKURAI_SKIP_HOOKS:-}" = "1" ]; then
  exit 0
fi

message_file="${1:?commit message file is required}"
repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
git_dir="$(git rev-parse --git-dir 2>/dev/null || printf '%s/.git' "$repo_root")"
case "$git_dir" in
  /*) ;;
  *) git_dir="$repo_root/$git_dir" ;;
esac
jankurai_dir="$git_dir/jankurai"

env_file="$jankurai_dir/env"
if [ -f "$env_file" ]; then
  # shellcheck disable=SC1090
  . "$env_file"
fi

if [ -n "${JANKURAI_PREPARE_COMMIT_MSG_CHAIN:-}" ] && [ -x "$JANKURAI_PREPARE_COMMIT_MSG_CHAIN" ] && [ -z "${JANKURAI_CHAINED_HOOK:-}" ]; then
  JANKURAI_CHAINED_HOOK=1 "$JANKURAI_PREPARE_COMMIT_MSG_CHAIN" "$@"
fi

last_score="$jankurai_dir/last-score.env"
if [ ! -f "$last_score" ]; then
  exit 0
fi

# shellcheck disable=SC1090
. "$last_score"

if [ -z "${JANKURAI_SCORE:-}" ] || grep -q '^Jankurai-Score:' "$message_file"; then
  exit 0
fi

{
  printf '\n'
  printf 'Jankurai-Score: %s\n' "$JANKURAI_SCORE"
  printf 'Jankurai-Raw-Score: %s\n' "${JANKURAI_RAW_SCORE:-$JANKURAI_SCORE}"
  printf 'Jankurai-Findings: %s\n' "${JANKURAI_FINDINGS:-0}"
  printf 'Jankurai-Hard-Findings: %s\n' "${JANKURAI_HARD_FINDINGS:-0}"
  printf 'Jankurai-Decision: %s\n' "${JANKURAI_DECISION:-unknown}"
  printf 'Jankurai-Report: %s\n' "${JANKURAI_REPORT:-target/jankurai/hooks/pre-commit-score.json}"
} >> "$message_file"
"#;

pub const TEMPLATES: &[Template] = &[
    Template {
        path: "AGENTS.md",
        body: "# Agent Instructions\n\nRead `agent/JANKURAI_STANDARD.md` first. For explicit phase or MASTER_PLAN work only, read `agent/MASTER_PLAN.md` before `tips/phases/00-phase-index.md`. Keep generated artifacts under their declared source commands.\n",
    },
    Template {
        path: "apps/web/AGENTS.md",
        body: WEB_AGENTS,
    },
    Template {
        path: "apps/api/AGENTS.md",
        body: API_AGENTS,
    },
    Template {
        path: ".cursor/rules/jankurai.mdc",
        body: "---\nalwaysApply: true\n---\n\n<!-- jankurai generated adapter -->\n<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->\nRead `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\nWhen a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\nFor explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.\nFor explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.\nIf jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.\n",
    },
    Template {
        path: "CLAUDE.md",
        body: ADAPTER_POINTER,
    },
    Template {
        path: "GEMINI.md",
        body: ADAPTER_POINTER,
    },
    Template {
        path: "Justfile",
        body: "# jankurai scaffold Justfile\n\nfast:\n\tjankurai doctor --fail-on critical\n\nscore:\n\tjankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md --score-history agent/score-history.jsonl --score-history-csv agent/score-history.csv\n\ndoctor:\n\tjankurai doctor --fail-on high\n\nsecurity:\n\tjankurai security run . --out target/jankurai/security/evidence.json\n\ncheck: fast score security\n",
    },
    Template {
        path: ".gitignore",
        body: "# jankurai scaffold .gitignore\n\n# Keep Jankurai receipts local without hiding the rest of target/.\ntarget/jankurai/\n.jankurai/\n",
    },
    Template {
        path: ".github/copilot-instructions.md",
        body: ADAPTER_POINTER,
    },
    Template {
        path: ".github/instructions/jankurai.instructions.md",
        body: ADAPTER_POINTER,
    },
    Template {
        path: ".github/instructions/jankurai-rust.instructions.md",
        body: "---\napplyTo: \"**/*.rs\"\n---\n\n<!-- jankurai generated adapter -->\n<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->\nRead `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\nWhen a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\nFor explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.\nFor explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.\nIf jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.\n",
    },
    Template {
        path: ".github/instructions/jankurai-web.instructions.md",
        body: "---\napplyTo: \"**/*.{ts,tsx,js,jsx,css}\"\n---\n\n<!-- jankurai generated adapter -->\n<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->\nRead `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\nWhen a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\nFor explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.\nFor explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.\nIf jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.\n",
    },
    Template {
        path: ".github/instructions/jankurai-python-ai.instructions.md",
        body: "---\napplyTo: \"python/ai-service/**/*.py\"\n---\n\n<!-- jankurai generated adapter -->\n<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->\nRead `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\nDo not create or expand Python unless a dated advanced-ML/data exception explicitly approves this path. Python must not own product truth, authorization, repo tools, proof lanes, backend glue, or direct production DB writes.\nIf jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.\n",
    },
    Template {
        path: "contracts/AGENTS.md",
        body: CONTRACTS_AGENTS,
    },
    Template {
        path: ".agents/agents.md",
        body: ADAPTER_POINTER,
    },
    Template {
        path: ".agents/skills/jankurai/SKILL.md",
        body: PROOF_ADAPTER_POINTER,
    },
    Template {
        path: ".agents/workflows/jankurai-audit.md",
        body: "# jankurai audit\n\n<!-- jankurai generated adapter -->\n<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->\nRead `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\nWhen a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\nFor explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.\nFor explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.\nRun `jankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md` for audit.\nIf jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.\n",
    },
    Template {
        path: ".agents/workflows/jankurai-kickoff.md",
        body: KICKOFF_WORKFLOW,
    },
    Template {
        path: ".agents/workflows/jankurai-context-pack.md",
        body: CONTEXT_PACK_WORKFLOW,
    },
    Template {
        path: ".agents/workflows/jankurai-prove.md",
        body: PROVE_WORKFLOW,
    },
    Template {
        path: ".agents/workflows/jankurai-witness.md",
        body: WITNESS_WORKFLOW,
    },
    Template {
        path: ".agents/workflows/jankurai-repair-plan.md",
        body: REPAIR_PLAN_WORKFLOW,
    },
    Template {
        path: ".claude/skills/jankurai/SKILL.md",
        body: PROOF_ADAPTER_POINTER,
    },
    Template {
        path: "agent/JANKURAI_STANDARD.md",
        body: "# jankurai Standard Agent Bootstrap\n\nStandard version: `0.9.0`\n\nRead `docs/agent-native-standard.md` when policy detail matters. Use `agent/owner-map.json`, `agent/test-map.json`, `agent/generated-zones.toml`, `agent/proof-lanes.toml`, `agent/tool-adoption.toml`, and `agent/boundaries.toml` before editing.\n",
    },
    Template {
        path: "crates/domain/AGENTS.md",
        body: DOMAIN_AGENTS,
    },
    Template {
        path: "crates/application/AGENTS.md",
        body: APPLICATION_AGENTS,
    },
    Template {
        path: "crates/adapters/AGENTS.md",
        body: ADAPTERS_AGENTS,
    },
    Template {
        path: "crates/workers/AGENTS.md",
        body: WORKERS_AGENTS,
    },
    Template {
        path: "agent/MASTER_PLAN.md",
        body: "# jankurai Master Plan\n\nRead `agent/JANKURAI_STANDARD.md`, then this file, before phase or audit work.\n\nFor phase work, read `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Pick the earliest incomplete phase whose dependencies can be advanced unless the user names a phase.\n\nWhen asked for planning, produce a worker-ready plan with objective, read-first files, ownership, current state, implementation steps, hard parts, validation, logging, and safe parallel work packets.\n\nAppend start, progress, and finish entries to `tips/phases/logs/<phase>.log`. Keep proof receipts and generated evidence under `target/jankurai/`.\n\nUse `agent/test-map.json` to choose the smallest credible proof lane. For audit, run `jankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md`.\n",
    },
    Template {
        path: "agent/boundaries.toml",
        body: include_str!("../../templates/agent/boundaries.toml"),
    },
    Template {
        path: "agent/generated-zones.toml",
        body: include_str!("../../templates/agent/generated-zones.toml"),
    },
    Template {
        path: "agent/owner-map.json",
        body: include_str!("../../templates/agent/owner-map.json"),
    },
    Template {
        path: "agent/proof-lanes.toml",
        body: include_str!("../../templates/agent/proof-lanes.toml"),
    },
    Template {
        path: "agent/audit-policy.toml",
        body: "minimum_score = 85\nfail_on = [\"critical\", \"high\"]\nadvisory_on = [\"medium\", \"low\"]\n\n[history]\nmax_rows = 500\nmax_bytes = 1048576\ndedupe = \"consecutive-equivalent\"\nmirror_env = \"JANKURAI_HISTORY_MIRROR\"\nmirror_required = false\nmirror_max_rows = 5000\n\n[scan]\nexcluded_paths = [\"tips/\"]\n\n[smart_scan]\n# After a clean full scan, only scan git-status changed files by default.\nfull_scan_interval_secs = 3600\nroulette_rate = 0.10\n",
    },
    Template {
        path: "agent/security-policy.toml",
        body: "schema_version = \"1.0.0\"\nenabled_tools = [\"gitleaks\", \"cargo audit\", \"npm audit\"]\nrequired_tools = []\nadvisory_tools = [\"gitleaks\", \"cargo audit\", \"npm audit\"]\n\n[severity_thresholds]\nfail_lane_on = \"high\"\n",
    },
    Template {
        path: "agent/tool-adoption.toml",
        body: "schema_version = \"1.0.0\"\n\n[[tools]]\nid = \"audit-ci\"\nmode = \"auto\"\n\n[[tools]]\nid = \"proof-routing\"\nmode = \"auto\"\n\n[[tools]]\nid = \"security\"\nmode = \"auto\"\n\n[[tools]]\nid = \"ux-qa\"\nmode = \"auto\"\n\n[[tools]]\nid = \"db-migration-analyze\"\nmode = \"auto\"\n\n[[tools]]\nid = \"contract-drift\"\nmode = \"auto\"\n\n[[tools]]\nid = \"rust-witness\"\nmode = \"auto\"\n",
    },
    Template {
        path: "agent/standard-version.toml",
        body: include_str!("../../templates/agent/standard-version.toml"),
    },
    Template {
        path: "agent/test-map.json",
        body: include_str!("../../templates/agent/test-map.json"),
    },
    Template {
        path: "docs/install.md",
        body: "# Install jankurai\n\nRun `jankurai init --profile rust-ts-postgres --ide all --mode advisory --dry-run`, review the plan, then rerun with `--yes`.\n\nFor Rust services that want runtime repair packets, an optional `witness-rt` crate can emit packets that feed the Rust witness and diagnose flows.\n",
    },
    Template {
        path: "docs/agent-native-standard.md",
        body: "# Agent-Native Standard\n\nKeep product truth in Rust, SQL, and generated contracts. Agents must not add Python except for rare dated advanced-ML/data exceptions under `python/ai-service`. Route every path to an owner and proof lane.\n",
    },
    Template {
        path: "docs/ide-integrations.md",
        body: "# IDE Integrations\n\nAll IDE adapters are thin pointers to `agent/JANKURAI_STANDARD.md`; keep durable policy there or in `docs/`.\n",
    },
    Template {
        path: "docs/exceptions/README.md",
        body: "# jankurai Exceptions\n\nDocument dated exceptions with owner, expiry, migration path, and proof lane.\n",
    },
    Template {
        path: "README-jankurai-scaffold.md",
        body: "# Greenfield scaffold (non-production)\n\nThis tree was bootstrapped with `jankurai init --profile rust-ts-postgres`. Replace this file with a real product README when you have one.\n",
    },
    Template {
        path: "contracts/README.md",
        body: "# Contracts\n\nPut OpenAPI, JSON Schema, or protobuf **sources** here. Generated clients and bindings must live only under paths declared in `agent/generated-zones.toml`.\n",
    },
    Template {
        path: "db/AGENTS.md",
        body: DB_AGENTS,
    },
    Template {
        path: "db/README.md",
        body: "# Database\n\nMigrations live in `db/migrations/`. Optional constraint scripts in `db/constraints/`.\n",
    },
    Template {
        path: "db/migrations/README.md",
        body: "# Migrations\n\nAdd versioned SQL migrations. Regenerate any derived artifacts with the recorded command in `agent/generated-zones.toml`.\n",
    },
    Template {
        path: "db/constraints/README.md",
        body: "# Constraints\n\nDeclare durable database truth (checks, FKs) appropriate to your stack.\n",
    },
    Template {
        path: "docs/architecture/README.md",
        body: "# Architecture\n\nDocument boundaries, owners, proof lanes, and data flow. This stub is not production architecture.\n",
    },
    Template {
        path: "docs/decisions/README.md",
        body: "# Architecture Decision Records\n\nRecord significant decisions with date, status, context, and consequences.\n",
    },
    Template {
        path: "docs/auth/README.md",
        body: "# Authentication\n\nDocument authentication mechanisms, session lifecycles, and token issuance boundaries here.\n",
    },
    Template {
        path: "docs/orgs/README.md",
        body: "# Organizations\n\nDocument tenant isolation, RBAC, and cross-organization boundaries here.\n",
    },
    Template {
        path: "docs/admin/README.md",
        body: "# Admin Tools\n\nDocument elevated privileges, support masquerading, and internal operational routes here.\n",
    },
    Template {
        path: "docs/ai/README.md",
        body: "# AI Product Boundary\n\nPrefer Rust/TypeScript service boundaries. Add Python only for rare dated advanced-ML/data exceptions under `python/ai-service`.\n\nAI services may classify, retrieve, rank, generate, summarize, or recommend. They may not silently own durable product truth. Version prompts and attach eval receipts to releases.\n",
    },
    Template {
        path: "docs/product/README.md",
        body: "# Product Intent\n\nDocument product intent, user-visible guarantees, non-goals, risk tolerance, and proof expectations here.\n",
    },
    Template {
        path: "docs/backups/README.md",
        body: "# Backup And Restore\n\nDocument backup scope, restore proof, retention, RPO/RTO targets, and test evidence here.\n",
    },
    Template {
        path: "docs/compliance/README.md",
        body: "# Compliance Evidence\n\nThis is an evidence shell, not a compliance claim.\n\nMap controls to durable, machine-readable evidence before claiming readiness.\n",
    },
    Template {
        path: "docs/migration/README.md",
        body: "# Migration Plan\n\nDocument legacy inventory, boundary map, migration slices, equivalence proof, rollback, and containment policy here.\n",
    },
    Template {
        path: "docs/migration/boundary-map.md",
        body: "# Boundary Map\n\nMap legacy surfaces to target owners, contracts, databases, generated zones, and proof lanes.\n",
    },
    Template {
        path: "docs/migration/slices/README.md",
        body: "# Migration Slices\n\nEach slice needs intent, owner, changed paths, proof lane, equivalence evidence, rollback plan, and residual risk.\n",
    },
    Template {
        path: "docs/observability/README.md",
        body: "# Observability\n\nDocument logs, metrics, traces, audit events, SLOs, and evidence retention boundaries here.\n",
    },
    Template {
        path: "docs/privacy/README.md",
        body: "# Privacy And PII\n\nClassify data, document retention, access boundaries, deletion flows, and proof lanes here.\n",
    },
    Template {
        path: "docs/security/README.md",
        body: "# Security\n\nDocument threat model, secret policy, dependency scanning, provenance, SBOM, and security evidence here.\n",
    },
    Template {
        path: "ops/AGENTS.md",
        body: OPS_AGENTS,
    },
    Template {
        path: "evals/README.md",
        body: "# Evals\n\nStore eval harness docs and receipt conventions here. Generated eval outputs belong under `target/jankurai/` unless explicitly declared.\n",
    },
    Template {
        path: "evals/golden/README.md",
        body: "# Golden Eval Cases\n\nKeep small, reviewed eval cases here. Do not store secrets or production data.\n",
    },
    Template {
        path: "prompts/README.md",
        body: "# Prompts\n\nVersion prompts here. Each prompt change needs an eval note and owner.\n",
    },
    Template {
        path: "python/ai-service/README.md",
        body: "# Exception-Only AI/Data Service\n\nScaffold only. Do not add Python here unless a dated advanced-ML/data exception exists. Keep retrieval, ranking, and generation behind explicit contracts; do not treat model output as source of truth.\n",
    },
    Template {
        path: "python/ai-service/AGENTS.md",
        body: PYTHON_AI_AGENTS,
    },
    Template {
        path: "tools/security-lane.sh",
        body: "#!/usr/bin/env bash\nset -euo pipefail\n# Scaffold stub: replace with real secret/dependency/SBOM checks before treating this lane as proof.\necho \"security-lane scaffold requires project-specific checks\" >&2\nexit 2\n",
    },
    Template {
        path: "tools/jankurai-rust/witness.sh",
        body: "#!/usr/bin/env bash\nset -euo pipefail\nrepo_root=\"${1:-.}\"\nexec jankurai rust witness build \"$repo_root\"\n",
    },
    Template {
        path: "tools/jankurai-hooks/pre-commit",
        body: PRE_COMMIT_HOOK,
    },
    Template {
        path: "tools/jankurai-hooks/prepare-commit-msg",
        body: PREPARE_COMMIT_MSG_HOOK,
    },
    Template {
        path: "agent/ux-qa.toml",
        body: "outputRoot = \".\"\nartifactRoot = \"target/jankurai/ux-qa\"\nreadyState = \"domcontentloaded\"\ntimeoutMs = 15000\nscreenshotRequired = true\nariaSnapshotRequired = true\naccessibilityScanRequired = true\nrequiredStates = [\"loading\", \"empty\", \"error\", \"success\", \"permission-denied\"]\n",
    },
    Template {
        path: "agent/jankurai-install.toml",
        body: "# jankurai install manifest\n# Generated by jankurai init/update.\n# DO NOT EDIT BY HAND.\n",
    },
    Template {
        path: ".github/workflows/jankurai.yml",
        body: "name: jankurai\n\non:\n  pull_request:\n  push:\n    branches: [main]\n\njobs:\n  audit:\n    runs-on: ubuntu-latest\n    permissions:\n      contents: read\n    steps:\n      - uses: actions/checkout@v6\n        with:\n          fetch-depth: 0\n      - uses: dtolnay/rust-toolchain@stable\n      - name: Install jankurai\n        run: cargo install jankurai --locked\n      - run: jankurai --version\n      - name: jankurai audit\n        run: jankurai audit . --mode advisory --baseline agent/repo-score.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md --sarif target/jankurai/jankurai.sarif --github-step-summary target/jankurai/summary.md --repair-queue-jsonl target/jankurai/repair-queue.jsonl\n      - name: Proofbind verify\n        run: jankurai proofbind verify . --changed-from origin/main\n      - name: Proofmark rust\n        run: jankurai proofmark rust . --obligations target/jankurai/proofbind/obligations.json\n      - name: Rust witness build\n        run: jankurai rust witness build .\n      - name: UX QA smoke\n        run: jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json\n      - uses: actions/upload-artifact@v7\n        if: always()\n        with:\n          name: jankurai-adoption-evidence\n          if-no-files-found: ignore\n          path: |\n            target/jankurai/repo-score.json\n            target/jankurai/repo-score.md\n            target/jankurai/jankurai.sarif\n            target/jankurai/repair-queue.jsonl\n            target/jankurai/proofbind/obligations.json\n            target/jankurai/proofbind/surface-witness.json\n            target/jankurai/proofmark/proofmark-receipt.json\n            target/jankurai/proofmark/proof-receipt.json\n            target/jankurai/rust/witness-graph.json\n            target/jankurai/ux-qa.json\n            target/jankurai/security/evidence.json\n            target/jankurai/migration-report.json\n",
    },
];
