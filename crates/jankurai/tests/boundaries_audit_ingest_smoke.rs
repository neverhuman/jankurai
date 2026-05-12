use jankurai::audit::run_audit;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn thin_repo(dir: &std::path::Path) {
    fs::write(dir.join("README.md"), "# thin repo\n").unwrap();
}

fn minimal_boundaries_toml() -> &'static str {
    r#"
[stack]
id = "fixture-stack"
version = "0.1.0"

[queues]
adapter_paths = ["a/"]
event_contract_paths = []
generated_type_paths = ["g/"]
client_markers = ["m"]
"#
}

fn runtime_repo(dir: &Path) {
    fs::write(dir.join("AGENTS.md"), "Read agent standard\n").unwrap();
    fs::write(
        dir.join("README.md"),
        "# fixture\nlayout map validate workspace\n",
    )
    .unwrap();
    fs::write(dir.join("Justfile"), "check:\n    cargo test\n").unwrap();
    fs::create_dir_all(dir.join("agent")).unwrap();
    fs::create_dir_all(dir.join("runtime_payload/python")).unwrap();
}

fn digest(path: &Path) -> String {
    let bytes = fs::read(path).unwrap();
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn boundary_manifest(boundary_id: &str, paths: &[&str], evidence: &str, extra: &str) -> String {
    format!(
        r#"
[stack]
id = "fixture-stack"

[queues]
adapter_paths = []
event_contract_paths = []
generated_type_paths = []

{}
"#,
        boundary_entry(boundary_id, paths, evidence, extra)
    )
}

fn boundary_entry(boundary_id: &str, paths: &[&str], evidence: &str, extra: &str) -> String {
    format!(
        r#"
[[audited_runtime_boundary]]
id = "{boundary_id}"
paths = [{paths}]
classification = "audited-runtime-payload"
product_surface = true
runtime_language = "python"
target_stack_exception = true
reclassifies = [
  "non-optimal-product-language-found",
  "too-much-python-in-product-surface",
  "python-direct-product-truth-or-db-ownership"
]
proof_command = "cargo test -p jankurai boundaries_audit_ingest_smoke"
rerun_command = "cargo test -p jankurai boundaries_audit_ingest_smoke"
required_evidence = ["{evidence}"]
required_checks = [
  "manifest-coverage",
  "payload-hash-match",
  "no-direct-db-access",
  "no-product-routes",
  "no-subprocess",
  "no-import-escape",
  "no-builtin-escape"
]
{extra}
"#,
        paths = paths
            .iter()
            .map(|path| format!(r#""{path}""#))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn write_passed_evidence(dir: &Path, boundary_id: &str, rel: &str, sha: &str) {
    let evidence_path = dir
        .join("target/jankurai/boundaries")
        .join(boundary_id)
        .join("evidence.json");
    fs::create_dir_all(evidence_path.parent().unwrap()).unwrap();
    let value = serde_json::json!({
        "boundary_id": boundary_id,
        "classification": "audited-runtime-payload",
        "runtime_language": "python",
        "paths": ["runtime_payload/python/*.py"],
        "files": [{ "path": rel, "sha256": sha }],
        "checks": [
            { "id": "manifest-coverage", "status": "passed" },
            { "id": "payload-hash-match", "status": "passed" },
            { "id": "no-direct-db-access", "status": "passed" },
            { "id": "no-product-routes", "status": "passed" },
            { "id": "no-subprocess", "status": "passed" },
            { "id": "no-import-escape", "status": "passed" },
            { "id": "no-builtin-escape", "status": "passed" }
        ],
        "summary": { "passed": true, "failed_count": 0 }
    });
    fs::write(evidence_path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
}

#[test]
fn audit_ingests_valid_boundaries_manifest_summary() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        minimal_boundaries_toml(),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let art = report.boundaries.artifact.as_ref().expect("artifact");
    assert_eq!(art.path, "agent/boundaries.toml");
    assert!(art.content_fingerprint.starts_with("sha256:"));
    assert_eq!(art.stack_id, "fixture-stack");
    assert_eq!(art.stack_version.as_deref(), Some("0.1.0"));
    assert_eq!(art.adapter_path_count, 1);
    assert_eq!(art.event_contract_path_count, 0);
    assert_eq!(art.generated_type_path_count, 1);
    assert_eq!(art.client_marker_count, 1);
    assert_eq!(art.streaming_exception_count, 0);
}

#[test]
fn audit_invalid_boundaries_toml_leaves_artifact_none() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        "[stack]\nid = \"only\"\n",
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.boundaries.artifact.is_none());
}

#[test]
fn audit_without_boundaries_file_leaves_artifact_none() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.boundaries.artifact.is_none());
}

#[test]
fn python_allowlist_paths_clear_python_caps_and_shape_largest_file_scoring() {
    let dir = tempdir().unwrap();
    runtime_repo(dir.path());
    let large_python = "value = 1\n".repeat(1201);
    fs::create_dir_all(dir.path().join("seed_data")).unwrap();
    fs::write(dir.path().join("seed_data/seed_payload.py"), &large_python).unwrap();
    fs::create_dir_all(
        dir.path()
            .join("crates/veox-bootstrap-interop/python_runtime"),
    )
    .unwrap();
    fs::write(
        dir.path()
            .join("crates/veox-bootstrap-interop/python_runtime/runtime.py"),
        &large_python,
    )
    .unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        r#"
[stack]
id = "fixture-stack"

[queues]
adapter_paths = []
event_contract_paths = []
generated_type_paths = []

[python]
allowed_non_product_paths = ["seed_data/", "ops/scripts/", "crates/veox-bootstrap-interop/python_runtime/"]
"#,
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    for cap in [
        "non-optimal-product-language-found",
        "too-much-python-in-product-surface",
        "python-direct-product-truth-or-db-ownership",
    ] {
        assert!(
            !report.caps_applied.iter().any(|applied| applied == cap),
            "{cap} should be ignored for allowed non-product python paths"
        );
    }

    let python = report
        .dimensions
        .iter()
        .find(|dimension| dimension.name == "Python containment and polyglot hygiene")
        .expect("python dimension");
    assert_eq!(python.score, 100, "{python:?}");

    let shape = report
        .dimensions
        .iter()
        .find(|dimension| dimension.name == "Code shape and semantic surface")
        .expect("shape dimension");
    assert_eq!(shape.score, 70, "{shape:?}");
}

#[test]
fn passing_audited_runtime_boundary_reclassifies_only_python_stack_caps() {
    let dir = tempdir().unwrap();
    runtime_repo(dir.path());
    let rel = "runtime_payload/python/payload.py";
    fs::write(dir.path().join(rel), "value = 1\n").unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        boundary_manifest(
            "payload",
            &["runtime_payload/python/*.py"],
            "target/jankurai/boundaries/payload/evidence.json",
            "",
        ),
    )
    .unwrap();
    write_passed_evidence(dir.path(), "payload", rel, &digest(&dir.path().join(rel)));

    let report = run_audit(dir.path(), &[]).unwrap();

    let boundary = report
        .boundaries
        .reclassifications
        .iter()
        .find(|boundary| boundary.id == "payload")
        .expect("boundary reclassification");
    assert_eq!(boundary.status, "passed");
    for cap in [
        "non-optimal-product-language-found",
        "too-much-python-in-product-surface",
        "python-direct-product-truth-or-db-ownership",
    ] {
        assert!(
            !report.caps_applied.iter().any(|applied| applied == cap),
            "{cap} should be removed by passed boundary evidence"
        );
    }
}

#[test]
fn missing_boundary_evidence_applies_targeted_gap_without_generic_python_move_fix() {
    let dir = tempdir().unwrap();
    runtime_repo(dir.path());
    fs::write(
        dir.path().join("runtime_payload/python/payload.py"),
        "value = 1\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        boundary_manifest(
            "payload",
            &["runtime_payload/python/*.py"],
            "target/jankurai/boundaries/payload/evidence.json",
            "",
        ),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();

    assert!(report
        .caps_applied
        .iter()
        .any(|cap| cap == "boundary-reclassification-evidence-gap"));
    assert!(!report
        .caps_applied
        .iter()
        .any(|cap| cap == "python-direct-product-truth-or-db-ownership"));
    assert!(report.findings.iter().any(|finding| {
        finding.rule_id.as_deref() == Some("HLT-028-BOUNDARY-EVIDENCE-GAP")
            && finding.rerun_command == "cargo test -p jankurai boundaries_audit_ingest_smoke"
    }));
}

#[test]
fn hash_mismatch_and_missing_file_coverage_name_exact_failed_file() {
    let dir = tempdir().unwrap();
    runtime_repo(dir.path());
    let rel = "runtime_payload/python/payload.py";
    fs::write(dir.path().join(rel), "value = 1\n").unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        boundary_manifest(
            "payload",
            &["runtime_payload/python/*.py"],
            "target/jankurai/boundaries/payload/evidence.json",
            "",
        ),
    )
    .unwrap();
    write_passed_evidence(
        dir.path(),
        "payload",
        "runtime_payload/python/other.py",
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    );

    let report = run_audit(dir.path(), &[]).unwrap();
    let boundary = report
        .boundaries
        .reclassifications
        .iter()
        .find(|boundary| boundary.id == "payload")
        .unwrap();

    assert_eq!(boundary.status, "failed");
    let checks = boundary
        .missing_checks
        .iter()
        .chain(boundary.failed_checks.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(checks.contains("runtime_payload/python/payload.py"));
    assert!(checks.contains("runtime_payload/python/other.py"));
}

#[test]
fn forbidden_python_boundary_behavior_rejects_reclassification() {
    for (marker, expected) in [
        ("import sqlalchemy\n", "no-direct-db-access"),
        ("import subprocess\n", "no-subprocess"),
        (
            "@app.route('/x')\ndef x():\n    return 1\n",
            "no-product-routes",
        ),
        ("import sys\nsys.path.append('x')\n", "no-import-escape"),
        ("eval('1')\n", "no-builtin-escape"),
    ] {
        let dir = tempdir().unwrap();
        runtime_repo(dir.path());
        let rel = "runtime_payload/python/payload.py";
        fs::write(dir.path().join(rel), marker).unwrap();
        fs::write(
            dir.path().join("agent/boundaries.toml"),
            boundary_manifest(
                "payload",
                &["runtime_payload/python/*.py"],
                "target/jankurai/boundaries/payload/evidence.json",
                "",
            ),
        )
        .unwrap();
        write_passed_evidence(dir.path(), "payload", rel, &digest(&dir.path().join(rel)));

        let report = run_audit(dir.path(), &[]).unwrap();
        let boundary = report
            .boundaries
            .reclassifications
            .iter()
            .find(|boundary| boundary.id == "payload")
            .unwrap();
        assert_eq!(boundary.status, "failed");
        assert!(
            boundary
                .failed_checks
                .iter()
                .any(|check| check.starts_with(expected)),
            "expected {expected} in {:?}",
            boundary.failed_checks
        );
        assert!(report
            .caps_applied
            .iter()
            .any(|cap| cap == "python-direct-product-truth-or-db-ownership"));
    }
}

#[test]
fn unrelated_cap_reclassification_is_invalid_and_does_not_suppress_secret_findings() {
    let dir = tempdir().unwrap();
    runtime_repo(dir.path());
    let rel = "runtime_payload/python/payload.py";
    fs::write(dir.path().join(rel), "api_key = 'abcdefghijk'\n").unwrap();
    let mut manifest = boundary_manifest(
        "payload",
        &["runtime_payload/python/*.py"],
        "target/jankurai/boundaries/payload/evidence.json",
        "",
    );
    manifest = manifest.replace(
        "\"python-direct-product-truth-or-db-ownership\"",
        "\"secret-like-content-detected\"",
    );
    fs::write(dir.path().join("agent/boundaries.toml"), manifest).unwrap();
    write_passed_evidence(dir.path(), "payload", rel, &digest(&dir.path().join(rel)));

    let report = run_audit(dir.path(), &[]).unwrap();
    let boundary = report
        .boundaries
        .reclassifications
        .iter()
        .find(|boundary| boundary.id == "payload")
        .unwrap();
    assert_eq!(boundary.status, "invalid");
    assert!(report
        .caps_applied
        .iter()
        .any(|cap| cap == "secret-like-content-detected"));
}

#[test]
fn overbroad_and_overlapping_boundary_paths_are_rejected() {
    let dir = tempdir().unwrap();
    runtime_repo(dir.path());
    let rel = "runtime_payload/python/payload.py";
    fs::write(dir.path().join(rel), "value = 1\n").unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        format!(
            r#"
[stack]
id = "fixture-stack"

[queues]
adapter_paths = []
event_contract_paths = []
generated_type_paths = []

{}
{}
"#,
            boundary_entry(
                "broad",
                &["**/*.py"],
                "target/jankurai/boundaries/broad/evidence.json",
                "",
            ),
            boundary_entry(
                "overlap",
                &["runtime_payload/python/*.py"],
                "target/jankurai/boundaries/overlap/evidence.json",
                "",
            )
        ),
    )
    .unwrap();
    write_passed_evidence(dir.path(), "broad", rel, &digest(&dir.path().join(rel)));
    write_passed_evidence(dir.path(), "overlap", rel, &digest(&dir.path().join(rel)));

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report
        .boundaries
        .reclassifications
        .iter()
        .any(|boundary| boundary.id == "broad" && boundary.status == "invalid"));
    assert!(report
        .boundaries
        .reclassifications
        .iter()
        .any(|boundary| boundary.id == "overlap" && boundary.status == "passed"));

    let dir = tempdir().unwrap();
    runtime_repo(dir.path());
    let rel = "runtime_payload/python/payload.py";
    fs::write(dir.path().join(rel), "value = 1\n").unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        format!(
            r#"
[stack]
id = "fixture-stack"

[queues]
adapter_paths = []
event_contract_paths = []
generated_type_paths = []

{}
{}
"#,
            boundary_entry(
                "one",
                &["runtime_payload/python/*.py"],
                "target/jankurai/boundaries/one/evidence.json",
                "",
            ),
            boundary_entry(
                "two",
                &["runtime_payload/python/*.py"],
                "target/jankurai/boundaries/two/evidence.json",
                "",
            )
        ),
    )
    .unwrap();
    write_passed_evidence(dir.path(), "one", rel, &digest(&dir.path().join(rel)));
    write_passed_evidence(dir.path(), "two", rel, &digest(&dir.path().join(rel)));

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(
        report
            .boundaries
            .reclassifications
            .iter()
            .filter(|boundary| boundary.status == "invalid")
            .count()
            >= 2
    );
}
