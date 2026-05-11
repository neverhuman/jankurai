use jankurai::validation;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn agent_ux_qa_toml_validates_against_policy_schema() {
    let repo = repo_root();
    let text = fs::read_to_string(repo.join("agent/ux-qa.toml")).unwrap();
    validation::validate_ux_qa_policy_toml_text(&repo, &text).unwrap();
}

#[test]
fn ux_qa_policy_invalid_decision_threshold_fails() {
    let repo = repo_root();
    let text = r#"
decisionThreshold = 3
artifactRoot = "."
"#;
    let err = validation::validate_ux_qa_policy_toml_text(&repo, text).unwrap_err();
    let s = err.to_string();
    assert!(
        s.contains("decisionThreshold") || s.contains("enum") || s.contains("expected"),
        "unexpected error: {s}"
    );
}

#[test]
fn ux_qa_policy_invalid_viewport_pattern_fails() {
    let repo = repo_root();
    let text = r#"
artifactRoot = "."
viewports = ["not-a-viewport"]
"#;
    let err = validation::validate_ux_qa_policy_toml_text(&repo, text).unwrap_err();
    let s = err.to_string();
    assert!(
        s.contains("pattern") || s.contains("viewports") || s.contains("does not match"),
        "unexpected error: {s}"
    );
}

#[test]
fn ux_qa_policy_route_missing_url_fails() {
    let repo = repo_root();
    let text = r#"
[[routes]]
id = "only-id"
"#;
    let err = validation::validate_ux_qa_policy_toml_text(&repo, text).unwrap_err();
    let s = err.to_string();
    assert!(
        s.contains("url") || s.contains("missing required"),
        "unexpected error: {s}"
    );
}
