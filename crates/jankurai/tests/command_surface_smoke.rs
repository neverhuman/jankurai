use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

use jankurai::commands::bench;
use jankurai::model::{
    AUDITOR_VERSION, PAPER_EDITION, SCHEMA_VERSION, STANDARD_VERSION, TARGET_STACK_ID,
};
use jankurai::validation::{self, ArtifactSchema};

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn run_command(repo: &PathBuf, args: &[&str]) -> (serde_json::Value, String) {
    let out_dir = tempdir().unwrap();
    let json_path = out_dir.path().join("out.json");
    let md_path = out_dir.path().join("out.md");
    let subcommand = args[0];
    let mut cmd = Command::new(binary_path());
    cmd.arg(subcommand)
        .arg(repo)
        .args(&args[1..])
        .arg("--out")
        .arg(&json_path)
        .arg("--md")
        .arg(&md_path);
    let status = cmd.status().unwrap();
    assert!(status.success(), "command failed: {:?}", cmd);
    let json_text = fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_text).unwrap();
    let md_text = fs::read_to_string(&md_path).unwrap();
    (json, md_text)
}

fn run_kickoff(
    repo: &PathBuf,
    intent: &str,
    changed: &[&str],
    extra_args: &[&str],
) -> (serde_json::Value, String) {
    let out_dir = tempdir().unwrap();
    let json_path = out_dir.path().join("kickoff.json");
    let md_path = out_dir.path().join("kickoff.md");
    let mut cmd = Command::new(binary_path());
    cmd.arg("kickoff").arg(repo).arg("--intent").arg(intent);
    for path in changed {
        cmd.arg("--changed").arg(path);
    }
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.arg("--out").arg(&json_path).arg("--md").arg(&md_path);
    let status = cmd.status().unwrap();
    assert!(status.success(), "kickoff failed: {:?}", cmd);
    let json_text = fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_text).unwrap();
    let md_text = fs::read_to_string(&md_path).unwrap();
    (json, md_text)
}

fn run_repair_with_draft(
    repo: &PathBuf,
    plan_path: &PathBuf,
    draft_path: &PathBuf,
) -> (serde_json::Value, serde_json::Value) {
    let run_dir = tempdir().unwrap();
    let run_json = run_dir.path().join("repair-run.json");
    let run_md = run_json.with_extension("md");
    let draft_md = draft_path.with_extension("md");
    let status = Command::new(binary_path())
        .arg("repair")
        .arg(repo)
        .arg("--plan")
        .arg(plan_path)
        .arg("--dry-run")
        .arg("--auto-pr")
        .arg("--max-risk")
        .arg("medium")
        .arg("--out")
        .arg(&run_json)
        .arg("--md")
        .arg(&run_md)
        .arg("--pr-draft-out")
        .arg(draft_path)
        .arg("--pr-draft-md")
        .arg(&draft_md)
        .status()
        .unwrap();
    assert!(status.success());
    let run: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&run_json).unwrap()).unwrap();
    let draft: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(draft_path).unwrap()).unwrap();
    (run, draft)
}

#[test]
fn copy_code_help_surfaces_the_new_command() {
    let output = Command::new(binary_path())
        .arg("copy-code")
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("copy-code"));
    assert!(help.contains("--json"));
    assert!(help.contains("--strict"));
}

#[test]
fn copy_code_command_writes_schema_valid_artifacts() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("schemas")).unwrap();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::copy(
        repo_root().join("schemas/copy-code.schema.json"),
        repo.path().join("schemas/copy-code.schema.json"),
    )
    .unwrap();
    fs::write(
        repo.path().join("src/a.rs"),
        "pub fn run() { println!(\"hi\"); }\n",
    )
    .unwrap();
    fs::write(
        repo.path().join("src/b.rs"),
        "pub fn run() { println!(\"hi\"); }\n",
    )
    .unwrap();

    let json = repo.path().join("copy-code.json");
    let md = repo.path().join("copy-code.md");
    let status = Command::new(binary_path())
        .arg("copy-code")
        .arg(repo.path())
        .arg("--json")
        .arg(&json)
        .arg("--md")
        .arg(&md)
        .status()
        .unwrap();
    assert!(status.success(), "copy-code command failed");

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::CopyCode, &report).unwrap();
    assert_eq!(report["summary"]["hard_classes"], 1);
    assert!(fs::read_to_string(&md).unwrap().contains("## Hard Classes"));
}

#[test]
fn new_planner_commands_emit_stable_json_and_markdown() {
    let repo = tempdir().unwrap();
    assert_eq!(fs::read_dir(repo.path()).unwrap().count(), 0);

    let (registry, registry_md) = run_command(&repo.path().to_path_buf(), &["registry"]);
    assert_eq!(registry["command"], "jankurai registry");
    assert_eq!(registry["status"], "complete");
    assert!(registry_md.starts_with("# jankurai Registry"));
    validation::validate_value(repo.path(), ArtifactSchema::CellRegistry, &registry).unwrap();

    let (cell, cell_md) = run_command(
        &repo.path().to_path_buf(),
        &["cell", "--cell-id", "demo-cell"],
    );
    assert_eq!(cell["command"], "jankurai cell");
    assert_eq!(cell["status"], "complete");
    assert_eq!(cell["mode"], "install-ready");
    assert_eq!(cell["cell_id"], "demo-cell");
    assert_eq!(cell["owner"], "workspace");
    assert!(cell_md.starts_with("# jankurai Cell Plan"));
    validation::validate_value(repo.path(), ArtifactSchema::CellManifest, &cell["manifest"])
        .unwrap();

    let (migrate, migrate_md) = run_command(&repo.path().to_path_buf(), &["migrate"]);
    assert_eq!(migrate["command"], "jankurai migrate");
    assert_eq!(migrate["status"], "complete");
    assert!(migrate_md.starts_with("# jankurai Migration Plan"));

    let (bench, bench_md) = run_command(&repo.path().to_path_buf(), &["bench"]);
    let suite = bench::build_benchmark_suite(repo.path()).unwrap();
    validation::validate_serializable(repo.path(), ArtifactSchema::BenchmarkSuite, &suite).unwrap();
    assert_eq!(bench["suite_id"], "smoke");
    assert!(bench["results"].as_array().unwrap().len() >= 2);
    assert!(bench["summary"]["passed"].as_i64().unwrap() >= 1);
    assert!(bench_md.starts_with("# jankurai Benchmark Report"));
    validation::validate_value(repo.path(), ArtifactSchema::BenchmarkReport, &bench).unwrap();

    let (certify, certify_md) = run_command(&repo.path().to_path_buf(), &["certify"]);
    assert_eq!(certify["standard_version"], STANDARD_VERSION);
    assert!(certify["score"].as_i64().unwrap() >= 0);
    assert!(certify["score"].as_i64().unwrap() <= 100);
    assert_eq!(certify["conformance_level"], "HL0");
    assert!(certify_md.starts_with("# jankurai Certification"));
    validation::validate_value(repo.path(), ArtifactSchema::Certification, &certify).unwrap();

    let (govern, govern_md) = run_command(&repo.path().to_path_buf(), &["govern"]);
    assert_eq!(govern["minimum_score"], 85);
    assert_eq!(govern["update_channel"], "stable");
    assert!(govern_md.starts_with("# jankurai Governance Policy"));
    validation::validate_value(repo.path(), ArtifactSchema::GovernancePolicy, &govern).unwrap();

    let plan_path = repo.path().join("repair-plan.json");
    fs::write(
        &plan_path,
        serde_json::json!({
            "schema_version": "1.0.0",
            "source_report": "agent/repo-score.json",
            "generated_at": "0",
            "target_stack_id": "jankurai:v0.4",
            "plan_mode": "dry-run",
            "planned_edits": [{
                "path": "docs/testing.md",
                "operation": "modify",
                "reason": "add docs",
                "finding_fingerprint": "sha256:test",
                "rule_id": "HLT-017-OPAQUE-OBSERVABILITY",
                "apply_strategy": "none",
                "risk_level": "medium",
                "repair_eligibility": "agent-assisted"
            }],
            "planned_commands": ["just fast"],
            "proof_lanes": ["audit"],
            "rollback_guidance": ["restore docs"],
            "human_approval_requirements": [],
            "packets": [{
                "finding_fingerprint": "sha256:test",
                "finding_path": "docs/testing.md",
                "rule_id": "HLT-017-OPAQUE-OBSERVABILITY",
                "check_id": "HLT-017-OPAQUE-OBSERVABILITY",
                "severity": "medium",
                "owner": "standard",
                "lane": "audit",
                "problem": "opaque observability",
                "why": "opaque observability",
                "permission_profile": "docs-only",
                "allowed_paths": ["docs/"],
                "forbidden_paths": ["reference/"],
                "expected_patch_shape": "add docs",
                "required_proof": ["just fast"],
                "stop_conditions": ["stop"],
                "repair_eligibility": "agent-assisted",
                "risk_level": "medium",
                "eligibility_reason": "observability repairs are typically scoped to telemetry and error receipts",
                "human_review_required": false,
                "rollback_guidance": "restore docs"
            }]
        })
        .to_string(),
    )
    .unwrap();
    let (repair, repair_md) = run_command(
        &repo.path().to_path_buf(),
        &["repair", "--plan", plan_path.to_str().unwrap(), "--dry-run"],
    );
    assert_eq!(repair["status"], "complete");
    assert_eq!(repair["execution_mode"], "dry-run");
    assert_eq!(repair["dry_run"], true);
    assert_eq!(repair["auto_pr_status"], "not-requested");
    assert_eq!(repair["planned_packets"], 1);
    assert!(repair["applied_edits"].as_array().unwrap().is_empty());
    assert!(repair["skipped_edits"].as_array().unwrap().is_empty());
    assert!(repair["files_written"].as_array().unwrap().is_empty());
    assert!(repair["proof_evidence_index"].is_null());
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &repair).unwrap();
    assert!(repair_md.starts_with("# jankurai Repair Run"));
    fs::remove_file(&plan_path).unwrap();

    assert_eq!(fs::read_dir(repo.path()).unwrap().count(), 0);
}

#[test]
fn kickoff_help_is_available() {
    let output = Command::new(binary_path())
        .arg("kickoff")
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("jankurai kickoff"), "{stdout}");
    assert!(stdout.contains("--intent"), "{stdout}");
    assert!(stdout.contains("target/jankurai/kickoff.json"), "{stdout}");
}

#[test]
fn kickoff_minimal_run_emits_no_write_plan_and_questions() {
    let repo = tempdir().unwrap();
    let (kickoff, md) = run_kickoff(
        &repo.path().to_path_buf(),
        "Add a README clarification",
        &[],
        &[],
    );
    assert_eq!(kickoff["command"], "jankurai kickoff");
    assert_eq!(kickoff["intent"], "Add a README clarification");
    assert!(kickoff["changed_paths"].as_array().unwrap().is_empty());
    assert!(kickoff["route_decisions"].as_array().unwrap().is_empty());
    assert!(!kickoff["clarifying_questions"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(kickoff["next_commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|cmd| cmd.as_str().unwrap().contains("context-pack")));
    assert!(kickoff["expected_receipts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "target/jankurai/kickoff.json"));
    assert!(md.starts_with("# jankurai Kickoff"));
    assert!(md.contains("## Forbidden paths"));
    assert!(md.contains("## Proof lanes"));
    validation::validate_value(&repo_root(), ArtifactSchema::Kickoff, &kickoff).unwrap();
}

#[test]
fn kickoff_generated_zone_touches_require_source_regeneration() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("agent")).unwrap();
    fs::write(
        repo.path().join("agent/generated-zones.toml"),
        r#"[[zone]]
path = "generated/openapi.json"
source = "contracts/openapi.yaml"
command = "cargo run -p api-gen -- contracts/openapi.yaml --out generated/openapi.json"
read_only = true
write_policy = "generator_only"
"#,
    )
    .unwrap();

    let (kickoff, _md) = run_kickoff(
        &repo.path().to_path_buf(),
        "Update generated OpenAPI output",
        &["generated/openapi.json"],
        &[],
    );
    let route_decisions = kickoff["route_decisions"].as_array().unwrap();
    assert_eq!(route_decisions.len(), 1);
    assert_eq!(route_decisions[0]["decision"], "read-only");
    assert_eq!(route_decisions[0]["generated_zone"], true);
    let stop_conditions = kickoff["stop_conditions"].as_array().unwrap();
    assert!(stop_conditions.iter().any(|item| item
        .as_str()
        .unwrap()
        .contains("updated before regenerating")));
    let touches = kickoff["generated_zone_touches"].as_array().unwrap();
    assert_eq!(touches.len(), 1);
    assert_eq!(touches[0]["source"], "contracts/openapi.yaml");
    assert!(touches[0]["reason"]
        .as_str()
        .unwrap()
        .contains("regenerated"));
    validation::validate_value(&repo_root(), ArtifactSchema::Kickoff, &kickoff).unwrap();
}

#[test]
fn kickoff_high_risk_unmapped_path_asks_for_the_missing_lane() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("agent")).unwrap();
    fs::write(
        repo.path().join("agent/owner-map.json"),
        r#"{"workspace":"fixture","owners":{"db/":"db","target/":"workspace"}}"#,
    )
    .unwrap();
    fs::write(
        repo.path().join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"db/":{"command":"cargo test -p jankurai db-proof","purpose":"db proof"}}}"#,
    )
    .unwrap();
    fs::write(
        repo.path().join("agent/proof-lanes.toml"),
        r#"[[lane]]
name = "audit"
command = "cargo test -p jankurai audit"
purpose = "fixture proof"
"#,
    )
    .unwrap();

    let (kickoff, _md) = run_kickoff(
        &repo.path().to_path_buf(),
        "Update the database migration",
        &["db/migrations/001.sql"],
        &[],
    );
    let route_decisions = kickoff["route_decisions"].as_array().unwrap();
    assert_eq!(route_decisions.len(), 1);
    assert_eq!(route_decisions[0]["decision"], "human-review");
    assert_eq!(route_decisions[0]["proof_lane"], "unmapped");

    let questions = kickoff["clarifying_questions"].as_array().unwrap();
    assert!(
        questions.iter().any(|question| {
            question["question"]
                .as_str()
                .unwrap()
                .contains("Which proof lane should own `db/migrations/001.sql`?")
        }),
        "expected a missing-lane question: {questions:?}"
    );
    assert!(
        questions.iter().all(|question| {
            !question["question"]
                .as_str()
                .unwrap()
                .contains("through the `unmapped` proof lane")
        }),
        "question should not ask the user to confirm an unmapped proof lane: {questions:?}"
    );
    validation::validate_value(&repo_root(), ArtifactSchema::Kickoff, &kickoff).unwrap();
}

#[test]
fn repair_command_emits_auto_pr_draft_artifact() {
    let repo = tempdir().unwrap();
    assert_eq!(fs::read_dir(repo.path()).unwrap().count(), 0);
    fs::create_dir_all(repo.path().join("agent")).unwrap();
    fs::write(
        repo.path().join("agent/owner-map.json"),
        r#"{"workspace":"fixture","owners":{"agent/":"agent","docs/":"standard","paper/":"paper","reference/":"read-only","target/":"workspace","crates/":"tools"}}"#,
    )
    .unwrap();
    fs::write(
        repo.path().join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"docs/":{"command":"true","purpose":"fixture docs proof"},"agent/":{"command":"true","purpose":"fixture agent proof"}}}"#,
    )
    .unwrap();
    fs::write(
        repo.path().join("agent/proof-lanes.toml"),
        r#"[[lane]]
name = "audit"
command = "true"
purpose = "fixture proof"
"#,
    )
    .unwrap();
    fs::write(
        repo.path().join("agent/generated-zones.toml"),
        r#"[[zone]]
path = "agent/repo-score.json"
source = "crates/jankurai"
command = "cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md"
read_only = false
"#,
    )
    .unwrap();

    let plan_path = repo.path().join("repair-plan.json");
    fs::write(
        &plan_path,
        serde_json::json!({
            "schema_version": "1.0.0",
            "source_report": "agent/repo-score.json",
            "generated_at": "0",
            "target_stack_id": "jankurai:v0.4",
            "plan_mode": "dry-run",
            "planned_edits": [{
                "path": "docs/testing.md",
                "operation": "modify",
                "reason": "add docs",
                "finding_fingerprint": "sha256:test",
                "rule_id": "HLT-017-OPAQUE-OBSERVABILITY",
                "apply_strategy": "append-text",
                "risk_level": "medium",
                "repair_eligibility": "agent-assisted"
            }],
            "planned_commands": ["false"],
            "proof_lanes": ["audit"],
            "rollback_guidance": ["restore docs"],
            "human_approval_requirements": [],
            "packets": [{
                "finding_fingerprint": "sha256:test",
                "finding_path": "docs/testing.md",
                "rule_id": "HLT-017-OPAQUE-OBSERVABILITY",
                "check_id": "HLT-017-OPAQUE-OBSERVABILITY",
                "severity": "medium",
                "owner": "standard",
                "lane": "audit",
                "problem": "opaque observability",
                "why": "opaque observability",
                "permission_profile": "docs-only",
                "allowed_paths": ["docs/"],
                "forbidden_paths": ["reference/"],
                "expected_patch_shape": "add docs",
                "required_proof": ["true"],
                "stop_conditions": ["stop"],
                "repair_eligibility": "agent-assisted",
                "risk_level": "medium",
                "eligibility_reason": "observability repairs are typically scoped to telemetry and error receipts",
                "human_review_required": false,
                "rollback_guidance": "restore docs"
            }]
        })
        .to_string(),
    )
    .unwrap();
    let draft_path = repo.path().join("repair-pr-draft.json");
    let (repair, draft) =
        run_repair_with_draft(&repo.path().to_path_buf(), &plan_path, &draft_path);

    assert_eq!(repair["auto_pr_status"], "eligible-dry-run-only");
    assert_eq!(draft["status"], "draft-only");
    assert!(draft["artifact_links"]
        .as_array()
        .unwrap()
        .iter()
        .any(|link| link == "agent/repo-score.json"));
    assert!(draft["pr_body"]
        .as_str()
        .unwrap()
        .contains("Eligible Packets"));
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &repair).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairPrDraft, &draft).unwrap();
}

#[test]
fn certified_cells_are_schema_valid_and_evidence_bound() {
    let repo = repo_root();

    let (registry, _registry_md) = run_command(&repo, &["registry"]);
    validation::validate_value(&repo, ArtifactSchema::CellRegistry, &registry).unwrap();
    let cells = registry["cells"].as_array().unwrap();
    let audit_log = cells
        .iter()
        .find(|cell| cell["cell_id"] == "audit-log")
        .expect("audit-log cell");
    let crud = cells
        .iter()
        .find(|cell| cell["cell_id"] == "crud-resource")
        .expect("crud-resource cell");
    let rbac = cells
        .iter()
        .find(|cell| cell["cell_id"] == "rbac")
        .expect("rbac cell");
    assert_eq!(audit_log["certification_status"], "certified");
    assert!(audit_log["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "audit"));
    assert_eq!(crud["dependencies"].as_array().unwrap()[0], "audit-log");
    assert_eq!(rbac["certification_status"], "certified");
    assert_eq!(rbac["dependencies"].as_array().unwrap()[0], "crud-resource");
    assert!(rbac["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "security"));

    let (cell, _cell_md) = run_command(&repo, &["cell", "--cell-id", "audit-log"]);
    validation::validate_value(&repo, ArtifactSchema::CellManifest, &cell["manifest"]).unwrap();
    assert!(cell["manifest"].is_object());
    assert_eq!(cell["install_plan"]["dry_run"], true);
    assert_eq!(cell["install_plan"]["conflict_policy"], "never-overwrite");

    let (prove, _prove_md) = run_command(
        &repo,
        &["cell", "--cell-id", "audit-log", "--mode", "prove"],
    );
    validation::validate_value(&repo, ArtifactSchema::CellManifest, &prove["manifest"]).unwrap();
    assert!(!prove["certification_evidence"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!prove["proof_commands"].as_array().unwrap().is_empty());

    let (rbac_prove, _rbac_md) =
        run_command(&repo, &["cell", "--cell-id", "rbac", "--mode", "prove"]);
    validation::validate_value(&repo, ArtifactSchema::CellManifest, &rbac_prove["manifest"])
        .unwrap();
    assert_eq!(rbac_prove["manifest"]["cell_id"], "rbac");
    assert_eq!(rbac_prove["manifest"]["certification_status"], "certified");

    // Auth-session cell: fourth certified cell with dependency-bound evidence
    let auth_session = cells
        .iter()
        .find(|cell| cell["cell_id"] == "auth-session")
        .expect("auth-session cell");
    assert_eq!(auth_session["certification_status"], "certified");
    assert_eq!(auth_session["lifecycle"], "certified");
    assert_eq!(auth_session["category"], "identity");
    assert!(auth_session["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d == "audit-log"));
    assert!(auth_session["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d == "rbac"));
    assert!(auth_session["certification_evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["kind"] == "dependency" && e["path"] == "rbac" && e["status"] == "present"));

    let (auth_session_prove, _auth_session_md) = run_command(
        &repo,
        &["cell", "--cell-id", "auth-session", "--mode", "prove"],
    );
    validation::validate_value(
        &repo,
        ArtifactSchema::CellManifest,
        &auth_session_prove["manifest"],
    )
    .unwrap();
    assert_eq!(auth_session_prove["manifest"]["cell_id"], "auth-session");
    assert_eq!(
        auth_session_prove["manifest"]["certification_status"],
        "certified"
    );

    // Background-job cell: sixth certified cell with retry policy marker evidence.
    let background_job = cells
        .iter()
        .find(|cell| cell["cell_id"] == "background-job")
        .expect("background-job cell");
    assert_eq!(background_job["certification_status"], "certified");
    assert_eq!(background_job["lifecycle"], "certified");
    assert_eq!(background_job["category"], "workflow");
    assert!(background_job["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d == "organization-team"));
    assert!(background_job["certification_evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| {
            e["kind"] == "content-marker"
                && e["path"] == "domain-background-job-retry-policy"
                && e["status"] == "present"
        }));
}

#[test]
fn update_subcommand_is_not_confused_with_repo_positional() {
    let repo = tempdir().unwrap();
    let status = Command::new(binary_path())
        .arg("update")
        .arg(repo.path())
        .arg("--offline")
        .arg("--quiet")
        .status()
        .unwrap();
    assert!(status.success());
    assert!(repo
        .path()
        .join("target/jankurai/update/update-plan.json")
        .exists());
    assert!(repo
        .path()
        .join("target/jankurai/update/update-plan.md")
        .exists());
    assert!(repo
        .path()
        .join("target/jankurai/update/state.json")
        .exists());
}

#[test]
fn update_self_flag_alias_is_accepted() {
    let repo = tempdir().unwrap();
    let status = Command::new(binary_path())
        .arg("update")
        .arg(repo.path())
        .arg("--self")
        .arg("--offline")
        .arg("--quiet")
        .status()
        .unwrap();
    assert!(status.success());
    assert!(repo
        .path()
        .join("target/jankurai/update/update-plan.json")
        .exists());
}

#[test]
fn update_self_update_alias_is_accepted() {
    let repo = tempdir().unwrap();
    let status = Command::new(binary_path())
        .arg("update")
        .arg(repo.path())
        .arg("--self-update")
        .arg("--offline")
        .arg("--quiet")
        .status()
        .unwrap();
    assert!(status.success());
    assert!(repo
        .path()
        .join("target/jankurai/update/update-plan.json")
        .exists());
}

#[test]
fn update_auto_prefers_newer_local_source_checkout() {
    let repo = tempdir().unwrap();
    fs::write(
        repo.path().join("AGENTS.md"),
        "Read `agent/JANKURAI_STANDARD.md` first.\n",
    )
    .unwrap();
    fs::create_dir_all(repo.path().join("crates/jankurai")).unwrap();
    fs::write(
        repo.path().join("crates/jankurai/Cargo.toml"),
        "[package]\nname = \"jankurai\"\nversion = \"999.0.0\"\n",
    )
    .unwrap();

    let plan_path = repo.path().join("target/jankurai/update/update-plan.json");
    let status = Command::new(binary_path())
        .arg("update")
        .arg(repo.path())
        .arg("--source")
        .arg("auto")
        .arg("--quiet")
        .status()
        .unwrap();
    assert!(status.success());

    let plan: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(plan_path).unwrap()).unwrap();
    assert_eq!(plan["current_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(plan["latest_version"], "999.0.0");
    assert_eq!(plan["self_update_available"], true);
}

#[test]
fn upgrade_offline_writes_update_artifacts_and_receipt() {
    let repo = tempdir().unwrap();
    let status = Command::new(binary_path())
        .arg("upgrade")
        .arg(repo.path())
        .arg("--offline")
        .arg("--quiet")
        .status()
        .unwrap();
    assert!(status.success());
    let plan_path = repo.path().join("target/jankurai/update/update-plan.json");
    assert!(plan_path.exists());
    assert!(repo
        .path()
        .join("target/jankurai/update/state.json")
        .exists());
    let plan: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::UpdatePlan, &plan).unwrap();
    assert_eq!(plan["command"], "jankurai update");
    assert_eq!(plan["current_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(plan["standard_version"], STANDARD_VERSION);
    assert_eq!(plan["auditor_version"], AUDITOR_VERSION);
    assert_eq!(plan["schema_contract_version"], SCHEMA_VERSION);
    assert_eq!(plan["paper_edition"], PAPER_EDITION);
    assert_eq!(plan["target_stack_id"], TARGET_STACK_ID);
    assert!(plan.get("latest_version").is_none() || plan["latest_version"].is_string());
    if let Some(resolved_source) = plan.get("resolved_source") {
        assert!(resolved_source.is_object());
        assert!(resolved_source.get("requested_source").is_some());
        assert!(resolved_source.get("resolved_source").is_some());
        assert!(resolved_source.get("reason").is_some());
    }
    assert!(plan.get("reexec_command").is_none() || plan["reexec_command"].is_string());
    assert!(
        plan.get("post_upgrade_score_command").is_none()
            || plan["post_upgrade_score_command"].is_string()
    );
    assert!(
        plan.get("post_upgrade_score_mode").is_none()
            || plan["post_upgrade_score_mode"].is_string()
    );
    assert!(
        plan.get("post_upgrade_score_json").is_none()
            || plan["post_upgrade_score_json"].is_string()
    );
    assert!(
        plan.get("post_upgrade_score_md").is_none() || plan["post_upgrade_score_md"].is_string()
    );
    assert!(plan.get("warnings").is_none() || plan["warnings"].is_array());
    assert!(plan.get("actions").is_none() || plan["actions"].is_array());
    assert!(plan.get("artifacts").is_none() || plan["artifacts"].is_array());

    let receipt_dir = repo.path().join("target/jankurai/receipts");
    let receipt_path = fs::read_dir(&receipt_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .next()
        .expect("update receipt");
    let receipt_count = fs::read_dir(receipt_dir).unwrap().count();
    assert_eq!(receipt_count, 1);
    let receipt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&receipt_path).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::UpdateReceipt, &receipt).unwrap();
    assert_eq!(receipt["command"], "jankurai update");
    assert_eq!(receipt["current_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(receipt["update_channel"], "stable");
    assert_eq!(receipt["source"], "auto");
    assert_eq!(receipt["self_update_requested"], true);
    assert_eq!(receipt["self_update_applied"], false);
    assert_eq!(receipt["repo_update_applied"], false);
    assert!(receipt.get("latest_version").is_none() || receipt["latest_version"].is_string());
    if let Some(resolved_source) = receipt.get("resolved_source") {
        assert!(resolved_source.is_object());
        assert!(resolved_source.get("requested_source").is_some());
        assert!(resolved_source.get("resolved_source").is_some());
        assert!(resolved_source.get("reason").is_some());
    }
    assert!(receipt.get("reexec_command").is_none() || receipt["reexec_command"].is_string());
    assert!(
        receipt.get("post_upgrade_score_command").is_none()
            || receipt["post_upgrade_score_command"].is_string()
    );
    assert!(
        receipt.get("post_upgrade_score_mode").is_none()
            || receipt["post_upgrade_score_mode"].is_string()
    );
    assert!(
        receipt.get("post_upgrade_score_json").is_none()
            || receipt["post_upgrade_score_json"].is_string()
    );
    assert!(
        receipt.get("post_upgrade_score_md").is_none()
            || receipt["post_upgrade_score_md"].is_string()
    );
    assert!(receipt.get("actions").is_none() || receipt["actions"].is_array());
    assert!(receipt.get("commands_run").is_none() || receipt["commands_run"].is_array());
    assert!(receipt.get("next_command").is_none() || receipt["next_command"].is_string());
    assert!(receipt.get("residual_risk").is_none() || receipt["residual_risk"].is_array());
    assert!(receipt.get("artifacts").is_none() || receipt["artifacts"].is_array());
}
