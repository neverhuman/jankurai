use std::fs;
use std::process::Command;
use tempfile::tempdir;

use jankurai::validation::{self, ArtifactSchema};
use sha2::{Digest, Sha256};

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_jankurai")
}

fn git(repo: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn seed_catalog(repo: &std::path::Path) {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/owner-map.json"),
        r#"{"workspace":"fixture","owners":{"agent/":"agent","docs/":"standard","tips/":"paper","target/":"workspace","fixtures/":"tests"}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"agent/":{"command":"cargo test -p jankurai","purpose":"agent checks"},"docs/":{"command":"just score","purpose":"audit"},"fixtures/":{"command":"true","purpose":"fixture proof"}}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/generated-zones.toml"),
        r#"[[zone]]
path = ".jankurai/repo-score.json"
source = "crates/jankurai"
command = "cargo run -p jankurai -- audit . --json .jankurai/repo-score.json --md .jankurai/repo-score.md"
read_only = false
"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/proof-lanes.toml"),
        r#"[[lane]]
name = "fast"
command = "just fast"
purpose = "fast lane"

[[lane]]
name = "audit"
command = "just score"
purpose = "audit lane"

[[lane]]
name = "security"
command = "just security"
purpose = "security lane"

[[lane]]
name = "release"
command = "just check"
purpose = "release lane"

[[lane]]
name = "fixture"
command = "true"
purpose = "integration test fixture"

[[lane]]
name = "fixture-fail"
command = "false"
purpose = "integration test fixture failure"
"#,
    )
    .unwrap();
}

fn run_lane(repo: &std::path::Path, subcommand: &str, changed: &str) -> serde_json::Value {
    let out = tempdir().unwrap();
    let json_path = out.path().join("plan.json");
    let md_path = out.path().join("plan.md");
    let status = Command::new(binary_path())
        .arg(subcommand)
        .arg(repo)
        .arg("--changed")
        .arg(changed)
        .arg("--out")
        .arg(&json_path)
        .arg("--md")
        .arg(&md_path)
        .status()
        .unwrap();
    assert!(status.success(), "{subcommand} failed");
    serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap()
}

#[test]
fn lane_and_proof_emit_same_plan_for_changed_path() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/moonshot.md"), "# moonshot\n").unwrap();

    let lane = run_lane(repo.path(), "lane", "docs/moonshot.md");
    let proof = run_lane(repo.path(), "proof", "docs/moonshot.md");

    assert_eq!(lane["commands"], serde_json::json!(["just score"]));
    assert_eq!(lane["matched_test_map"], serde_json::json!(["docs"]));
    assert_eq!(lane["required_lanes"], serde_json::json!(["audit"]));
    assert_eq!(lane["commands"], proof["commands"]);
    assert_eq!(lane["required_lanes"], proof["required_lanes"]);
    assert!(lane["planned_runs"][0]["lane"] == "audit");
    assert!(lane["planned_runs"][0]["command"] == "just score");
    validation::validate_value(repo.path(), ArtifactSchema::ProofPlan, &lane).unwrap();
}

#[test]
fn lane_marks_unmapped_paths_as_risky() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());

    let plan = run_lane(repo.path(), "lane", "notes/todo.md");
    assert!(plan["risk_notes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|note| note.as_str().unwrap().contains("no test-map proof route")));
    assert!(plan["skipped_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "full"));
    assert!(plan["required_lanes"].as_array().unwrap().is_empty());
    let entries = plan["skipped_lane_entries"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["lane"] == "full"));
}

#[test]
fn prove_rejects_unsigned_command_without_hatch() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    let work = repo.path().join("target/jankurai");
    fs::create_dir_all(&work).unwrap();

    let plan_path = work.join("proof-plan.json");
    let plan = serde_json::json!({
        "schema_version": "1.0.0",
        "standard_version": "0.5.0",
        "repo_root": repo.path().display().to_string(),
        "git_head": "unknown",
        "changed_paths": ["docs/moonshot.md"],
        "matched_owner_map": ["docs/"],
        "matched_test_map": ["docs/"],
        "required_lanes": ["audit"],
        "optional_lanes": ["fast", "security", "release"],
        "skipped_lanes": ["fast", "security", "release"],
        "commands": ["rm -f /tmp/nope"],
        "expected_artifacts": ["target/jankurai/proof-receipts/*.json"],
        "risk_notes": [],
        "human_approval_requirements": [],
        "planned_runs": [{
            "lane": "audit",
            "command": "rm -f /tmp/nope",
            "owner": "standard",
            "changed_paths": ["docs/moonshot.md"],
            "artifacts": ["target/jankurai/logs/*.log"],
            "residual_risk": []
        }]
    });
    fs::write(&plan_path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();

    let output = Command::new(binary_path())
        .arg("prove")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("allowlist") || stderr.contains("allow-unsigned"),
        "{stderr}"
    );
}

#[test]
fn prove_writes_receipts_and_logs() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    let work = repo.path().join("target/jankurai");
    fs::create_dir_all(&work).unwrap();

    let plan_path = work.join("proof-plan.json");
    let receipt_dir = work.join("proof-receipts");
    let evidence_index = work.join("evidence-index.json");
    let plan = serde_json::json!({
        "schema_version": "1.0.0",
        "standard_version": "0.5.0",
        "repo_root": repo.path().display().to_string(),
        "git_head": "unknown",
        "changed_paths": ["docs/moonshot.md"],
        "matched_owner_map": ["docs/"],
        "matched_test_map": ["docs/"],
        "required_lanes": ["audit"],
        "optional_lanes": ["fast", "security", "release"],
        "skipped_lanes": ["fast", "security", "release"],
        "commands": ["true"],
        "expected_artifacts": ["target/jankurai/proof-receipts/*.json"],
        "risk_notes": [],
        "human_approval_requirements": [],
        "planned_runs": [{
            "lane": "fixture",
            "command": "true",
            "owner": "standard",
            "changed_paths": ["docs/moonshot.md"],
            "artifacts": ["target/jankurai/logs/*.log"],
            "residual_risk": []
        }]
    });
    fs::write(&plan_path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();

    fs::create_dir_all(work.join("security")).unwrap();
    let ux_report = r#"{"reports":[{"schemaVersion":"1.4.0","toolVersion":"0.5.0","url":"about:blank","checkedAt":"2026-05-02T12:00:00.000Z","viewport":{"width":1280,"height":720},"metrics":{"scrollWidth":1280,"clientWidth":1280,"scrollHeight":720,"clientHeight":720},"elements":[],"violations":[],"artifacts":[],"summary":{"errors":0,"warnings":0,"byRule":{}},"decision":"pass"}]}"#;
    fs::write(work.join("ux-qa.json"), ux_report).unwrap();
    fs::write(work.join("security/evidence.json"), "{}\n").unwrap();
    fs::create_dir_all(work.join("coverage")).unwrap();
    fs::write(
        work.join("coverage/coverage-audit.json"),
        r#"{"schema_version":1,"generated_by":"jankurai coverage audit","repo_root":".","config_path":"agent/coverage-sources.toml","strict":false,"changed_from":null,"summary":{"status":"pass","sources_total":0,"sources_present":0,"sources_missing":0,"hard_findings":0,"soft_findings":0},"sources":[],"findings":[]}"#,
    )
    .unwrap();
    fs::create_dir_all(repo.path().join(".jankurai")).unwrap();
    fs::write(
        repo.path().join(".jankurai/repo-score.json"),
        "{\"score\":0}\n",
    )
    .unwrap();
    fs::write(work.join("jankurai.sarif"), "{}\n").unwrap();
    fs::write(work.join("summary.md"), "# summary\n").unwrap();
    fs::write(
        work.join("repair-queue.jsonl"),
        "{\"path\":\"docs/moonshot.md\"}\n",
    )
    .unwrap();

    fs::write(
        repo.path().join("agent/boundaries.toml"),
        r#"
[stack]
id = "proof-fixture"

[queues]
adapter_paths = []
event_contract_paths = []
generated_type_paths = []
"#,
    )
    .unwrap();

    let status = Command::new(binary_path())
        .arg("prove")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--out-dir")
        .arg(&receipt_dir)
        .arg("--evidence-index")
        .arg(&evidence_index)
        .status()
        .unwrap();
    assert!(status.success());

    let receipts: Vec<_> = fs::read_dir(&receipt_dir).unwrap().collect();
    assert_eq!(receipts.len(), 1);
    let receipt_path = receipts[0].as_ref().unwrap().path();
    let receipt_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&receipt_path).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::ProofReceipt, &receipt_value).unwrap();
    assert_eq!(receipt_value["lane"], "fixture");
    assert_eq!(receipt_value["exit_code"], 0);
    assert!(receipt_value["log_path"].as_str().unwrap().contains("logs"));
    let evidence_raw = fs::read_to_string(&evidence_index).unwrap();
    assert!(evidence_raw.contains("proof-receipts"));
    let evidence_value: serde_json::Value = serde_json::from_str(&evidence_raw).unwrap();
    assert_eq!(evidence_value["schema_version"], "1.2.0");
    assert_eq!(
        evidence_value["ux_qa_report_path"],
        "target/jankurai/ux-qa.json"
    );
    let expected_digest = format!("sha256:{:x}", Sha256::digest(ux_report.as_bytes()));
    assert_eq!(evidence_value["ux_qa_report_digest"], expected_digest);
    assert_eq!(
        evidence_value["security_evidence_path"],
        "target/jankurai/security/evidence.json"
    );
    assert_eq!(
        evidence_value["repo_score_json_path"],
        ".jankurai/repo-score.json"
    );
    assert_eq!(
        evidence_value["coverage_audit_path"],
        "target/jankurai/coverage/coverage-audit.json"
    );
    assert_eq!(
        evidence_value["sarif_path"],
        "target/jankurai/jankurai.sarif"
    );
    assert_eq!(
        evidence_value["github_step_summary_path"],
        "target/jankurai/summary.md"
    );
    assert_eq!(
        evidence_value["repair_queue_jsonl_path"],
        "target/jankurai/repair-queue.jsonl"
    );
    assert_eq!(
        evidence_value["boundaries_manifest_path"],
        "agent/boundaries.toml"
    );
    validation::validate_value(repo.path(), ArtifactSchema::EvidenceIndex, &evidence_value)
        .unwrap();
}

#[test]
fn prove_continues_with_failures_when_requested() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    let work = repo.path().join("target/jankurai");
    fs::create_dir_all(&work).unwrap();

    let plan_path = work.join("proof-plan.json");
    let receipt_dir = work.join("proof-receipts");
    let evidence_index = work.join("evidence-index.json");
    let plan = serde_json::json!({
        "schema_version": "1.0.0",
        "standard_version": "0.5.0",
        "repo_root": repo.path().display().to_string(),
        "git_head": "unknown",
        "changed_paths": ["docs/moonshot.md"],
        "matched_owner_map": ["docs/"],
        "matched_test_map": ["docs/"],
        "required_lanes": ["audit"],
        "optional_lanes": ["fast", "security", "release"],
        "skipped_lanes": ["fast", "security", "release"],
        "commands": ["false", "true"],
        "expected_artifacts": ["target/jankurai/proof-receipts/*.json"],
        "risk_notes": [],
        "human_approval_requirements": [],
        "planned_runs": [
            {
                "lane": "fixture-fail",
                "command": "false",
                "owner": "standard",
                "changed_paths": ["docs/moonshot.md"],
                "artifacts": ["target/jankurai/logs/*.log"],
                "residual_risk": []
            },
            {
                "lane": "fixture",
                "command": "true",
                "owner": "standard",
                "changed_paths": ["docs/moonshot.md"],
                "artifacts": ["target/jankurai/logs/*.log"],
                "residual_risk": []
            }
        ]
    });
    fs::write(&plan_path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();

    let output = Command::new(binary_path())
        .arg("prove")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--out-dir")
        .arg(&receipt_dir)
        .arg("--evidence-index")
        .arg(&evidence_index)
        .arg("--continue-on-error")
        .output()
        .unwrap();
    assert!(!output.status.success());

    let receipts: Vec<_> = fs::read_dir(&receipt_dir).unwrap().collect();
    assert_eq!(receipts.len(), 2);
    let evidence = fs::read_to_string(&evidence_index).unwrap();
    assert!(evidence.contains("failed_receipts"));
    assert!(evidence.contains("logs"));
}

#[test]
fn prove_shorthand_writes_receipts_with_changed() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    fs::write(
        repo.path().join("agent/test-map.json"),
        r#"{"tests":{"docs/":{"command":"true","purpose":"audit"}}}"#,
    )
    .unwrap();
    let work = repo.path().join("target/jankurai");
    fs::create_dir_all(&work).unwrap();

    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/moonshot.md"), "test").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("prove")
        .arg("--changed")
        .arg("docs/moonshot.md")
        .current_dir(repo.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "prove --changed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let receipts_dir = repo.path().join("target/jankurai/proof-receipts");
    let receipts: Vec<_> = fs::read_dir(receipts_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();

    assert_eq!(receipts.len(), 1);
    let receipt_text = fs::read_to_string(&receipts[0]).unwrap();
    assert!(receipt_text.contains("docs/moonshot.md"));
}

#[test]
fn prove_changed_builds_plan_runs_and_indexes_evidence() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    fs::create_dir_all(repo.path().join("fixtures")).unwrap();
    fs::write(repo.path().join("fixtures/demo.txt"), "demo").unwrap();

    let work = repo.path().join("target/jankurai/phase03-test");
    let plan_path = work.join("proof-plan.json");
    let plan_md = work.join("proof-plan.md");
    let receipt_dir = work.join("proof-receipts");
    let evidence_index = work.join("evidence-index.json");

    let output = Command::new(binary_path())
        .arg("prove")
        .arg(repo.path())
        .arg("--changed")
        .arg("fixtures/demo.txt")
        .arg("--plan-out")
        .arg(&plan_path)
        .arg("--plan-md")
        .arg(&plan_md)
        .arg("--out-dir")
        .arg(&receipt_dir)
        .arg("--evidence-index")
        .arg(&evidence_index)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "prove --changed failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let plan: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::ProofPlan, &plan).unwrap();
    assert_eq!(
        plan["changed_paths"],
        serde_json::json!(["fixtures/demo.txt"])
    );
    assert_eq!(plan["commands"], serde_json::json!(["true"]));
    assert_eq!(plan["planned_runs"][0]["lane"], "fixture");
    assert_eq!(plan["route_decisions"].as_array().unwrap().len(), 1);
    assert_eq!(
        plan["route_decisions"][0]["changed_path"],
        "fixtures/demo.txt"
    );
    assert_eq!(plan["route_decisions"][0]["match_kind"], "directory");
    assert_eq!(plan["route_decisions"][0]["decision"], "pass");

    let plan_md_text = fs::read_to_string(&plan_md).unwrap();
    assert!(plan_md_text.contains("# jankurai Proof Plan"));
    assert!(plan_md_text.contains("## Route Decisions"));

    let receipts: Vec<_> = fs::read_dir(&receipt_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(receipts.len(), 1);
    let receipt_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&receipts[0]).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::ProofReceipt, &receipt_value).unwrap();
    assert_eq!(receipt_value["lane"], "fixture");
    assert_eq!(receipt_value["exit_code"], 0);
    let expected_plan_path = plan_path.to_string_lossy().to_string();
    assert_eq!(
        receipt_value["plan_path"].as_str(),
        Some(expected_plan_path.as_str())
    );
    assert!(
        receipt_value.get("rules_covered").is_none(),
        "custom fixture lanes must not claim rule coverage"
    );

    let evidence_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&evidence_index).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::EvidenceIndex, &evidence_value)
        .unwrap();
    assert_eq!(
        evidence_value["plan_path"].as_str(),
        Some(expected_plan_path.as_str())
    );
    assert_eq!(
        evidence_value["changed_paths"],
        serde_json::json!(["fixtures/demo.txt"])
    );

    let verification_path = work.join("proof-verification.json");
    let verification_md = work.join("proof-verification.md");
    let verify_output = Command::new(binary_path())
        .arg("proof-verify")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--evidence-index")
        .arg(&evidence_index)
        .arg("--out")
        .arg(&verification_path)
        .arg("--md")
        .arg(&verification_md)
        .output()
        .unwrap();
    assert!(
        verify_output.status.success(),
        "proof-verify failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify_output.stdout),
        String::from_utf8_lossy(&verify_output.stderr)
    );
    let verification: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&verification_path).unwrap()).unwrap();
    validation::validate_value(
        repo.path(),
        ArtifactSchema::ProofVerification,
        &verification,
    )
    .unwrap();
    assert_eq!(verification["verdict"], "pass");
    assert!(
        verification
            .get("issues")
            .and_then(serde_json::Value::as_array)
            .map(|issues| issues.is_empty())
            .unwrap_or(true),
        "{:?}",
        verification["issues"]
    );
    assert!(fs::read_to_string(&verification_md)
        .unwrap()
        .contains("# jankurai Proof Verification"));
}

#[test]
fn prove_changed_from_builds_plan_runs_and_records_base_ref() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    git(repo.path(), &["init"]);
    git(
        repo.path(),
        &["config", "user.email", "jankurai@example.test"],
    );
    git(repo.path(), &["config", "user.name", "Jankurai Test"]);

    fs::create_dir_all(repo.path().join("fixtures")).unwrap();
    fs::write(repo.path().join("fixtures/demo.txt"), "before\n").unwrap();
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-m", "base"]);
    let base = git(repo.path(), &["rev-parse", "HEAD"]);

    fs::write(repo.path().join("fixtures/demo.txt"), "after\n").unwrap();
    git(repo.path(), &["add", "fixtures/demo.txt"]);
    git(repo.path(), &["commit", "-m", "change fixture"]);

    let work = repo.path().join("target/jankurai/phase03-changed-from");
    let plan_path = work.join("proof-plan.json");
    let plan_md = work.join("proof-plan.md");
    let receipt_dir = work.join("proof-receipts");
    let evidence_index = work.join("evidence-index.json");

    let output = Command::new(binary_path())
        .arg("prove")
        .arg(repo.path())
        .arg("--changed-from")
        .arg(&base)
        .arg("--plan-out")
        .arg(&plan_path)
        .arg("--plan-md")
        .arg(&plan_md)
        .arg("--out-dir")
        .arg(&receipt_dir)
        .arg("--evidence-index")
        .arg(&evidence_index)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "prove --changed-from failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let plan: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    assert_eq!(plan["base_ref"].as_str(), Some(base.as_str()));
    assert!(plan["changed_paths"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("fixtures/demo.txt")));
    assert_eq!(plan["commands"], serde_json::json!(["true"]));

    let receipts: Vec<_> = fs::read_dir(&receipt_dir).unwrap().collect();
    assert_eq!(receipts.len(), 1);
}

#[test]
fn prove_requires_plan_or_changed_input() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());

    let output = Command::new(binary_path())
        .arg("prove")
        .arg(repo.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("provide --plan, --changed, or --changed-from"),
        "{stderr}"
    );
}

#[test]
fn prove_rejects_plan_combined_with_changed() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    let work = repo.path().join("target/jankurai");
    fs::create_dir_all(&work).unwrap();
    let plan_path = work.join("proof-plan.json");
    fs::write(&plan_path, "{}").unwrap();

    let output = Command::new(binary_path())
        .arg("prove")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--changed")
        .arg("fixtures/demo.txt")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("use either --plan or --changed/--changed-from, not both"),
        "{stderr}"
    );
}

#[test]
fn prove_changed_rejects_stdout_plan_out() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());

    let output = Command::new(binary_path())
        .arg("prove")
        .arg(repo.path())
        .arg("--changed")
        .arg("fixtures/demo.txt")
        .arg("--plan-out")
        .arg("-")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--plan-out must be a file path when prove builds a plan"),
        "{stderr}"
    );
}

#[test]
fn prove_changed_without_runnable_route_fails_after_writing_repairable_evidence() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    fs::create_dir_all(repo.path().join("notes")).unwrap();
    fs::write(repo.path().join("notes/todo.md"), "unrouted\n").unwrap();

    let work = repo.path().join("target/jankurai/unrouted");
    let plan_path = work.join("proof-plan.json");
    let plan_md = work.join("proof-plan.md");
    let receipt_dir = work.join("proof-receipts");
    let evidence_index = work.join("evidence-index.json");

    let output = Command::new(binary_path())
        .arg("prove")
        .arg(repo.path())
        .arg("--changed")
        .arg("notes/todo.md")
        .arg("--plan-out")
        .arg(&plan_path)
        .arg("--plan-md")
        .arg(&plan_md)
        .arg("--out-dir")
        .arg(&receipt_dir)
        .arg("--evidence-index")
        .arg(&evidence_index)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("proof plan contains no runnable proof commands"),
        "{stderr}"
    );
    assert!(plan_path.exists(), "repairable plan artifact should exist");
    assert!(
        evidence_index.exists(),
        "repairable evidence index should exist"
    );
}

#[test]
fn prove_receipts_include_rules_for_named_lanes() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    let work = repo.path().join("target/jankurai");
    fs::create_dir_all(&work).unwrap();

    let plan_path = work.join("security-plan.json");
    let receipt_dir = work.join("security-receipts");
    let evidence_index = work.join("security-evidence-index.json");
    let plan = serde_json::json!({
        "schema_version": "1.0.0",
        "standard_version": "0.5.0",
        "repo_root": repo.path().display().to_string(),
        "git_head": "unknown",
        "changed_paths": ["fixtures/demo.txt"],
        "matched_owner_map": ["fixtures/"],
        "matched_test_map": ["fixtures/"],
        "required_lanes": ["security"],
        "optional_lanes": [],
        "skipped_lanes": [],
        "commands": ["true"],
        "expected_artifacts": ["target/jankurai/proof-receipts/*.json"],
        "risk_notes": [],
        "human_approval_requirements": [],
        "planned_runs": [{
            "lane": "security",
            "command": "true",
            "owner": "tests",
            "changed_paths": ["fixtures/demo.txt"],
            "artifacts": ["target/jankurai/logs/*.log"],
            "residual_risk": []
        }]
    });
    fs::write(&plan_path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();

    let output = Command::new(binary_path())
        .arg("prove")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--out-dir")
        .arg(&receipt_dir)
        .arg("--evidence-index")
        .arg(&evidence_index)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "security proof failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let receipts: Vec<_> = fs::read_dir(&receipt_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(receipts.len(), 1);
    let receipt_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&receipts[0]).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::ProofReceipt, &receipt_value).unwrap();
    let covered = receipt_value["rules_covered"].as_array().unwrap();
    let ids: Vec<&str> = covered
        .iter()
        .map(|entry| entry["rule_id"].as_str().unwrap())
        .collect();

    assert!(ids.contains(&"HLT-010-SECRET-SPRAWL"));
    assert!(ids.contains(&"HLT-020-CI-HARDENING-GAP"));
    for id in ids {
        assert!(
            jankurai::audit::rules::lookup(id).is_some(),
            "unregistered rule id in receipt: {id}"
        );
    }
}
