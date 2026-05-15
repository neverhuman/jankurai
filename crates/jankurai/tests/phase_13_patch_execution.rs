use jankurai::validation::{self, ArtifactSchema};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn seed_fixture_repo(repo: &Path) {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/owner-map.json"),
        r#"{"workspace":"fixture","owners":{"agent/":"agent","docs/":"standard","paper/":"paper","reference/":"read-only","target/":"workspace","crates/":"tools"}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"docs/":{"command":"true","purpose":"fixture docs proof"},"agent/":{"command":"true","purpose":"fixture agent proof"}}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/proof-lanes.toml"),
        r#"[[lane]]
name = "fixture"
command = "true"
purpose = "fixture proof"
"#,
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
    fs::write(repo.join("agent/repair-fixture.toml"), "fixture = true\n").unwrap();
}

fn write_plan(repo: &Path, edit: serde_json::Value, packet: serde_json::Value) -> PathBuf {
    fs::create_dir_all(repo.join("target/jankurai")).unwrap();
    let path = repo.join("target/jankurai/repair-plan.json");
    let plan = json!({
        "schema_version": "1.0.0",
        "source_report": "target/jankurai/repo-score.json",
        "generated_at": "0",
        "target_stack_id": "jankurai:v0.4",
        "plan_mode": "dry-run",
        "planned_edits": [edit],
        "planned_commands": ["true"],
        "proof_lanes": ["fixture"],
        "rollback_guidance": ["restore the original file"],
        "human_approval_requirements": [],
        "packets": [packet]
    });
    fs::write(&path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();
    path
}

fn run_repair(
    repo: &Path,
    plan_path: &Path,
    extra_args: &[&str],
    out_name: &str,
) -> (Output, PathBuf) {
    let out_path = repo.join(out_name);
    let md_path = out_path.with_extension("md");
    let output = Command::new(binary_path())
        .arg("repair")
        .arg(repo)
        .arg("--plan")
        .arg(plan_path)
        .args(extra_args)
        .arg("--out")
        .arg(&out_path)
        .arg("--md")
        .arg(&md_path)
        .output()
        .unwrap();
    (output, out_path)
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn allowed_packet(
    path: &str,
    fingerprint: &str,
    allowed_paths: &[&str],
    patch_fields: serde_json::Value,
) -> serde_json::Value {
    let mut packet = serde_json::Map::new();
    packet.insert("finding_fingerprint".to_string(), json!(fingerprint));
    packet.insert("finding_path".to_string(), json!(path));
    packet.insert("rule_id".to_string(), json!("HLT-017-OPAQUE-OBSERVABILITY"));
    packet.insert(
        "check_id".to_string(),
        json!("HLT-017-OPAQUE-OBSERVABILITY"),
    );
    packet.insert("severity".to_string(), json!("medium"));
    packet.insert("owner".to_string(), json!("standard"));
    packet.insert("lane".to_string(), json!("audit"));
    packet.insert("problem".to_string(), json!("fixture problem"));
    packet.insert("why".to_string(), json!("fixture reason"));
    packet.insert("permission_profile".to_string(), json!("docs-only"));
    packet.insert("allowed_paths".to_string(), json!(allowed_paths));
    packet.insert("forbidden_paths".to_string(), json!(["reference/"]));
    packet.insert("expected_patch_shape".to_string(), json!("fixture patch"));
    packet.insert("required_proof".to_string(), json!(["true"]));
    packet.insert("stop_conditions".to_string(), json!(["stop"]));
    packet.insert("repair_eligibility".to_string(), json!("agent-assisted"));
    packet.insert("risk_level".to_string(), json!("medium"));
    packet.insert(
        "eligibility_reason".to_string(),
        json!("fixture repair is scoped to a docs-only patch"),
    );
    packet.insert("human_review_required".to_string(), json!(false));
    packet.insert("rollback_guidance".to_string(), json!("restore the file"));
    packet.extend(patch_fields.as_object().unwrap().clone());
    serde_json::Value::Object(packet)
}

fn fixture_edit(
    path: &str,
    fingerprint: &str,
    strategy: &str,
    patch_fields: serde_json::Value,
) -> serde_json::Value {
    let mut edit = serde_json::Map::new();
    edit.insert("path".to_string(), json!(path));
    edit.insert("operation".to_string(), json!("modify"));
    edit.insert("reason".to_string(), json!("fixture repair"));
    edit.insert("finding_fingerprint".to_string(), json!(fingerprint));
    edit.insert("rule_id".to_string(), json!("HLT-017-OPAQUE-OBSERVABILITY"));
    edit.insert("apply_strategy".to_string(), json!(strategy));
    edit.insert("risk_level".to_string(), json!("medium"));
    edit.insert("repair_eligibility".to_string(), json!("agent-assisted"));
    edit.extend(patch_fields.as_object().unwrap().clone());
    serde_json::Value::Object(edit)
}

#[test]
fn non_dry_run_without_fixture_apply_still_bails() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    let plan_path = write_plan(
        repo.path(),
        fixture_edit(
            "docs/notes.md",
            "sha256:non-dry-run",
            "append-text",
            json!({"append_text": "ignored\n"}),
        ),
        allowed_packet(
            "docs/notes.md",
            "sha256:non-dry-run",
            &["docs/"],
            json!({"append_text": "ignored\n"}),
        ),
    );

    let output = Command::new(binary_path())
        .arg("repair")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("dry-run only unless `--fixture-apply`"),
        "{stderr}"
    );
}

#[test]
fn fixture_apply_requires_marker() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    fs::remove_file(repo.path().join("agent/repair-fixture.toml")).unwrap();
    let plan_path = write_plan(
        repo.path(),
        fixture_edit(
            "docs/notes.md",
            "sha256:marker",
            "append-text",
            json!({"append_text": "ignored\n"}),
        ),
        allowed_packet(
            "docs/notes.md",
            "sha256:marker",
            &["docs/"],
            json!({"append_text": "ignored\n"}),
        ),
    );

    let output = Command::new(binary_path())
        .arg("repair")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--fixture-apply")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("repair-fixture.toml"),
        "expected fixture marker failure, got {stderr}"
    );
}

#[test]
fn fixture_apply_rejects_auto_pr() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    let plan_path = write_plan(
        repo.path(),
        fixture_edit(
            "docs/notes.md",
            "sha256:auto-pr",
            "append-text",
            json!({"append_text": "ignored\n"}),
        ),
        allowed_packet(
            "docs/notes.md",
            "sha256:auto-pr",
            &["docs/"],
            json!({"append_text": "ignored\n"}),
        ),
    );

    let output = Command::new(binary_path())
        .arg("repair")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--fixture-apply")
        .arg("--auto-pr")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be combined"), "{stderr}");
}

#[test]
fn fixture_apply_appends_text_inside_allowed_path() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(
        repo.path(),
        fixture_edit(
            "docs/notes.md",
            "sha256:append",
            "append-text",
            json!({"append_text": "beta\n"}),
        ),
        allowed_packet(
            "docs/notes.md",
            "sha256:append",
            &["docs/"],
            json!({"append_text": "beta\n"}),
        ),
    );

    let (output, out_path) = run_repair(
        repo.path(),
        &plan_path,
        &["--fixture-apply", "--max-risk", "medium"],
        "target/jankurai/p13-fixture-repair-run.json",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("docs/notes.md")).unwrap(),
        "alpha\nbeta\n"
    );

    let run = read_json(&out_path);
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    assert_eq!(run["status"], "complete");
    assert_eq!(run["execution_mode"], "fixture-apply");
    assert_eq!(run["applied_edits"].as_array().unwrap().len(), 1);
    assert!(run["skipped_edits"].as_array().unwrap().is_empty());
    assert!(run["files_written"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "docs/notes.md"));
}

#[test]
fn fixture_apply_replaces_exactly_one_match() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(
        repo.path().join("docs/replace.md"),
        "alpha\nneedle\nomega\n",
    )
    .unwrap();
    let plan_path = write_plan(
        repo.path(),
        fixture_edit(
            "docs/replace.md",
            "sha256:replace",
            "replace-exact",
            json!({
                "match_text": "needle",
                "replacement_text": "marker"
            }),
        ),
        allowed_packet(
            "docs/replace.md",
            "sha256:replace",
            &["docs/"],
            json!({
                "match_text": "needle",
                "replacement_text": "marker"
            }),
        ),
    );

    let (output, out_path) = run_repair(
        repo.path(),
        &plan_path,
        &["--fixture-apply", "--max-risk", "medium"],
        "target/jankurai/p13-fixture-repair-run.json",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("docs/replace.md")).unwrap(),
        "alpha\nmarker\nomega\n"
    );

    let run = read_json(&out_path);
    assert_eq!(run["status"], "complete");
    assert_eq!(run["applied_edits"][0]["apply_strategy"], "replace-exact");
    assert_ne!(
        run["applied_edits"][0]["before_sha256"],
        run["applied_edits"][0]["after_sha256"]
    );
}

#[test]
fn fixture_apply_creates_new_file_inside_allowed_path() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    let plan_path = write_plan(
        repo.path(),
        fixture_edit(
            "docs/new-file.md",
            "sha256:create",
            "create-file",
            json!({"create_text": "created\n"}),
        ),
        allowed_packet(
            "docs/new-file.md",
            "sha256:create",
            &["docs/"],
            json!({"create_text": "created\n"}),
        ),
    );

    let (output, out_path) = run_repair(
        repo.path(),
        &plan_path,
        &["--fixture-apply", "--max-risk", "medium"],
        "target/jankurai/p13-fixture-repair-run.json",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("docs/new-file.md")).unwrap(),
        "created\n"
    );

    let run = read_json(&out_path);
    assert_eq!(run["status"], "complete");
    assert_eq!(run["applied_edits"][0]["apply_strategy"], "create-file");
    assert_eq!(run["applied_edits"][0]["before_sha256"], "missing");
}

#[test]
fn fixture_apply_rejects_path_escape() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    let plan_path = write_plan(
        repo.path(),
        fixture_edit(
            "../escape.md",
            "sha256:escape",
            "append-text",
            json!({"append_text": "ignored\n"}),
        ),
        allowed_packet(
            "../escape.md",
            "sha256:escape",
            &["docs/"],
            json!({"append_text": "ignored\n"}),
        ),
    );

    let (output, out_path) = run_repair(
        repo.path(),
        &plan_path,
        &["--fixture-apply", "--max-risk", "medium"],
        "target/jankurai/p13-fixture-repair-run.json",
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("path traversal"), "{stderr}");

    let run = read_json(&out_path);
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    assert_eq!(run["status"], "failed");
    assert!(run["applied_edits"].as_array().unwrap().is_empty());
    assert!(run["skipped_edits"].as_array().unwrap().is_empty());
}

#[test]
fn fixture_apply_rejects_generated_zone() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    fs::write(repo.path().join("agent/repo-score.json"), "{\"score\":0}\n").unwrap();
    let plan_path = write_plan(
        repo.path(),
        fixture_edit(
            "agent/repo-score.json",
            "sha256:generated",
            "append-text",
            json!({"append_text": "ignored\n"}),
        ),
        allowed_packet(
            "agent/repo-score.json",
            "sha256:generated",
            &["agent/"],
            json!({"append_text": "ignored\n"}),
        ),
    );

    let (output, out_path) = run_repair(
        repo.path(),
        &plan_path,
        &["--fixture-apply", "--max-risk", "medium"],
        "target/jankurai/p13-fixture-repair-run.json",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = read_json(&out_path);
    assert_eq!(run["status"], "blocked");
    assert_eq!(run["skipped_edits"].as_array().unwrap().len(), 1);
    assert!(run["skipped_edits"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("generated zone"));
    assert!(run["proof_evidence_index"].is_null());
}

#[test]
fn fixture_apply_rejects_high_risk_or_human_required_packet() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/high-risk.md"), "alpha\n").unwrap();
    let mut packet = allowed_packet(
        "docs/high-risk.md",
        "sha256:high-risk",
        &["docs/"],
        json!({"append_text": "beta\n"}),
    );
    packet["risk_level"] = json!("high");
    packet["repair_eligibility"] = json!("agent-assisted");
    packet["human_review_required"] = json!(true);
    let mut edit = fixture_edit(
        "docs/high-risk.md",
        "sha256:high-risk",
        "append-text",
        json!({"append_text": "beta\n"}),
    );
    edit["risk_level"] = json!("high");
    edit["repair_eligibility"] = json!("agent-assisted");
    let plan_path = write_plan(repo.path(), edit, packet);

    let (output, out_path) = run_repair(
        repo.path(),
        &plan_path,
        &["--fixture-apply", "--max-risk", "medium"],
        "target/jankurai/p13-fixture-repair-run.json",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("docs/high-risk.md")).unwrap(),
        "alpha\n"
    );

    let run = read_json(&out_path);
    assert_eq!(run["status"], "blocked");
    assert_eq!(run["blocked_packets"].as_array().unwrap().len(), 1);
    assert!(run["blocked_packets"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("risk high exceeds max medium"));
    assert_eq!(run["skipped_edits"].as_array().unwrap().len(), 1);
}

#[test]
fn fixture_apply_runs_allowed_fixture_proof_and_records_receipt() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/proof.md"), "alpha\n").unwrap();
    let plan_path = write_plan(
        repo.path(),
        fixture_edit(
            "docs/proof.md",
            "sha256:proof",
            "append-text",
            json!({"append_text": "beta\n"}),
        ),
        allowed_packet(
            "docs/proof.md",
            "sha256:proof",
            &["docs/"],
            json!({"append_text": "beta\n"}),
        ),
    );

    let (output, out_path) = run_repair(
        repo.path(),
        &plan_path,
        &["--fixture-apply", "--max-risk", "medium"],
        "target/jankurai/p13-fixture-repair-run.json",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = read_json(&out_path);
    assert_eq!(
        run["proof_evidence_index"],
        "target/jankurai/p13-fixture-evidence-index.json"
    );
    let evidence_path = repo
        .path()
        .join("target/jankurai/p13-fixture-evidence-index.json");
    let evidence = read_json(&evidence_path);
    assert_eq!(evidence["schema_version"], "1.2.0");
    validation::validate_value(repo.path(), ArtifactSchema::EvidenceIndex, &evidence).unwrap();

    let receipts_dir = repo
        .path()
        .join("target/jankurai/p13-fixture-proof-receipts");
    let receipts: Vec<_> = fs::read_dir(&receipts_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert!(!receipts.is_empty());
    let receipt = read_json(&receipts[0]);
    validation::validate_value(repo.path(), ArtifactSchema::ProofReceipt, &receipt).unwrap();
    assert_eq!(receipt["lane"], "fixture");
    assert_eq!(receipt["exit_code"], 0);
}

#[test]
fn fixture_apply_writes_schema_valid_repair_run() {
    let repo = tempdir().unwrap();
    seed_fixture_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/schema.md"), "alpha\n").unwrap();
    let plan_path = write_plan(
        repo.path(),
        fixture_edit(
            "docs/schema.md",
            "sha256:schema",
            "append-text",
            json!({"append_text": "beta\n"}),
        ),
        allowed_packet(
            "docs/schema.md",
            "sha256:schema",
            &["docs/"],
            json!({"append_text": "beta\n"}),
        ),
    );

    let (output, out_path) = run_repair(
        repo.path(),
        &plan_path,
        &["--fixture-apply", "--max-risk", "medium"],
        "target/jankurai/p13-fixture-repair-run.json",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = read_json(&out_path);
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    assert_eq!(run["execution_mode"], "fixture-apply");
    assert_eq!(run["status"], "complete");
    assert_eq!(run["applied_edits"].as_array().unwrap().len(), 1);
    assert!(run["skipped_edits"].as_array().unwrap().is_empty());
    assert!(run["files_written"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "docs/schema.md"));
}
