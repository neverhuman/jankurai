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
fn auth_session_is_fourth_certified_dependency_bound_cell() {
    let repo = repo_root();

    let (registry, _registry_md) = run_command(&repo, &["registry"]);
    validation::validate_value(&repo, ArtifactSchema::CellRegistry, &registry).unwrap();

    let cells = registry["cells"].as_array().unwrap();
    assert!(
        cells.len() >= 4,
        "expected at least four built-in cells, got {}",
        cells.len()
    );

    let auth_session = cells
        .iter()
        .find(|cell| cell["cell_id"] == "auth-session")
        .expect("auth-session cell must be present in registry");

    assert_eq!(auth_session["lifecycle"], "certified");
    assert_eq!(auth_session["certification_status"], "certified");
    assert_eq!(auth_session["category"], "identity");

    // Dependency ordering
    let dependencies = auth_session["dependencies"].as_array().unwrap();
    assert!(dependencies.iter().any(|d| d == "audit-log"));
    assert!(dependencies.iter().any(|d| d == "rbac"));

    // Dependency evidence
    let evidence = auth_session["certification_evidence"].as_array().unwrap();
    assert!(evidence.iter().any(|item| {
        item["kind"] == "dependency" && item["path"] == "audit-log" && item["status"] == "present"
    }));
    assert!(evidence.iter().any(|item| {
        item["kind"] == "dependency" && item["path"] == "rbac" && item["status"] == "present"
    }));

    // Content-marker evidence for SessionTokenHash
    assert!(evidence.iter().any(|item| {
        item["kind"] == "content-marker"
            && item["path"]
                .as_str()
                .unwrap()
                .contains("domain-session-token-hash")
            && item["status"] == "present"
    }));

    // Proof lanes include security and ux-qa
    assert!(auth_session["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "security"));
    assert!(auth_session["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|lane| lane == "ux-qa"));
}

#[test]
fn auth_session_prove_emits_evidence_bound_decision() {
    let repo = repo_root();

    let (prove, prove_md) = run_command(
        &repo,
        &["cell", "--cell-id", "auth-session", "--mode", "prove"],
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
fn auth_session_upgrade_plan_emits_notes() {
    let repo = repo_root();

    let (upgrade, upgrade_md) = run_command(
        &repo,
        &[
            "cell",
            "--cell-id",
            "auth-session",
            "--mode",
            "upgrade-plan",
        ],
    );

    assert_eq!(upgrade["lifecycle_action"], "upgrade-plan");
    assert!(!upgrade["upgrade_plan"].as_array().unwrap().is_empty());
    assert!(upgrade_md.contains("Upgrade Plan"));
}

#[test]
fn auth_session_deprecate_plan_emits_notes() {
    let repo = repo_root();

    let (deprecate, deprecate_md) = run_command(
        &repo,
        &[
            "cell",
            "--cell-id",
            "auth-session",
            "--mode",
            "deprecate-plan",
        ],
    );

    assert_eq!(deprecate["lifecycle_action"], "deprecate-plan");
    assert!(!deprecate["deprecation_plan"].as_array().unwrap().is_empty());
    assert!(deprecate_md.contains("Deprecation Plan"));
}
