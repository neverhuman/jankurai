use jankurai::validation::{self, ArtifactSchema};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

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

#[test]
fn background_job_is_sixth_certified_dependency_bound_cell() {
    let repo = repo_root();

    let (registry, _registry_md) = run_command(&repo, &["registry"]);
    validation::validate_value(&repo, ArtifactSchema::CellRegistry, &registry).unwrap();

    let cells = registry["cells"].as_array().unwrap();
    assert!(
        cells.len() >= 6,
        "expected at least six built-in cells, got {}",
        cells.len()
    );

    let background_job = cells
        .iter()
        .find(|cell| cell["cell_id"] == "background-job")
        .expect("background-job cell must be present in registry");

    assert_eq!(background_job["lifecycle"], "certified");
    assert_eq!(background_job["certification_status"], "certified");
    assert_eq!(background_job["category"], "workflow");

    let dependencies = background_job["dependencies"].as_array().unwrap();
    assert!(dependencies.iter().any(|d| d == "audit-log"));
    assert!(dependencies.iter().any(|d| d == "rbac"));
    assert!(dependencies.iter().any(|d| d == "auth-session"));
    assert!(dependencies.iter().any(|d| d == "organization-team"));

    let evidence = background_job["certification_evidence"].as_array().unwrap();
    assert!(evidence.iter().any(|item| {
        item["kind"] == "dependency"
            && item["path"] == "organization-team"
            && item["status"] == "present"
    }));
    assert!(evidence.iter().any(|item| {
        item["kind"] == "content-marker"
            && item["path"]
                .as_str()
                .unwrap()
                .contains("domain-background-job-retry-policy")
            && item["status"] == "present"
    }));

    assert!(background_job["source_paths"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "examples/perfect-web-api-db/backend/src/background_job.rs"));
    assert!(background_job["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "security"));
    assert!(background_job["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "db-migration-analyze"));
    assert!(background_job["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "ux-qa"));
}

#[test]
fn background_job_prove_emits_retry_policy_bound_decision() {
    let repo = repo_root();

    let (prove, prove_md) = run_command(
        &repo,
        &["cell", "--cell-id", "background-job", "--mode", "prove"],
    );
    validation::validate_value(&repo, ArtifactSchema::CellManifest, &prove["manifest"]).unwrap();

    assert_eq!(prove["mode"], "prove");
    assert_eq!(prove["lifecycle_action"], "prove-certification");
    assert_eq!(prove["certification_decision"]["status"], "certified");
    assert_eq!(prove["certification_decision"]["merge_ready"], true);
    assert_eq!(
        prove["certification_decision"]["dependency_satisfied"],
        true
    );
    assert!(!prove["certification_evidence"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!prove["proof_commands"].as_array().unwrap().is_empty());
    assert!(prove_md.contains("Dependency Closure"));
    assert!(prove_md.contains("Certification Decision"));
}

#[test]
fn background_job_lifecycle_plans_emit_queue_safety_notes() {
    let repo = repo_root();

    let (upgrade, upgrade_md) = run_command(
        &repo,
        &[
            "cell",
            "--cell-id",
            "background-job",
            "--mode",
            "upgrade-plan",
        ],
    );
    assert_eq!(upgrade["lifecycle_action"], "upgrade-plan");
    assert!(upgrade["upgrade_plan"]
        .as_array()
        .unwrap()
        .iter()
        .any(|note| { note.as_str().unwrap().contains("idempotency") }));
    assert!(upgrade_md.contains("Upgrade Plan"));

    let (deprecate, deprecate_md) = run_command(
        &repo,
        &[
            "cell",
            "--cell-id",
            "background-job",
            "--mode",
            "deprecate-plan",
        ],
    );
    assert_eq!(deprecate["lifecycle_action"], "deprecate-plan");
    assert!(deprecate["deprecation_plan"]
        .as_array()
        .unwrap()
        .iter()
        .any(|note| { note.as_str().unwrap().contains("pause workers") }));
    assert!(deprecate_md.contains("Deprecation Plan"));
}
