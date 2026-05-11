use std::fs;
use std::process::Command;

use jankurai::validation::{self, ArtifactSchema};
use tempfile::tempdir;

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_jankurai")
}

fn seed_catalog(repo: &std::path::Path) {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/owner-map.json"),
        r#"{"workspace":"fixture","owners":{"src/":"tools","agent/":"agent","target/":"workspace"}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"src/":{"command":"cargo test -p fixture","purpose":"rust"},"agent/":{"command":"just score","purpose":"agent"}}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/proof-lanes.toml"),
        r#"[[lane]]
name = "proofmark-rust"
command = "cargo test -p fixture"
purpose = "proofmark"

[[lane]]
name = "audit"
command = "just score"
purpose = "audit"
"#,
    )
    .unwrap();
}

#[test]
fn proofbind_and_proofmark_help_surfaces_exist() {
    for args in [["proofbind", "--help"], ["proofmark", "--help"]] {
        let output = Command::new(binary_path()).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{args:?} failed\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Usage:"), "{stdout}");
    }
}

#[test]
fn proofbind_obligation_can_be_satisfied_by_proofmark_receipt() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::write(
        repo.path().join("src/lib.rs"),
        "pub fn api() -> bool { true }\n",
    )
    .unwrap();

    let status = Command::new(binary_path())
        .current_dir(repo.path())
        .arg("proofbind")
        .arg("verify")
        .arg(repo.path())
        .arg("--changed")
        .arg("src/lib.rs")
        .status()
        .unwrap();
    assert!(status.success());

    let obligations_path = repo
        .path()
        .join("target/jankurai/proofbind/obligations.json");
    let obligations: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&obligations_path).unwrap()).unwrap();
    validation::validate_value(
        repo.path(),
        ArtifactSchema::ProofBindObligations,
        &obligations,
    )
    .unwrap();
    assert_eq!(obligations["summary"]["missing"], 1);

    fs::write(
        repo.path().join("coverage.lcov"),
        "TN:\nSF:src/lib.rs\nDA:1,1\nend_of_record\n",
    )
    .unwrap();
    let status = Command::new(binary_path())
        .current_dir(repo.path())
        .arg("proofmark")
        .arg("rust")
        .arg(repo.path())
        .arg("--changed")
        .arg("src/lib.rs")
        .arg("--coverage")
        .arg("coverage.lcov")
        .status()
        .unwrap();
    assert!(status.success());

    let proofmark_receipt_path = repo
        .path()
        .join("target/jankurai/proofmark/proofmark-receipt.json");
    let proofmark: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&proofmark_receipt_path).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::ProofMarkReceipt, &proofmark).unwrap();
    assert_eq!(proofmark["summary"]["satisfied_obligations"], 1);

    let proof_receipt_path = repo
        .path()
        .join("target/jankurai/proofmark/proof-receipt.json");
    let proof_receipt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&proof_receipt_path).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::ProofReceipt, &proof_receipt).unwrap();

    let witness_out = repo.path().join("target/jankurai/merge-witness.json");
    let witness_md = repo.path().join("target/jankurai/merge-witness.md");
    let _output = Command::new(binary_path())
        .current_dir(repo.path())
        .arg("witness")
        .arg(repo.path())
        .arg("--changed")
        .arg("src/lib.rs")
        .arg("--proof-receipts")
        .arg(proof_receipt_path)
        .arg("--out")
        .arg(&witness_out)
        .arg("--md")
        .arg(&witness_md)
        .output()
        .unwrap();
    assert!(
        witness_out.exists(),
        "witness JSON is written before any merge gate failure"
    );
    let witness: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&witness_out).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::MergeWitness, &witness).unwrap();
    assert_eq!(witness["proofbind"]["missing_obligation_count"], 0);
}

#[test]
fn proofbind_required_mode_fails_when_any_obligation_is_missing() {
    let repo = tempdir().unwrap();
    seed_catalog(repo.path());
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::write(
        repo.path().join("src/lib.rs"),
        "pub fn api() -> bool { true }\n",
    )
    .unwrap();

    let output = Command::new(binary_path())
        .current_dir(repo.path())
        .arg("proofbind")
        .arg("verify")
        .arg(repo.path())
        .arg("--changed")
        .arg("src/lib.rs")
        .arg("--mode")
        .arg("required")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "required mode should fail when any obligation is unresolved"
    );
}
