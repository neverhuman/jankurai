use std::fs;
use std::path::PathBuf;

use jankurai_proofmark::{build_proofmark, ProofMarkMode, ProofMarkRequest};
use tempfile::tempdir;

#[test]
fn lcov_covered_change_satisfies_non_boundary_obligation() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::create_dir_all(repo.path().join("target/jankurai/proofbind")).unwrap();
    fs::write(
        repo.path().join("src/lib.rs"),
        "pub fn api() -> bool { true }\n",
    )
    .unwrap();
    fs::write(
        repo.path().join("target/jankurai/proofbind/obligations.json"),
        serde_json::json!({
            "schema_version": "1.0.0",
            "standard_version": "0.7.0",
            "generated_at": "0",
            "repo_root": repo.path().display().to_string(),
            "git_head": "unknown",
            "mode": "advisory",
            "obligations": [{
                "obligation_id": "obligation:HLT-007-HANDWRITTEN-CONTRACT:surface:rust_public_api:src:lib:api",
                "surface_id": "surface:rust_public_api:src:lib:api",
                "path": "src/lib.rs",
                "symbol": "api",
                "surface_type": "rust_public_api",
                "severity": "medium",
                "risk_tags": ["public_api"],
                "rule_ids": ["HLT-007-HANDWRITTEN-CONTRACT"],
                "required_lanes": ["contract", "proofmark-rust"],
                "required_receipt_kinds": ["proof-receipt", "proofmark"],
                "repair_task": "prove public API",
                "satisfied": false,
                "status": "missing",
                "receipt_paths": []
            }],
            "summary": {
                "total": 1,
                "satisfied": 0,
                "missing": 1,
                "high_or_critical_missing": 0,
                "changed_surface_count": 1,
                "verdict": "review"
            }
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        repo.path().join("coverage.lcov"),
        "TN:\nSF:src/lib.rs\nDA:1,1\nend_of_record\n",
    )
    .unwrap();
    let output = build_proofmark(ProofMarkRequest {
        repo_root: repo.path().to_path_buf(),
        changed_paths: vec![PathBuf::from("src/lib.rs")],
        changed_from: None,
        obligations_path: Some(PathBuf::from("target/jankurai/proofbind/obligations.json")),
        coverage_path: Some(PathBuf::from("coverage.lcov")),
        mutation_path: None,
        negative_proofs: vec![],
        mode: ProofMarkMode::Advisory,
    })
    .unwrap();
    assert_eq!(output.receipt.summary.satisfied_obligations, 1);
    assert_eq!(output.receipt.coverage.status, "pass");
    assert_eq!(output.proof_receipt.lane, "proofmark-rust");
    assert_eq!(
        output.proof_receipt.extensions["proofmark"]["satisfied_obligations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn boundary_obligation_requires_negative_proof_marker() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::create_dir_all(repo.path().join("target/jankurai/proofbind")).unwrap();
    fs::write(
        repo.path().join("src/auth.rs"),
        "pub fn authorize() -> bool { true }\n",
    )
    .unwrap();
    let obligation_id =
        "obligation:HLT-022-AUTHZ-ISOLATION-GAP:surface:authz_boundary:src:auth:authz";
    fs::write(
        repo.path()
            .join("target/jankurai/proofbind/obligations.json"),
        serde_json::json!({
            "obligations": [{
                "obligation_id": obligation_id,
                "path": "src/auth.rs",
                "rule_ids": ["HLT-022-AUTHZ-ISOLATION-GAP"],
                "required_lanes": ["security", "proofmark-rust"],
                "required_receipt_kinds": ["proof-receipt", "proofmark", "negative-behavior-proof"],
                "severity": "critical"
            }]
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        repo.path().join("coverage.lcov"),
        "SF:src/auth.rs\nDA:1,1\nend_of_record\n",
    )
    .unwrap();
    fs::write(
        repo.path().join("mutation.json"),
        r#"{"survived":0,"timeout":0}"#,
    )
    .unwrap();
    let review = build_proofmark(ProofMarkRequest {
        repo_root: repo.path().to_path_buf(),
        changed_paths: vec![PathBuf::from("src/auth.rs")],
        changed_from: None,
        obligations_path: Some(PathBuf::from("target/jankurai/proofbind/obligations.json")),
        coverage_path: Some(PathBuf::from("coverage.lcov")),
        mutation_path: Some(PathBuf::from("mutation.json")),
        negative_proofs: vec![],
        mode: ProofMarkMode::Advisory,
    })
    .unwrap();
    assert_eq!(review.receipt.obligation_results[0].status, "review");
    let pass = build_proofmark(ProofMarkRequest {
        repo_root: repo.path().to_path_buf(),
        changed_paths: vec![PathBuf::from("src/auth.rs")],
        changed_from: None,
        obligations_path: Some(PathBuf::from("target/jankurai/proofbind/obligations.json")),
        coverage_path: Some(PathBuf::from("coverage.lcov")),
        mutation_path: Some(PathBuf::from("mutation.json")),
        negative_proofs: vec![obligation_id.into()],
        mode: ProofMarkMode::Advisory,
    })
    .unwrap();
    assert_eq!(pass.receipt.obligation_results[0].status, "pass");
}
