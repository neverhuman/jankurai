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

#[test]
fn adopt_legacy_node_emits_migration_target_plan() {
    let repo = repo_root();
    let fixture = repo.join("examples/legacy-node-api");
    let out_dir = tempdir().unwrap();
    let json_path = out_dir.path().join("adoption-plan.json");
    let md_path = out_dir.path().join("adoption-plan.md");

    let status = Command::new(binary_path())
        .arg("adopt")
        .arg(&fixture)
        .arg("--mode")
        .arg("observe")
        .arg("--out")
        .arg(&json_path)
        .arg("--md")
        .arg(&md_path)
        .status()
        .unwrap();
    assert!(status.success());

    let plan: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    validation::validate_value(&repo, ArtifactSchema::AdoptionPlan, &plan).unwrap();
    assert_eq!(plan["command"], "jankurai adopt");
    assert_eq!(plan["mode"], "observe");
    assert_eq!(plan["recommended_profile"], "migration-target");
    assert!(plan["tool_rollout"].is_array());
    if let Some(first) = plan["tool_rollout"]
        .as_array()
        .and_then(|items| items.first())
    {
        assert!(
            first["next_command"]
                .as_str()
                .unwrap()
                .starts_with("cargo run -p jankurai")
                || first["next_command"]
                    .as_str()
                    .unwrap()
                    .starts_with("jankurai ")
        );
    }
    assert!(plan["safe_commands"]
        .as_array()
        .unwrap()
        .iter()
        .all(|command| !command.as_str().unwrap().contains("cargo run -p jankurai")));
    let md = fs::read_to_string(md_path).unwrap();
    assert!(md.starts_with("# jankurai Adoption Plan"));
    assert!(md.contains("## Tool Rollout"));
}

#[test]
fn ci_install_observe_dry_run_is_non_blocking_and_preserves_files() {
    let dir = tempdir().unwrap();
    let output = Command::new(binary_path())
        .arg("ci")
        .arg("install")
        .arg(dir.path())
        .arg("--github")
        .arg("--mode")
        .arg("observe")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!dir.path().join(".github/workflows/jankurai.yml").exists());

    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("jankurai audit . --mode advisory"));
    assert!(text.contains("cargo install jankurai --locked"));
    assert!(!text.contains("Enforce score floor"));
    assert!(!text.contains("-ge 85"));

    let workflow = dir.path().join(".github/workflows/jankurai.yml");
    fs::create_dir_all(workflow.parent().unwrap()).unwrap();
    fs::write(&workflow, "name: existing\n").unwrap();
    let status = Command::new(binary_path())
        .arg("ci")
        .arg("install")
        .arg(dir.path())
        .arg("--github")
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(fs::read_to_string(workflow).unwrap(), "name: existing\n");
}
