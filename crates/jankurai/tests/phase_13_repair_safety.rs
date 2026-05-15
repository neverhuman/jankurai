use jankurai::validation::{self, ArtifactSchema};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

#[test]
fn repair_plan_emits_risk_and_eligibility_metadata() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    let report_path = write_report(
        dir.path(),
        "HLT-011-PROMPT-INJECTION",
        "high",
        "AGENTS.md",
        "agent",
        "security",
        "just security",
    );
    let plan_path = write_repair_plan(dir.path(), &report_path);

    let plan = read_json(&plan_path);
    validation::validate_value(dir.path(), ArtifactSchema::RepairPlan, &plan).unwrap();
    assert_eq!(plan["plan_mode"], "dry-run");
    assert_eq!(plan["planned_edits"][0]["operation"], "modify");
    assert_eq!(plan["planned_edits"][0]["risk_level"], "high");
    assert_eq!(
        plan["planned_edits"][0]["repair_eligibility"],
        "agent-assisted"
    );
    assert!(plan["planned_commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "just security"));
    assert!(plan["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "security"));
    assert!(plan["human_approval_requirements"]
        .as_array()
        .unwrap()
        .is_empty());

    let packet = &plan["packets"][0];
    assert_eq!(packet["repair_eligibility"], "agent-assisted");
    assert_eq!(packet["risk_level"], "high");
    assert!(packet["eligibility_reason"]
        .as_str()
        .unwrap()
        .contains("prompt injection"));
}

#[test]
fn secret_sprawl_packet_is_never_auto_and_critical() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    let report_path = write_report(
        dir.path(),
        "HLT-010-SECRET-SPRAWL",
        "critical",
        "docs/leak.md",
        "standard",
        "security",
        "just security",
    );
    let plan_path = write_repair_plan(dir.path(), &report_path);
    let plan = read_json(&plan_path);
    let packet = &plan["packets"][0];

    assert_eq!(packet["repair_eligibility"], "never-auto");
    assert_eq!(packet["risk_level"], "critical");
    assert!(!packet["human_review_required"].as_bool().unwrap());
}

#[test]
fn auto_pr_request_is_blocked_for_high_risk_packet() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    let report_path = write_report(
        dir.path(),
        "HLT-011-PROMPT-INJECTION",
        "high",
        "AGENTS.md",
        "agent",
        "security",
        "just security",
    );
    let plan_path = write_repair_plan(dir.path(), &report_path);
    let run_path = run_repair(
        dir.path(),
        &plan_path,
        &["--dry-run", "--auto-pr", "--max-risk", "low"],
    );
    let run = read_json(&run_path);

    validation::validate_value(dir.path(), ArtifactSchema::RepairRun, &run).unwrap();
    assert_eq!(run["auto_pr_status"], "blocked");
    assert_eq!(run["blocked_packets"].as_array().unwrap().len(), 1);
    assert_eq!(run["blocked_packets"][0]["risk_level"], "high");
    assert_eq!(
        run["blocked_packets"][0]["repair_eligibility"],
        "agent-assisted"
    );
}

#[test]
fn auto_pr_request_can_be_eligible_dry_run_only_for_agent_assisted_medium_packet_with_max_medium() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    let report_path = write_report(
        dir.path(),
        "HLT-017-OPAQUE-OBSERVABILITY",
        "high",
        "crates/service/src/lib.rs",
        "tools",
        "observability",
        "just fast",
    );
    let plan_path = write_repair_plan(dir.path(), &report_path);
    let run_path = run_repair(
        dir.path(),
        &plan_path,
        &["--dry-run", "--auto-pr", "--max-risk", "medium"],
    );
    let run = read_json(&run_path);

    validation::validate_value(dir.path(), ArtifactSchema::RepairRun, &run).unwrap();
    assert_eq!(run["auto_pr_status"], "eligible-dry-run-only");
    assert!(run["blocked_packets"].as_array().unwrap().is_empty());
    assert_eq!(run["risk_summary"]["medium"], 1);
}

#[test]
fn repair_plan_widens_ci_workflow_scope_for_repo_policy_files() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    let report_path = write_report_with_text(
        dir.path(),
        "HLT-042-CI-LOCAL-PARITY",
        "high",
        ".github/workflows/ci.yml",
        "ops",
        "security",
        "bash scripts/ci-local.sh quick",
        "workflow commands inline the local runner; add scripts/ci-local.sh, scripts/ci-doctor.sh, and rust-toolchain.toml",
        "workflow commands should move into scripts and the pinned toolchain file",
        "repair safely",
    );
    let plan_path = write_repair_plan(dir.path(), &report_path);
    let plan = read_json(&plan_path);

    let packet = &plan["packets"][0];
    assert_eq!(packet["repair_eligibility"], "agent-assisted");
    assert!(!packet["human_review_required"].as_bool().unwrap());
    assert!(packet["allowed_paths"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "scripts/ci-local.sh"));
    assert!(packet["allowed_paths"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "scripts/ci-doctor.sh"));
    assert!(packet["allowed_paths"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "rust-toolchain.toml"));
}

#[test]
fn repair_plan_marks_unsafe_inferred_paths_human_required() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    let report_path = write_report_with_text(
        dir.path(),
        "HLT-042-CI-LOCAL-PARITY",
        "high",
        ".github/workflows/ci.yml",
        "ops",
        "security",
        "bash scripts/ci-local.sh quick",
        "workflow commands should update docs/testing.md and packages/web/src/App.tsx",
        "the fix needs docs/testing.md and packages/web/src/App.tsx",
        "repair safely",
    );
    let plan_path = write_repair_plan(dir.path(), &report_path);
    let plan = read_json(&plan_path);

    let packet = &plan["packets"][0];
    assert_eq!(packet["repair_eligibility"], "agent-assisted");
    assert!(!packet["human_review_required"].as_bool().unwrap());
    assert!(packet["eligibility_reason"]
        .as_str()
        .unwrap()
        .contains("required fix path outside allowed_paths"));
    assert!(!packet["allowed_paths"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "docs/testing.md"));
}

#[test]
fn repair_rejects_unknown_max_risk() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    let report_path = write_report(
        dir.path(),
        "HLT-017-OPAQUE-OBSERVABILITY",
        "medium",
        "docs/testing.md",
        "standard",
        "audit",
        "just fast",
    );
    let plan_path = write_repair_plan(dir.path(), &report_path);
    let output = Command::new(binary_path())
        .arg("repair")
        .arg(dir.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--dry-run")
        .arg("--max-risk")
        .arg("reckless")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown --max-risk"),
        "expected max-risk failure, got {stderr}"
    );
}

#[test]
fn repair_run_output_validates_against_schema() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    let report_path = write_report(
        dir.path(),
        "HLT-017-OPAQUE-OBSERVABILITY",
        "medium",
        "docs/testing.md",
        "standard",
        "audit",
        "just fast",
    );
    let plan_path = write_repair_plan(dir.path(), &report_path);
    let run_path = run_repair(dir.path(), &plan_path, &["--dry-run"]);
    let run = read_json(&run_path);

    validation::validate_value(dir.path(), ArtifactSchema::RepairRun, &run).unwrap();
    assert_eq!(run["auto_pr_status"], "not-requested");
    assert_eq!(run["planned_packets"], 1);
    assert_eq!(run["execution_mode"], "dry-run");
    assert!(run.get("auto_pr_draft").is_none());
    assert!(run["applied_edits"].as_array().unwrap().is_empty());
    assert!(run["skipped_edits"].as_array().unwrap().is_empty());
    assert!(run["files_written"].as_array().unwrap().is_empty());
    assert!(run["proof_evidence_index"].is_null());
    assert!(run["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "audit"));
}

fn write_report(
    repo: &Path,
    rule_id: &str,
    severity: &str,
    path: &str,
    owner: &str,
    lane: &str,
    rerun_command: &str,
) -> PathBuf {
    write_report_with_text(
        repo,
        rule_id,
        severity,
        path,
        owner,
        lane,
        rerun_command,
        &format!("{rule_id} problem"),
        &format!("{rule_id} reason"),
        "repair safely",
    )
}

#[allow(clippy::too_many_arguments)]
fn write_report_with_text(
    repo: &Path,
    rule_id: &str,
    severity: &str,
    path: &str,
    owner: &str,
    lane: &str,
    rerun_command: &str,
    problem: &str,
    reason: &str,
    agent_fix: &str,
) -> PathBuf {
    fs::create_dir_all(repo.join("target/jankurai")).unwrap();
    let report_path = repo.join("target/jankurai/repo-score.json");
    let report = json!({
        "findings": [{
            "severity": severity,
            "category": "phase-13",
            "path": path,
            "problem": problem,
            "reason": reason,
            "agent_fix": agent_fix,
            "evidence": ["phase 13 fixture"],
            "check_id": rule_id,
            "hardness": "hard",
            "confidence": 0.9,
            "evidence_kind": "fixture",
            "rerun_command": rerun_command,
            "fingerprint": format!("sha256:{rule_id}"),
            "rule_id": rule_id,
            "owner": owner,
            "lane": lane
        }]
    });
    fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    report_path
}

fn write_repair_plan(repo: &Path, report_path: &Path) -> PathBuf {
    let plan_path = repo.join("target/jankurai/repair-plan.json");
    let status = Command::new(binary_path())
        .arg("repair-plan")
        .arg(repo)
        .arg("--from")
        .arg(report_path)
        .arg("--out")
        .arg(&plan_path)
        .status()
        .unwrap();
    assert!(status.success());
    plan_path
}

fn run_repair(repo: &Path, plan_path: &Path, extra_args: &[&str]) -> PathBuf {
    let run_path = repo.join("target/jankurai/repair-run.json");
    let status = Command::new(binary_path())
        .arg("repair")
        .arg(repo)
        .arg("--plan")
        .arg(plan_path)
        .args(extra_args)
        .arg("--out")
        .arg(&run_path)
        .status()
        .unwrap();
    assert!(status.success());
    run_path
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn seed_catalog(repo: &Path) {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/owner-map.json"),
        r#"{"workspace":"fixture","owners":{"agent/":"agent","docs/":"standard","tips/":"paper","crates/":"tools","scripts/":"ops",".github/":"ops","ops/":"ops","rust-toolchain.toml":"tools","target/":"workspace"}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"agent/":{"command":"just security","purpose":"security checks"},"docs/":{"command":"just fast","purpose":"docs checks"},"crates/":{"command":"just fast","purpose":"rust checks"},"scripts/":{"command":"bash scripts/ci-doctor.sh && bash scripts/ci-local.sh quick","purpose":"ci parity"},"rust-toolchain.toml":{"command":"rustup show","purpose":"toolchain parity"},".github/":{"command":"bash scripts/ci-local.sh quick","purpose":"workflow parity"},"ops/":{"command":"bash scripts/ci-doctor.sh && bash scripts/ci-local.sh quick","purpose":"ops parity"}}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/generated-zones.toml"),
        r#"[[zone]]
path = "agent/repo-score.json"
source = "crates/jankurai"
command = "cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md"
read_only = false
"#,
    )
    .unwrap();
}
