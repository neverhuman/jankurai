use jankurai::validation::{self, ArtifactSchema};
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn agent_boundaries_toml_validates_against_schema() {
    let repo = repo_root();
    let text = fs::read_to_string(repo.join("agent/boundaries.toml")).unwrap();
    validation::validate_boundaries_toml_text(&repo, &text).unwrap();
}

#[test]
fn boundaries_toml_minimal_fixture_validates() {
    let repo = repo_root();
    let text = r#"
[stack]
id = "fixture"

[queues]
adapter_paths = []
event_contract_paths = []
generated_type_paths = []
    "#;
    validation::validate_boundaries_toml_text(&repo, text).unwrap();
}

#[test]
fn boundaries_toml_accepts_python_non_product_allowlist() {
    let repo = repo_root();
    let text = r#"
[stack]
id = "fixture"

[queues]
adapter_paths = []
event_contract_paths = []
generated_type_paths = []

[python]
allowed_non_product_paths = ["seed_data/", "ops/scripts/", "crates/veox-bootstrap-interop/python_runtime/"]
"#;
    validation::validate_boundaries_toml_text(&repo, text).unwrap();
}

#[test]
fn boundaries_toml_without_queues_fails_schema() {
    let repo = repo_root();
    let text = r#"
[stack]
id = "only-stack"
"#;
    let err = validation::validate_boundaries_toml_text(&repo, text).unwrap_err();
    assert!(
        err.to_string().contains("queues") || err.to_string().contains("missing"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn boundaries_json_shape_enum_registered() {
    let repo = repo_root();
    let v = serde_json::json!({
        "stack": { "id": "x" },
        "queues": {
            "adapter_paths": [],
            "event_contract_paths": [],
            "generated_type_paths": []
        }
    });
    validation::validate_value(&repo, ArtifactSchema::Boundaries, &v).unwrap();
}
