use std::fs;
use std::path::PathBuf;

use jankurai_proofbind::{build_proofbind, ProofBindMode, ProofBindRequest};
use tempfile::tempdir;

fn seed_repo() -> tempfile::TempDir {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("agent")).unwrap();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::write(
        repo.path().join("agent/owner-map.json"),
        r#"{"workspace":"fixture","owners":{"src/":"tools","db/":"standard","agent/":"agent"}}"#,
    )
    .unwrap();
    fs::write(
        repo.path().join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"src/":{"command":"cargo test -p fixture","purpose":"rust"},"db/":{"command":"cargo run -p jankurai -- migrate . --analyze --json target/jankurai/migration-report.json","purpose":"db"},"agent/":{"command":"just score","purpose":"agent"}}}"#,
    )
    .unwrap();
    fs::write(
        repo.path().join("agent/proof-lanes.toml"),
        r#"[[lane]]
name = "proofmark-rust"
command = "cargo test -p fixture"
purpose = "proofmark"

[[lane]]
name = "db-migration-analyze"
command = "cargo run -p jankurai -- migrate . --analyze --json target/jankurai/migration-report.json"
purpose = "migration"

[[lane]]
name = "audit"
command = "just score"
purpose = "audit"
"#,
    )
    .unwrap();
    repo
}

#[test]
fn authz_rust_change_emits_critical_obligation() {
    let repo = seed_repo();
    fs::write(
        repo.path().join("src/auth.rs"),
        "pub fn authorize(tenant_id: &str) -> bool { !tenant_id.is_empty() }\n",
    )
    .unwrap();
    let output = build_proofbind(ProofBindRequest {
        repo_root: repo.path().to_path_buf(),
        changed_paths: vec![PathBuf::from("src/auth.rs")],
        changed_from: None,
        mode: ProofBindMode::Advisory,
        proof_receipts: None,
    })
    .unwrap();
    assert!(output
        .witness
        .surfaces
        .iter()
        .any(|surface| surface.surface_type == "authz_boundary"));
    assert!(output.obligations.obligations.iter().any(|obligation| {
        obligation
            .rule_ids
            .contains(&"HLT-022-AUTHZ-ISOLATION-GAP".to_string())
            && obligation.severity == "critical"
    }));
    assert_eq!(output.obligations.summary.high_or_critical_missing, 1);
}

#[test]
fn proofmark_receipt_satisfies_matching_obligation() {
    let repo = seed_repo();
    fs::write(
        repo.path().join("src/lib.rs"),
        "pub fn api() -> bool { true }\n",
    )
    .unwrap();
    let initial = build_proofbind(ProofBindRequest {
        repo_root: repo.path().to_path_buf(),
        changed_paths: vec![PathBuf::from("src/lib.rs")],
        changed_from: None,
        mode: ProofBindMode::Advisory,
        proof_receipts: None,
    })
    .unwrap();
    let obligation_id = initial.obligations.obligations[0].obligation_id.clone();
    let receipt_dir = repo.path().join("target/jankurai/proofmark");
    fs::create_dir_all(&receipt_dir).unwrap();
    fs::write(
        receipt_dir.join("proof-receipt.json"),
        serde_json::json!({
            "lane": "proofmark-rust",
            "command": "jankurai proofmark rust",
            "exit_code": 0,
            "elapsed_ms": 1,
            "artifacts": [],
            "changed_paths": ["src/lib.rs"],
            "rules_covered": [{"rule_id":"HLT-007-HANDWRITTEN-CONTRACT","status":"covered"}],
            "extensions": {
                "proofmark": {
                    "satisfied_obligations": [obligation_id]
                }
            }
        })
        .to_string(),
    )
    .unwrap();
    let output = build_proofbind(ProofBindRequest {
        repo_root: repo.path().to_path_buf(),
        changed_paths: vec![PathBuf::from("src/lib.rs")],
        changed_from: None,
        mode: ProofBindMode::Advisory,
        proof_receipts: Some(PathBuf::from("target/jankurai/proofmark")),
    })
    .unwrap();
    assert_eq!(output.obligations.summary.satisfied, 1);
    assert_eq!(output.obligations.summary.missing, 0);
}
