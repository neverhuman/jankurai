use jankurai::validation::{self, ArtifactSchema};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn minimal_valid_envelope() -> serde_json::Value {
    serde_json::json!({
        "reports": [{
            "schemaVersion": "1.2.0",
            "toolVersion": "0.5.0",
            "url": "about:blank",
            "checkedAt": "2026-05-02T12:00:00.000Z",
            "viewport": { "width": 1280, "height": 720 },
            "metrics": {
                "scrollWidth": 1280,
                "clientWidth": 1280,
                "scrollHeight": 720,
                "clientHeight": 720
            },
            "elements": [],
            "violations": [],
            "artifacts": [],
            "summary": { "errors": 0, "warnings": 0, "byRule": {} },
            "decision": "pass"
        }]
    })
}

#[test]
fn ux_qa_report_envelope_validates() {
    let repo = repo_root();
    let value = minimal_valid_envelope();
    validation::validate_value(&repo, ArtifactSchema::UxQaReport, &value).unwrap();
}

#[test]
fn ux_qa_report_v14_visual_baseline_fields_validate() {
    let repo = repo_root();
    let mut value = minimal_valid_envelope();
    value["reports"][0]["schemaVersion"] = serde_json::json!("1.4.0");
    value["reports"][0]["artifacts"] = serde_json::json!([{
        "kind": "accessibility",
        "path": "target/jankurai/ux-qa/local.a11y.json",
        "sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "viewport": { "width": 1280, "height": 720 }
    }]);
    value["reports"][0]["visualBaseline"] = serde_json::json!({
        "mode": "review",
        "status": "changed",
        "decision": "review",
        "actualPath": "target/jankurai/ux-qa/local.png",
        "baselinePath": "target/jankurai/ux-qa/baseline.png",
        "diffPath": "target/jankurai/ux-qa/diff.json",
        "actualSha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "baselineSha256": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "owner": "design",
        "approvedBy": "ux",
        "approvedAt": "2026-05-02T12:00:00.000Z",
        "approvalNote": "fixture"
    });
    value["reports"][0]["artifactCoverage"] = serde_json::json!({
        "required": ["screenshot", "aria-snapshot", "accessibility"],
        "present": ["accessibility"],
        "missing": ["screenshot", "aria-snapshot"]
    });
    value["reports"][0]["accessibility"] = serde_json::json!({
        "violations": 0,
        "incomplete": 0,
        "passes": 3,
        "artifactPath": "target/jankurai/ux-qa/local.a11y.json"
    });
    validation::validate_value(&repo, ArtifactSchema::UxQaReport, &value).unwrap();
}

#[test]
fn ux_qa_report_missing_checked_at_fails() {
    let repo = repo_root();
    let mut value = minimal_valid_envelope();
    value["reports"][0]
        .as_object_mut()
        .unwrap()
        .remove("checkedAt");
    let err = validation::validate_value(&repo, ArtifactSchema::UxQaReport, &value).unwrap_err();
    assert!(
        err.to_string().contains("checkedAt") || err.to_string().contains("missing required"),
        "{}",
        err
    );
}

#[test]
fn ux_qa_report_wrong_schema_version_fails() {
    let repo = repo_root();
    let mut value = minimal_valid_envelope();
    value["reports"][0]["schemaVersion"] = serde_json::json!("1.0.0");
    let err = validation::validate_value(&repo, ArtifactSchema::UxQaReport, &value).unwrap_err();
    let s = err.to_string();
    assert!(
        s.contains("1.2.0")
            || s.contains("1.3.0")
            || s.contains("1.4.0")
            || s.contains("constant")
            || s.contains("const")
            || s.contains("enum"),
        "unexpected error: {s}"
    );
}
