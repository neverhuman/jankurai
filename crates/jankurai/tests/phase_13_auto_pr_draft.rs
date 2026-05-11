use jankurai::validation::{self, ArtifactSchema};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn seed_repo(repo: &Path) {
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
name = "audit"
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
}

fn write_plan(
    repo: &Path,
    packets: Vec<serde_json::Value>,
    edits: Vec<serde_json::Value>,
) -> PathBuf {
    fs::create_dir_all(repo.join("target/jankurai")).unwrap();
    let path = repo.join("target/jankurai/repair-plan.json");
    let plan = json!({
        "schema_version": "1.0.0",
        "source_report": "target/jankurai/repo-score.json",
        "generated_at": "0",
        "target_stack_id": "jankurai:v0.4",
        "plan_mode": "dry-run",
        "planned_edits": edits,
        "planned_commands": ["just fast"],
        "proof_lanes": ["audit"],
        "rollback_guidance": ["restore the original file"],
        "human_approval_requirements": [],
        "packets": packets
    });
    fs::write(&path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();
    path
}

fn packet(
    path: &str,
    fingerprint: &str,
    rule_id: &str,
    severity: &str,
    repair_eligibility: &str,
    risk_level: &str,
    human_review_required: bool,
) -> serde_json::Value {
    json!({
        "finding_fingerprint": fingerprint,
        "finding_path": path,
        "rule_id": rule_id,
        "check_id": rule_id,
        "severity": severity,
        "owner": "standard",
        "lane": "audit",
        "problem": "fixture problem",
        "why": "fixture reason",
        "permission_profile": "docs-only",
        "allowed_paths": ["docs/"],
        "forbidden_paths": ["reference/", "paper/", "target/"],
        "expected_patch_shape": "fixture patch",
        "required_proof": ["true"],
        "stop_conditions": ["stop"],
        "repair_eligibility": repair_eligibility,
        "risk_level": risk_level,
        "eligibility_reason": "fixture repair is scoped to docs",
        "human_review_required": human_review_required,
        "rollback_guidance": "restore the file"
    })
}

fn edit(
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

fn run_repair(
    repo: &Path,
    plan_path: &Path,
    run_name: &str,
    draft_name: Option<&str>,
    extra_args: &[&str],
) -> (Output, PathBuf, Option<PathBuf>) {
    let run_path = repo.join(run_name);
    let run_md = run_path.with_extension("md");
    let draft_path = draft_name.map(|name| repo.join(name));
    let draft_md = draft_path.as_ref().map(|path| path.with_extension("md"));
    let mut cmd = Command::new(binary_path());
    cmd.arg("repair")
        .arg(repo)
        .arg("--plan")
        .arg(plan_path)
        .args(extra_args)
        .arg("--out")
        .arg(&run_path)
        .arg("--md")
        .arg(&run_md);
    if let Some(path) = &draft_path {
        cmd.arg("--pr-draft-out").arg(path);
    }
    if let Some(path) = &draft_md {
        cmd.arg("--pr-draft-md").arg(path);
    }
    let output = cmd.output().unwrap();
    (output, run_path, draft_path)
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn auto_pr_draft_requires_auto_pr_flag() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    let plan_path = write_plan(
        repo.path(),
        vec![packet(
            "docs/notes.md",
            "sha256:eligible",
            "HLT-017-OPAQUE-OBSERVABILITY",
            "medium",
            "agent-assisted",
            "medium",
            false,
        )],
        vec![edit(
            "docs/notes.md",
            "sha256:eligible",
            "append-text",
            json!({"append_text": "beta\n"}),
        )],
    );

    let output = Command::new(binary_path())
        .arg("repair")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--pr-draft-out")
        .arg(repo.path().join("target/jankurai/repair-pr-draft.json"))
        .arg("--out")
        .arg(repo.path().join("target/jankurai/repair-run.json"))
        .arg("--md")
        .arg(repo.path().join("target/jankurai/repair-run.md"))
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("require `--auto-pr`"), "{stderr}");
}

#[test]
fn auto_pr_draft_emits_schema_valid_artifact_for_eligible_dry_run() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(
        repo.path(),
        vec![packet(
            "docs/notes.md",
            "sha256:eligible",
            "HLT-017-OPAQUE-OBSERVABILITY",
            "medium",
            "agent-assisted",
            "medium",
            false,
        )],
        vec![edit(
            "docs/notes.md",
            "sha256:eligible",
            "append-text",
            json!({"append_text": "beta\n"}),
        )],
    );

    let (output, run_path, draft_path) = run_repair(
        repo.path(),
        &plan_path,
        "target/jankurai/repair-run.json",
        Some("target/jankurai/repair-pr-draft.json"),
        &["--dry-run", "--auto-pr", "--max-risk", "medium"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = read_json(&run_path);
    let draft = read_json(draft_path.as_ref().unwrap());
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairPrDraft, &draft).unwrap();

    assert_eq!(run["execution_mode"], "dry-run");
    assert_eq!(run["auto_pr_status"], "eligible-dry-run-only");
    assert!(run["auto_pr_draft"].is_object());
    assert_eq!(run["auto_pr_draft"]["status"], "draft-only");
    assert_eq!(draft["status"], "draft-only");
    assert_eq!(draft["execution_mode"], "dry-run");
    assert_eq!(draft["git_mutation_allowed"], false);
    assert_eq!(draft["github_mutation_allowed"], false);
    assert_eq!(draft["planned_changed_paths"][0], "docs/notes.md");
    assert!(draft["eligible_packets"].as_array().unwrap().len() == 1);
    assert!(draft["blocked_packets"].as_array().unwrap().is_empty());
    assert!(draft["branch_name"]
        .as_str()
        .unwrap()
        .starts_with("jankurai/repair/"));
    assert!(draft["pr_body"]
        .as_str()
        .unwrap()
        .contains("Eligible Packets"));
}

#[test]
fn auto_pr_draft_blocks_high_risk_packet() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    let plan_path = write_plan(
        repo.path(),
        vec![packet(
            "docs/notes.md",
            "sha256:high",
            "HLT-011-PROMPT-INJECTION",
            "high",
            "human-required",
            "high",
            true,
        )],
        vec![edit(
            "docs/notes.md",
            "sha256:high",
            "review-only",
            json!({}),
        )],
    );

    let (output, run_path, draft_path) = run_repair(
        repo.path(),
        &plan_path,
        "target/jankurai/repair-run.json",
        Some("target/jankurai/repair-pr-draft.json"),
        &["--dry-run", "--auto-pr", "--max-risk", "low"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = read_json(&run_path);
    let draft = read_json(draft_path.as_ref().unwrap());
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairPrDraft, &draft).unwrap();

    assert_eq!(run["auto_pr_status"], "blocked");
    assert_eq!(draft["status"], "blocked");
    assert_eq!(draft["blocked_packets"].as_array().unwrap().len(), 1);
    assert!(draft["blocked_packets"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("risk high exceeds max low"));
}

#[test]
fn auto_pr_draft_blocks_secret_and_prompt_injection_packets() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    let plan_path = write_plan(
        repo.path(),
        vec![
            packet(
                "docs/secret.md",
                "sha256:secret",
                "HLT-010-SECRET-SPRAWL",
                "critical",
                "never-auto",
                "critical",
                true,
            ),
            packet(
                "docs/prompt.md",
                "sha256:prompt",
                "HLT-011-PROMPT-INJECTION",
                "high",
                "human-required",
                "high",
                true,
            ),
        ],
        vec![
            edit(
                "docs/secret.md",
                "sha256:secret",
                "replace-exact",
                json!({"match_text": "secret", "replacement_text": "redacted"}),
            ),
            edit("docs/prompt.md", "sha256:prompt", "review-only", json!({})),
        ],
    );

    let (output, run_path, draft_path) = run_repair(
        repo.path(),
        &plan_path,
        "target/jankurai/repair-run.json",
        Some("target/jankurai/repair-pr-draft.json"),
        &["--dry-run", "--auto-pr", "--max-risk", "critical"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = read_json(&run_path);
    let draft = read_json(draft_path.as_ref().unwrap());
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairPrDraft, &draft).unwrap();

    assert_eq!(draft["status"], "blocked");
    assert!(draft["blocked_packets"].as_array().unwrap().len() >= 2);
    let blocked_reason = draft["blocked_packets"][0]["reason"].as_str().unwrap();
    assert!(
        blocked_reason.contains("repair eligibility is never-auto")
            || blocked_reason.contains("risk critical exceeds max critical")
    );
}

#[test]
fn auto_pr_draft_includes_proof_lanes_and_artifact_links() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(
        repo.path(),
        vec![packet(
            "docs/notes.md",
            "sha256:proof",
            "HLT-017-OPAQUE-OBSERVABILITY",
            "medium",
            "agent-assisted",
            "medium",
            false,
        )],
        vec![edit(
            "docs/notes.md",
            "sha256:proof",
            "append-text",
            json!({"append_text": "beta\n"}),
        )],
    );

    let (output, run_path, draft_path) = run_repair(
        repo.path(),
        &plan_path,
        "target/jankurai/repair-run.json",
        Some("target/jankurai/repair-pr-draft.json"),
        &["--dry-run", "--auto-pr", "--max-risk", "medium"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = read_json(&run_path);
    let draft = read_json(draft_path.as_ref().unwrap());
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairPrDraft, &draft).unwrap();
    let run_link = run_path.display().to_string();
    let draft_link = draft_path.as_ref().unwrap().display().to_string();

    assert!(run["auto_pr_draft"]["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "audit"));
    assert!(draft["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "audit"));
    assert!(draft["artifact_links"]
        .as_array()
        .unwrap()
        .iter()
        .any(|link| link == "target/jankurai/repo-score.json"));
    assert!(draft["artifact_links"]
        .as_array()
        .unwrap()
        .iter()
        .any(|link| link.as_str() == Some(run_link.as_str())));
    assert!(draft["artifact_links"]
        .as_array()
        .unwrap()
        .iter()
        .any(|link| link.as_str() == Some(draft_link.as_str())));
}

#[test]
fn auto_pr_draft_branch_name_is_deterministic_and_sanitized() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(
        repo.path(),
        vec![packet(
            "docs/notes.md",
            "sha256:stable",
            "HLT-017-OPAQUE-OBSERVABILITY",
            "medium",
            "agent-assisted",
            "medium",
            false,
        )],
        vec![edit(
            "docs/notes.md",
            "sha256:stable",
            "append-text",
            json!({"append_text": "beta\n"}),
        )],
    );

    let (first_output, first_run_path, first_draft_path) = run_repair(
        repo.path(),
        &plan_path,
        "target/jankurai/repair-run-1.json",
        Some("target/jankurai/repair-pr-draft-1.json"),
        &["--dry-run", "--auto-pr", "--max-risk", "medium"],
    );
    assert!(
        first_output.status.success(),
        "{}",
        String::from_utf8_lossy(&first_output.stderr)
    );
    let (second_output, second_run_path, second_draft_path) = run_repair(
        repo.path(),
        &plan_path,
        "target/jankurai/repair-run-2.json",
        Some("target/jankurai/repair-pr-draft-2.json"),
        &["--dry-run", "--auto-pr", "--max-risk", "medium"],
    );
    assert!(
        second_output.status.success(),
        "{}",
        String::from_utf8_lossy(&second_output.stderr)
    );

    let first_draft = read_json(first_draft_path.as_ref().unwrap());
    let second_draft = read_json(second_draft_path.as_ref().unwrap());
    let first_run = read_json(&first_run_path);
    let second_run = read_json(&second_run_path);
    validation::validate_value(repo.path(), ArtifactSchema::RepairPrDraft, &first_draft).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairPrDraft, &second_draft).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &first_run).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &second_run).unwrap();

    let first_branch = first_draft["branch_name"].as_str().unwrap();
    let second_branch = second_draft["branch_name"].as_str().unwrap();
    assert_eq!(first_branch, second_branch);
    assert!(first_branch.starts_with("jankurai/repair/"));
    assert!(!first_branch.contains(' '));
    assert!(!first_branch.contains(".."));
}

#[test]
fn auto_pr_draft_does_not_create_git_directory() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    let plan_path = write_plan(
        repo.path(),
        vec![packet(
            "docs/notes.md",
            "sha256:nogit",
            "HLT-017-OPAQUE-OBSERVABILITY",
            "medium",
            "agent-assisted",
            "medium",
            false,
        )],
        vec![edit(
            "docs/notes.md",
            "sha256:nogit",
            "append-text",
            json!({"append_text": "beta\n"}),
        )],
    );

    let (output, _, _) = run_repair(
        repo.path(),
        &plan_path,
        "target/jankurai/repair-run.json",
        Some("target/jankurai/repair-pr-draft.json"),
        &["--dry-run", "--auto-pr", "--max-risk", "medium"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!repo.path().join(".git").exists());
}

#[test]
fn auto_pr_draft_does_not_change_existing_git_branch() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    let status = Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
        .arg(repo.path())
        .status()
        .unwrap();
    assert!(status.success());
    let branch_before = Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .arg("symbolic-ref")
        .arg("--short")
        .arg("HEAD")
        .output()
        .unwrap();
    assert!(branch_before.status.success());
    let branch_before = String::from_utf8_lossy(&branch_before.stdout)
        .trim()
        .to_string();
    let plan_path = write_plan(
        repo.path(),
        vec![packet(
            "docs/notes.md",
            "sha256:git",
            "HLT-017-OPAQUE-OBSERVABILITY",
            "medium",
            "agent-assisted",
            "medium",
            false,
        )],
        vec![edit(
            "docs/notes.md",
            "sha256:git",
            "append-text",
            json!({"append_text": "beta\n"}),
        )],
    );

    let (output, _, _) = run_repair(
        repo.path(),
        &plan_path,
        "target/jankurai/repair-run.json",
        Some("target/jankurai/repair-pr-draft.json"),
        &["--dry-run", "--auto-pr", "--max-risk", "medium"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let branch_after = Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .arg("symbolic-ref")
        .arg("--short")
        .arg("HEAD")
        .output()
        .unwrap();
    assert!(branch_after.status.success());
    let branch_after = String::from_utf8_lossy(&branch_after.stdout)
        .trim()
        .to_string();
    assert_eq!(branch_before, branch_after);
}

#[test]
fn auto_pr_draft_does_not_execute_plan_commands() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(
        repo.path(),
        vec![packet(
            "docs/notes.md",
            "sha256:command",
            "HLT-017-OPAQUE-OBSERVABILITY",
            "medium",
            "agent-assisted",
            "medium",
            false,
        )],
        vec![edit(
            "docs/notes.md",
            "sha256:command",
            "append-text",
            json!({"append_text": "beta\n"}),
        )],
    );
    let mut plan: serde_json::Value = read_json(&plan_path);
    plan["planned_commands"] = json!(["false"]);
    fs::write(&plan_path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();

    let (output, run_path, draft_path) = run_repair(
        repo.path(),
        &plan_path,
        "target/jankurai/repair-run.json",
        Some("target/jankurai/repair-pr-draft.json"),
        &["--dry-run", "--auto-pr", "--max-risk", "medium"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = read_json(&run_path);
    let draft = read_json(draft_path.as_ref().unwrap());
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairPrDraft, &draft).unwrap();
    assert!(run["auto_pr_draft"].is_object());
    assert_eq!(draft["status"], "draft-only");
}
