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
fn webhook_receiver_is_seventh_certified_dependency_bound_cell() {
    let repo = repo_root();

    let (registry, _) = run_command(&repo, &["registry"]);
    validation::validate_value(&repo, ArtifactSchema::CellRegistry, &registry).unwrap();

    let cells = registry["cells"].as_array().unwrap();
    assert!(
        cells.len() >= 7,
        "expected at least seven built-in cells, got {}",
        cells.len()
    );

    let webhook_receiver = cells
        .iter()
        .find(|cell| cell["cell_id"] == "webhook-receiver")
        .expect("webhook-receiver cell must be present in registry");

    assert_eq!(webhook_receiver["lifecycle"], "certified");
    assert_eq!(webhook_receiver["certification_status"], "certified");
    assert_eq!(webhook_receiver["category"], "integration");

    let evidence = webhook_receiver["certification_evidence"]
        .as_array()
        .unwrap();
    assert!(evidence.iter().any(|item| {
        item["kind"] == "content-marker"
            && item["path"]
                .as_str()
                .unwrap()
                .contains("domain-webhook-signature-policy")
            && item["status"] == "present"
    }));
}
