use std::fs;
use std::path::PathBuf;
use std::process::Command;

use jankurai::validation::{self, ArtifactSchema};
use tempfile::tempdir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

#[test]
fn publish_emits_schema_valid_bundle_and_badges_from_bench_certify_govern() {
    let sandbox = tempdir().unwrap();
    let work = tempdir().unwrap();
    let repo = sandbox.path().to_string_lossy();

    let bench_json = work.path().join("p12-benchmark-report.json");
    let bench_md = work.path().join("p12-benchmark-report.md");
    let certify_json = work.path().join("p12-certification.json");
    let certify_md = work.path().join("p12-certification.md");
    let govern_json = work.path().join("p12-governance-policy.json");
    let govern_md = work.path().join("p12-governance-policy.md");
    let public_json = work.path().join("p12-public-evidence.json");
    let public_md = work.path().join("p12-public-evidence.md");
    let badge_json = work.path().join("jankurai-badge.json");
    let badge_svg = work.path().join("jankurai-badge.svg");

    fn assert_ok(cmd: &mut Command) {
        let status = cmd.status().expect("spawn");
        assert!(status.success(), "command failed: {cmd:?}");
    }

    assert_ok(Command::new(binary_path()).args([
        "bench",
        repo.as_ref(),
        "--out",
        bench_json.to_str().unwrap(),
        "--md",
        bench_md.to_str().unwrap(),
    ]));

    assert_ok(Command::new(binary_path()).args([
        "certify",
        repo.as_ref(),
        "--out",
        certify_json.to_str().unwrap(),
        "--md",
        certify_md.to_str().unwrap(),
    ]));

    assert_ok(Command::new(binary_path()).args([
        "govern",
        repo.as_ref(),
        "--out",
        govern_json.to_str().unwrap(),
        "--md",
        govern_md.to_str().unwrap(),
    ]));

    assert_ok(Command::new(binary_path()).args([
        "publish",
        repo.as_ref(),
        "--certification",
        certify_json.to_str().unwrap(),
        "--benchmark",
        bench_json.to_str().unwrap(),
        "--governance",
        govern_json.to_str().unwrap(),
        "--out",
        public_json.to_str().unwrap(),
        "--md",
        public_md.to_str().unwrap(),
        "--badge-json",
        badge_json.to_str().unwrap(),
        "--badge-svg",
        badge_svg.to_str().unwrap(),
    ]));

    let public: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&public_json).unwrap()).unwrap();
    validation::validate_value(
        sandbox.path(),
        ArtifactSchema::PublicEvidenceBundle,
        &public,
    )
    .unwrap();

    assert!(matches!(
        public["public_status"].as_str(),
        Some("publishable" | "advisory" | "blocked")
    ));

    let badge: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&badge_json).unwrap()).unwrap();
    validation::validate_value(sandbox.path(), ArtifactSchema::CertificationBadge, &badge).unwrap();
    assert_eq!(badge["label"], "jankurai");

    let svg = fs::read_to_string(&badge_svg).unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("jankurai"));

    let md = fs::read_to_string(&public_md).unwrap();
    assert!(md.starts_with("# jankurai Public Evidence"));
    assert!(public["attestation"]["subject_digest"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
}
