use serde_yaml::Value as YamlValue;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn root_action_metadata_is_a_composite_jankurai_action() {
    let root = repo_root();
    let action_path = root.join("action.yml");
    let text = fs::read_to_string(&action_path).expect("read root action.yml");
    let yaml: YamlValue = serde_yaml::from_str(&text).expect("action.yml parses as YAML");

    assert_eq!(yaml["name"].as_str(), Some("Jankurai"));
    assert_eq!(yaml["runs"]["using"].as_str(), Some("composite"));
    assert!(yaml["inputs"].get("mode").is_some());
    assert!(yaml["inputs"].get("baseline").is_some());

    let steps = yaml["runs"]["steps"]
        .as_sequence()
        .expect("composite action has steps");
    let step_text = steps
        .iter()
        .map(|step| serde_yaml::to_string(step).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(step_text
        .contains("cargo install --path \"$GITHUB_ACTION_PATH/crates/jankurai\" --locked --force"));
    assert!(step_text.contains("rustup toolchain install stable --profile minimal"));
    assert!(step_text.contains("jankurai audit . --mode"));
    assert!(step_text.contains("--baseline"));
    assert!(step_text.contains("target/jankurai/jankurai.sarif"));
    assert!(step_text.contains("target/jankurai/repair-queue.jsonl"));
    assert!(!step_text.contains("dtolnay/rust-toolchain@stable"));
    assert!(!step_text.contains("continue-on-error"));
    assert!(!step_text.contains("pull_request_target"));
    assert!(!step_text.contains("write-all"));
}
