use std::fs;
use std::path::PathBuf;

use jankurai::model::{
    FileInfo, AUDITOR_VERSION, PAPER_EDITION, SCHEMA_VERSION, STANDARD_VERSION, TARGET_STACK_ID,
};
use jankurai::validation::{self, ArtifactSchema};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn coverage_schemas_parse_and_fixtures_validate() {
    let repo = repo_root();
    let sources_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/coverage-sources.schema.json")).unwrap(),
    )
    .unwrap();
    let audit_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/coverage-audit.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        sources_schema["$id"],
        "https://jankurai.dev/schemas/coverage-sources.schema.json"
    );
    assert_eq!(
        audit_schema["$id"],
        "https://jankurai.dev/schemas/coverage-audit.schema.json"
    );

    let config = fs::read_to_string(
        repo.join("crates/jankurai/tests/fixtures/coverage/minimal_coverage_sources.toml"),
    )
    .unwrap();
    validation::validate_coverage_sources_toml_text(&repo, &config).unwrap();

    let audit = serde_json::json!({
        "schema_version": 1,
        "generated_by": "jankurai coverage audit",
        "repo_root": ".",
        "config_path": "agent/coverage-sources.toml",
        "strict": false,
        "changed_from": null,
        "summary": {
            "status": "pass",
            "sources_total": 1,
            "sources_present": 1,
            "sources_missing": 0,
            "hard_findings": 0,
            "soft_findings": 0
        },
        "sources": [{
            "id": "fixture-lcov",
            "kind": "line_coverage",
            "format": "lcov",
            "mode": "required",
            "status": "pass",
            "artifact_paths": ["coverage/lcov.info"],
            "matched_artifact": "coverage/lcov.info",
            "applies_to": ["crates/**/*.rs"],
            "owner": "tools",
            "lane": "coverage-audit",
            "metrics": {"total_lines": 1},
            "parser_warnings": []
        }],
        "findings": []
    });
    validation::validate_value(&repo, ArtifactSchema::CoverageAudit, &audit).unwrap();
}

#[test]
fn copy_code_schema_parses_and_report_validates() {
    let repo = repo_root();
    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/copy-code.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        schema["$id"],
        "https://jankurai.dev/schemas/copy-code.schema.json"
    );

    let files = vec![
        FileInfo {
            rel_path: "src/a.rs".into(),
            name: "a.rs".into(),
            suffix: ".rs".into(),
            size: 32,
            line_count: 1,
            text: "pub fn run() { println!(\"hi\"); }\n".into(),
            is_generated: false,
            is_code: true,
        },
        FileInfo {
            rel_path: "src/b.rs".into(),
            name: "b.rs".into(),
            suffix: ".rs".into(),
            size: 32,
            line_count: 1,
            text: "pub fn run() { println!(\"hi\"); }\n".into(),
            is_generated: false,
            is_code: true,
        },
    ];
    let report = jankurai::audit::copy_code::scan_files(
        &repo,
        &files,
        jankurai::audit::copy_code::CopyCodeOptions::default(),
    );
    validation::validate_serializable(&repo, ArtifactSchema::CopyCode, &report).unwrap();
    assert_eq!(report.summary.hard_classes, 1);
    assert_eq!(report.classes[0].language, "rust");
}

#[test]
fn adoption_plan_schema_parses_and_fixture_validates() {
    let repo = repo_root();
    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/adoption-plan.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        schema["$id"],
        "https://jankurai.dev/schemas/adoption-plan.schema.json"
    );
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key == "recommended_profile"));

    let fixture = serde_json::json!({
        "schema_version": "1.0.0",
        "command": "jankurai adopt",
        "status": "complete",
        "generated_at": "0",
        "source_root": ".",
        "mode": "observe",
        "requested_profile": "auto",
        "recommended_profile": "migration-target",
        "risk_tier": "medium",
        "detected_surfaces": ["node"],
        "source_stack": "typescript/express",
        "target_stack": "rust-ts-postgres",
        "liability_score": 48,
        "audit_score": null,
        "safe_commands": ["jankurai audit . --mode advisory"],
        "tool_rollout": [],
        "stop_conditions": ["stop if workflow enforces a score gate"],
        "next_milestones": ["review plan"],
        "artifacts": ["target/jankurai/adoption-plan.json"],
        "warnings": []
    });
    validation::validate_value(&repo, ArtifactSchema::AdoptionPlan, &fixture).unwrap();
}

#[test]
fn score_history_schemas_parse_and_fixture_validate() {
    let repo = repo_root();
    let entry_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/score-history-entry.schema.json")).unwrap(),
    )
    .unwrap();
    let export_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/score-history-export.schema.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(
        entry_schema["$id"],
        "https://jankurai.dev/schemas/score-history-entry.schema.json"
    );
    assert_eq!(
        export_schema["$id"],
        "https://jankurai.dev/schemas/score-history-export.schema.json"
    );

    let entry = serde_json::json!({
        "schema_version": "1.1.0",
        "standard_version": "0.8.0",
        "auditor_version": "0.8.0",
        "generated_at": "2026-05-05T00:00:00Z",
        "run_id": "run-1",
        "repo_id": "sha256:repo",
        "repo_remote": "https://example.com/org/repo.git",
        "branch": "main",
        "commit": "abc123",
        "dirty_worktree": false,
        "scope": "full",
        "changed_paths": ["README.md"],
        "score": 91,
        "raw_score": 95,
        "finding_count": 1,
        "hard_findings": 0,
        "soft_findings": 1,
        "decision": "pass",
        "minimum_score": 85,
        "caps_applied": [],
        "report_fingerprint": "sha256:report",
        "input_fingerprint": "sha256:input",
        "policy_fingerprint": "sha256:policy",
        "repo_score_json_path": "agent/repo-score.json",
        "repo_score_md_path": "agent/repo-score.md"
    });
    validation::validate_value(&repo, ArtifactSchema::ScoreHistoryEntry, &entry).unwrap();

    let export = serde_json::json!({
        "schema_version": "1.1.0",
        "command": "jankurai history export",
        "history": "agent/score-history.jsonl",
        "window": 3,
        "source": "auto",
        "repo_id": "sha256:repo",
        "history_bytes": 128,
        "sample_count": 1,
        "latest_generated_at": "2026-05-05T00:00:00Z",
        "latest_commit": "abc123",
        "rows": [entry],
        "summary": {
            "source": "auto",
            "repo_id": "sha256:repo",
            "history_bytes": 128,
            "sample_count": 1,
            "latest_generated_at": "2026-05-05T00:00:00Z",
            "latest_commit": "abc123",
            "first_score": 91,
            "latest_score": 91,
            "score_delta": 0,
            "best_score": 91,
            "worst_score": 91,
            "latest_decision": "pass",
            "high_or_critical_latest": 0,
            "decision": "pass"
        }
    });
    validation::validate_value(&repo, ArtifactSchema::ScoreHistoryExport, &export).unwrap();
}

#[test]
fn cell_registry_and_manifest_schemas_parse() {
    let repo = repo_root();
    let manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/cell-manifest.schema.json")).unwrap(),
    )
    .unwrap();
    let registry: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/cell-registry.schema.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(
        manifest["$id"],
        "https://jankurai.dev/schemas/cell-manifest.schema.json"
    );
    assert_eq!(
        registry["$id"],
        "https://jankurai.dev/schemas/cell-registry.schema.json"
    );
    assert_eq!(
        registry["properties"]["cells"]["items"]["$ref"],
        "cell-manifest.schema.json"
    );

    let required = manifest["required"].as_array().unwrap();
    for key in [
        "cell_id",
        "version",
        "lifecycle",
        "install_strategy",
        "conflict_policy",
        "certification_evidence",
        "proof_commands",
        "rollback_notes",
        "certification_status",
    ] {
        assert!(required.iter().any(|value| value == key));
    }
    assert_eq!(
        manifest["properties"]["proof_lanes"]["items"]["type"],
        "string"
    );

    let proof_receipt: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/proof-receipt.schema.json")).unwrap(),
    )
    .unwrap();
    let proofbind_witness: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/proofbind-witness.schema.json")).unwrap(),
    )
    .unwrap();
    let proofbind_obligations: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/proofbind-obligations.schema.json")).unwrap(),
    )
    .unwrap();
    let proofmark_receipt: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/proofmark-receipt.schema.json")).unwrap(),
    )
    .unwrap();
    let proof_plan: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/proof-plan.schema.json")).unwrap(),
    )
    .unwrap();
    let proof_verification: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/proof-verification.schema.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(
        proof_receipt["$id"],
        "https://jankurai.dev/schemas/proof-receipt.schema.json"
    );
    assert_eq!(
        proofbind_witness["$id"],
        "https://jankurai.dev/schemas/proofbind-witness.schema.json"
    );
    assert_eq!(
        proofbind_obligations["$id"],
        "https://jankurai.dev/schemas/proofbind-obligations.schema.json"
    );
    assert_eq!(
        proofmark_receipt["$id"],
        "https://jankurai.dev/schemas/proofmark-receipt.schema.json"
    );
    assert_eq!(
        proof_plan["$id"],
        "https://jankurai.dev/schemas/proof-plan.schema.json"
    );
    assert_eq!(
        proof_verification["$id"],
        "https://jankurai.dev/schemas/proof-verification.schema.json"
    );
    assert_eq!(proof_receipt["properties"]["lane"]["type"], "string");
    assert_eq!(proof_plan["properties"]["changed_paths"]["type"], "array");
    assert!(proof_plan["properties"].get("route_decisions").is_some());
    assert!(proof_receipt["properties"].get("plan_digest").is_some());
    assert!(proof_receipt["properties"]
        .get("artifact_digests")
        .is_some());
    assert_eq!(
        proofbind_witness["properties"]["surfaces"]["items"]["properties"]["surface_type"]["enum"],
        serde_json::json!([
            "rust_public_api",
            "authz_boundary",
            "input_boundary",
            "sql_query",
            "db_migration",
            "cli_command",
            "mcp_tool",
            "unsafe_or_process_sink",
            "business_invariant"
        ])
    );
    assert!(proofbind_obligations["properties"]
        .get("obligations")
        .is_some());
    assert!(proofmark_receipt["properties"]
        .get("obligation_results")
        .is_some());
    assert!(proof_verification["properties"]
        .get("manifest_fingerprints")
        .is_some());

    let benchmark_suite: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/benchmark-suite.schema.json")).unwrap(),
    )
    .unwrap();
    let benchmark_report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/benchmark-report.schema.json")).unwrap(),
    )
    .unwrap();
    let certification: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/certification.schema.json")).unwrap(),
    )
    .unwrap();
    let governance_policy: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/governance-policy.schema.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(
        benchmark_suite["$id"],
        "https://jankurai.dev/schemas/benchmark-suite.schema.json"
    );
    let bs_required = benchmark_suite["required"].as_array().unwrap();
    for key in ["fixtures", "tasks"] {
        assert!(bs_required.iter().any(|value| value == key));
    }
    assert!(benchmark_suite["properties"].get("fixtures").is_some());
    assert!(benchmark_suite["properties"].get("tasks").is_some());

    assert_eq!(
        benchmark_report["$id"],
        "https://jankurai.dev/schemas/benchmark-report.schema.json"
    );
    let br_required = benchmark_report["required"].as_array().unwrap();
    for key in ["results", "summary", "target_stack_id"] {
        assert!(br_required.iter().any(|value| value == key));
    }
    assert!(benchmark_report["properties"].get("results").is_some());
    assert!(benchmark_report["properties"].get("summary").is_some());

    assert_eq!(
        certification["$id"],
        "https://jankurai.dev/schemas/certification.schema.json"
    );
    let cert_required = certification["required"].as_array().unwrap();
    for key in [
        "standard_version",
        "score",
        "conformance_level",
        "proof_receipt_index",
        "security_receipt_index",
        "ux_receipt_index",
        "contract_db_receipt_index",
    ] {
        assert!(cert_required.iter().any(|value| value == key));
    }
    assert!(certification["properties"]
        .get("findings_summary")
        .is_some());
    assert!(certification["properties"].get("provenance").is_some());

    assert_eq!(
        governance_policy["$id"],
        "https://jankurai.dev/schemas/governance-policy.schema.json"
    );
    let gp_required = governance_policy["required"].as_array().unwrap();
    for key in [
        "minimum_score",
        "fail_on",
        "rule_change_policy",
        "exception_policy",
    ] {
        assert!(gp_required.iter().any(|value| value == key));
    }
    assert!(governance_policy["properties"]
        .get("update_channel")
        .is_some());
    assert_eq!(
        governance_policy["properties"]["update_channel"]["enum"],
        serde_json::json!(["draft", "beta", "stable", "lts"])
    );

    let evidence_index: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/evidence-index.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        evidence_index["$id"],
        "https://jankurai.dev/schemas/evidence-index.schema.json"
    );
    assert_eq!(evidence_index["properties"]["plan_path"]["type"], "string");
    let ei_props = evidence_index["properties"].as_object().unwrap();
    assert!(ei_props.contains_key("ux_qa_report_path"));
    assert!(ei_props.contains_key("security_evidence_path"));
    assert!(ei_props.contains_key("repo_score_json_path"));
    assert!(ei_props.contains_key("coverage_audit_path"));
    assert!(ei_props.contains_key("sarif_path"));
    assert!(ei_props.contains_key("github_step_summary_path"));
    assert!(ei_props.contains_key("repair_queue_jsonl_path"));
    assert!(ei_props.contains_key("boundaries_manifest_path"));
    assert!(ei_props.contains_key("ux_qa_report_digest"));
    let ei_required = evidence_index["required"].as_array().unwrap();
    for key in [
        "ux_qa_report_path",
        "ux_qa_report_digest",
        "security_evidence_path",
        "repo_score_json_path",
        "coverage_audit_path",
        "sarif_path",
        "github_step_summary_path",
        "repair_queue_jsonl_path",
        "boundaries_manifest_path",
    ] {
        assert!(
            !ei_required.iter().any(|v| v == key),
            "companion path {key} must stay optional"
        );
    }
    assert!(evidence_index["properties"].get("plan_digest").is_some());
    assert!(evidence_index["properties"]
        .get("manifest_fingerprints")
        .is_some());
    assert!(evidence_index["properties"]
        .get("command_digests")
        .is_some());
    assert!(evidence_index["properties"].get("log_digests").is_some());
    assert!(evidence_index["properties"]
        .get("artifact_digests")
        .is_some());
    assert!(evidence_index["properties"]
        .get("receipt_digests")
        .is_some());
    assert!(evidence_index["properties"]
        .get("coverage_verdicts")
        .is_some());

    let init_profile: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/init-profile.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        init_profile["$id"],
        "https://jankurai.dev/schemas/init-profile.schema.json"
    );

    let security_evidence: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/security-evidence.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        security_evidence["$id"],
        "https://jankurai.dev/schemas/security-evidence.schema.json"
    );
    assert_eq!(security_evidence["properties"]["lane"]["const"], "security");
    assert!(security_evidence["properties"].get("policy").is_some());
    assert!(security_evidence["properties"].get("commands").is_some());

    let proof_verification: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/proof-verification.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        proof_verification["$id"],
        "https://jankurai.dev/schemas/proof-verification.schema.json"
    );
    assert!(proof_verification["properties"].get("verdict").is_some());
    assert!(proof_verification["properties"]
        .get("manifest_fingerprints")
        .is_some());
    assert!(proof_verification["properties"]
        .get("coverage_verdicts")
        .is_some());

    let postmortem_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/postmortem.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        postmortem_schema["$id"],
        "https://jankurai.dev/schemas/postmortem.schema.json"
    );
    assert_eq!(
        postmortem_schema["properties"]["failure_mode"]["enum"],
        serde_json::json!([
            "aspirational-spec",
            "env-prerequisite",
            "interop-runtime",
            "equivalence-gap",
            "cutover-rollback",
            "perf-regression"
        ])
    );
    let postmortem = serde_json::json!({
        "schema_version": "1.0.0",
        "postmortem_id": "alpha",
        "title": "Missing env bootstrap",
        "owner": "tools",
        "failure_mode": "env-prerequisite",
        "severity": "high",
        "blocker_type": "env-prerequisite",
        "summary": "The migration stalled because the required environment variable was absent.",
        "evidence": [
            "MODEL_HMAC_KEY was not present",
            "local handoff could not start"
        ],
        "actions": [
            "document the prerequisite in the runbook",
            "add a preflight check before the cutover step"
        ],
        "notes": ["captured after a failed handoff"],
        "source": "docs/incidents/alpha.md",
        "recorded_at": "2026-05-11T00:00:00Z"
    });
    validation::validate_value(&repo, ArtifactSchema::Postmortem, &postmortem).unwrap();

    let doctor_receipt: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/doctor-receipt.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        doctor_receipt["$id"],
        "https://jankurai.dev/schemas/doctor-receipt.schema.json"
    );
    assert!(
        doctor_receipt["properties"]["diagnostics"]["items"]["properties"]
            .get("kind")
            .is_some()
    );

    let context_pack: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/context-pack.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        context_pack["$id"],
        "https://jankurai.dev/schemas/context-pack.schema.json"
    );
    assert!(context_pack["properties"].get("included_files").is_some());
    assert!(context_pack["properties"]
        .get("source_trust_summary")
        .is_some());

    let kickoff_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/kickoff.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        kickoff_schema["$id"],
        "https://jankurai.dev/schemas/kickoff.schema.json"
    );
    assert!(kickoff_schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key == "clarifying_questions"));
    assert!(kickoff_schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key == "forbidden_paths"));
    assert!(kickoff_schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key == "proof_lanes"));
    assert!(kickoff_schema["properties"]
        .get("implementation_steps")
        .is_some());
    assert!(kickoff_schema["properties"]
        .get("parallel_packets")
        .is_some());
    assert!(kickoff_schema["properties"]
        .get("forbidden_paths")
        .is_some());
    assert!(kickoff_schema["properties"].get("proof_lanes").is_some());

    let kickoff = serde_json::json!({
        "schema_version": "1.0.0",
        "command": "jankurai kickoff",
        "generated_at": "2026-05-05T00:00:00Z",
        "repo": ".",
        "intent": "Add a README clarification",
        "agent": "generic",
        "changed_paths": [],
        "read_first": ["AGENTS.md", "agent/JANKURAI_STANDARD.md"],
        "route_decisions": [],
        "generated_zone_touches": [],
        "ownership_boundaries": [],
        "forbidden_paths": ["reference/", "target/"],
        "proof_lanes": ["fast", "audit"],
        "clarifying_questions": [{
            "id": "scope",
            "question": "Which exact files or directories are in scope?",
            "reason": "kickoff needs concrete changed paths before it can hand off bounded work for `Add a README clarification`",
            "suggested_paths": [],
            "blocking": true
        }],
        "implementation_steps": [{
            "order": 1,
            "title": "Read the kickoff sources",
            "files": ["AGENTS.md", "agent/JANKURAI_STANDARD.md", "docs/agent-native-standard.md", "docs/mission.md", "docs/moonshot.md", "README.md", "docs/artifact-contracts.md", "docs/testing.md", "agent/owner-map.json", "agent/test-map.json", "agent/generated-zones.toml", "agent/proof-lanes.toml"],
            "commands": [],
            "notes": "Start from the repo guardrails and policy maps before editing."
        }],
        "parallel_packets": [],
        "stop_conditions": ["stop before editing until the blocking clarifying questions are answered"],
        "expected_receipts": ["target/jankurai/kickoff.json", "target/jankurai/kickoff.md"],
        "next_commands": ["jankurai context-pack . --changed README.md --max-tokens 6000 --out target/jankurai/context-pack.json --md target/jankurai/context-pack.md"]
    });
    validation::validate_value(&repo, ArtifactSchema::Kickoff, &kickoff).unwrap();

    for (file, id) in [
        (
            "merge-witness.schema.json",
            "https://jankurai.dev/schemas/merge-witness.schema.json",
        ),
        (
            "score-diff.schema.json",
            "https://jankurai.dev/schemas/score-diff.schema.json",
        ),
        (
            "score-trend.schema.json",
            "https://jankurai.dev/schemas/score-trend.schema.json",
        ),
        (
            "rule-registry.schema.json",
            "https://jankurai.dev/schemas/rule-registry.schema.json",
        ),
        (
            "rule-verify.schema.json",
            "https://jankurai.dev/schemas/rule-verify.schema.json",
        ),
    ] {
        let schema: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(repo.join("schemas").join(file)).unwrap())
                .unwrap();
        assert_eq!(schema["$id"], id);
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|key| key == "schema_version"));
    }

    let repair_plan: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/repair-plan.schema.json")).unwrap(),
    )
    .unwrap();
    let repair_pr_draft: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/repair-pr-draft.schema.json")).unwrap(),
    )
    .unwrap();
    let optimization_report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/optimization-report.schema.json")).unwrap(),
    )
    .unwrap();
    let exception_expiry_report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/exception-expiry-report.schema.json")).unwrap(),
    )
    .unwrap();
    let repair_packet: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/repair-packet.schema.json")).unwrap(),
    )
    .unwrap();
    let repair_run: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/repair-run.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        repair_plan["$id"],
        "https://jankurai.dev/schemas/repair-plan.schema.json"
    );
    let rp_required = repair_plan["required"].as_array().unwrap();
    assert!(rp_required.iter().any(|value| value == "packets"));
    for key in [
        "plan_mode",
        "planned_edits",
        "planned_commands",
        "proof_lanes",
        "rollback_guidance",
        "human_approval_requirements",
    ] {
        assert!(rp_required.iter().any(|value| value == key));
    }
    assert_eq!(repair_plan["properties"]["plan_mode"]["const"], "dry-run");
    assert_eq!(
        repair_plan["properties"]["planned_edits"]["items"]["properties"]["operation"]["enum"],
        serde_json::json!([
            "modify",
            "regenerate",
            "review-only",
            "none",
            "append-text",
            "replace-exact",
            "create-file"
        ])
    );
    let planned_edit_props = repair_plan["properties"]["planned_edits"]["items"]["properties"]
        .as_object()
        .unwrap();
    for key in [
        "finding_fingerprint",
        "rule_id",
        "apply_strategy",
        "match_text",
        "replacement_text",
        "append_text",
        "create_text",
    ] {
        assert!(planned_edit_props.contains_key(key));
    }
    assert_eq!(
        repair_packet["$id"],
        "https://jankurai.dev/schemas/repair-packet.schema.json"
    );
    let packet_required = repair_packet["required"].as_array().unwrap();
    for key in ["repair_eligibility", "risk_level", "eligibility_reason"] {
        assert!(packet_required.iter().any(|value| value == key));
    }
    assert_eq!(
        repair_packet["properties"]["repair_eligibility"]["enum"],
        serde_json::json!([
            "auto-safe",
            "agent-assisted",
            "human-required",
            "never-auto"
        ])
    );
    assert_eq!(
        repair_run["$id"],
        "https://jankurai.dev/schemas/repair-run.schema.json"
    );
    let run_required = repair_run["required"].as_array().unwrap();
    for key in [
        "execution_mode",
        "auto_pr_status",
        "risk_summary",
        "blocked_packets",
        "applied_edits",
        "skipped_edits",
        "files_written",
        "proof_lanes",
    ] {
        assert!(run_required.iter().any(|value| value == key));
    }
    assert_eq!(
        repair_run["properties"]["status"]["enum"],
        serde_json::json!(["complete", "blocked", "failed"])
    );
    assert_eq!(
        repair_run["properties"]["execution_mode"]["enum"],
        serde_json::json!(["dry-run", "fixture-apply", "real-apply"])
    );
    assert_eq!(
        repair_run["properties"]["auto_pr_status"]["enum"],
        serde_json::json!([
            "not-requested",
            "eligible-dry-run-only",
            "blocked",
            "prepared",
            "created"
        ])
    );
    assert!(repair_run["properties"].get("auto_pr_draft").is_some());
    assert!(repair_run["properties"].get("git_mutation").is_some());
    assert!(repair_run["properties"].get("github_pr").is_some());
    let auto_pr_draft_props = repair_run["properties"]["auto_pr_draft"]["properties"]
        .as_object()
        .unwrap();
    for key in [
        "status",
        "branch_name",
        "commit_title",
        "pr_title",
        "planned_changed_paths",
        "proof_lanes",
        "artifact_links",
        "git_mutation_allowed",
        "github_mutation_allowed",
    ] {
        assert!(auto_pr_draft_props.contains_key(key));
    }
    let git_mutation_props = repair_run["properties"]["git_mutation"]["properties"]
        .as_object()
        .unwrap();
    for key in [
        "status",
        "base_branch",
        "head_branch",
        "base_sha",
        "head_sha",
        "commit_title",
        "files_committed",
        "rollback_command",
        "remote",
        "pushed",
    ] {
        assert!(git_mutation_props.contains_key(key));
    }
    let github_pr_props = repair_run["properties"]["github_pr"]["properties"]
        .as_object()
        .unwrap();
    for key in [
        "status",
        "draft",
        "remote",
        "base_branch",
        "head_branch",
        "command",
    ] {
        assert!(github_pr_props.contains_key(key));
    }
    assert!(repair_run["properties"]
        .get("proof_evidence_index")
        .is_some());

    assert_eq!(
        optimization_report["$id"],
        "https://jankurai.dev/schemas/optimization-report.schema.json"
    );
    let opt_required = optimization_report["required"].as_array().unwrap();
    for key in [
        "mode",
        "context_size_before_bytes",
        "context_size_after_bytes",
        "estimated_tokens_before",
        "estimated_tokens_after",
        "context_files",
        "benchmark_summary",
        "findings",
        "proof_requirements",
    ] {
        assert!(opt_required.iter().any(|value| value == key));
    }
    assert_eq!(
        optimization_report["properties"]["mode"]["enum"],
        serde_json::json!(["all", "token", "performance", "dependency", "dead-code"])
    );
    assert!(optimization_report["properties"]
        .get("context_files")
        .is_some());
    assert!(optimization_report["properties"]
        .get("benchmark_summary")
        .is_some());
    assert!(optimization_report["properties"].get("findings").is_some());

    assert_eq!(
        exception_expiry_report["$id"],
        "https://jankurai.dev/schemas/exception-expiry-report.schema.json"
    );
    let expiry_required = exception_expiry_report["required"].as_array().unwrap();
    for key in [
        "exception_root",
        "warning_days",
        "total_exceptions",
        "expired_count",
        "expiring_soon_count",
        "invalid_count",
        "exceptions",
        "proof_requirements",
    ] {
        assert!(expiry_required.iter().any(|value| value == key));
    }
    assert_eq!(
        exception_expiry_report["properties"]["status"]["enum"],
        serde_json::json!(["complete", "blocked", "failed"])
    );
    let expiry_item_props = exception_expiry_report["properties"]["exceptions"]["items"]
        ["properties"]
        .as_object()
        .unwrap();
    for key in [
        "path",
        "code",
        "owner",
        "reason",
        "expires",
        "migration_plan",
        "proof_lane",
        "status",
        "repair_options",
        "notes",
    ] {
        assert!(expiry_item_props.contains_key(key));
    }

    assert_eq!(
        repair_pr_draft["$id"],
        "https://jankurai.dev/schemas/repair-pr-draft.schema.json"
    );
    let draft_required = repair_pr_draft["required"].as_array().unwrap();
    for key in [
        "source_plan",
        "status",
        "execution_mode",
        "branch_name",
        "commit_title",
        "pr_title",
        "pr_body",
        "planned_changed_paths",
        "eligible_packets",
        "blocked_packets",
        "proof_lanes",
        "artifact_links",
        "residual_risk",
        "safety_notes",
        "git_mutation_allowed",
        "github_mutation_allowed",
    ] {
        assert!(draft_required.iter().any(|value| value == key));
    }
    assert_eq!(
        repair_pr_draft["properties"]["status"]["enum"],
        serde_json::json!(["draft-only", "blocked", "failed"])
    );
    assert_eq!(
        repair_pr_draft["properties"]["execution_mode"]["enum"],
        serde_json::json!(["dry-run"])
    );
    assert!(repair_pr_draft["properties"]
        .get("eligible_packets")
        .is_some());
    assert!(repair_pr_draft["properties"]
        .get("blocked_packets")
        .is_some());

    let boundaries: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/boundaries.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        boundaries["$id"],
        "https://jankurai.dev/schemas/boundaries.schema.json"
    );
    let b_required = boundaries["required"].as_array().unwrap();
    assert!(b_required.iter().any(|value| value == "stack"));
    assert!(b_required.iter().any(|value| value == "queues"));
    assert!(boundaries["properties"]
        .get("audited_runtime_boundary")
        .is_some());
    let boundary_evidence: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/boundary-evidence.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        boundary_evidence["$id"],
        "https://jankurai.dev/schemas/boundary-evidence.schema.json"
    );
    let boundary_evidence_fixture = serde_json::json!({
        "boundary_id": "runtime-payload",
        "classification": "audited-runtime-payload",
        "runtime_language": "python",
        "paths": ["runtime_payload/python/**/*.py"],
        "files": [
            {
                "path": "runtime_payload/python/payload.py",
                "sha256": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            }
        ],
        "checks": [
            { "id": "manifest-coverage", "status": "passed" }
        ],
        "summary": { "passed": true, "failed_count": 0 }
    });
    validation::validate_value(
        &repo,
        ArtifactSchema::BoundaryEvidence,
        &boundary_evidence_fixture,
    )
    .unwrap();

    let ux_policy: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/ux-qa-policy.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        ux_policy["$id"],
        "https://jankurai.dev/schemas/ux-qa-policy.schema.json"
    );
    assert!(ux_policy["properties"].get("artifactRoot").is_some());
    assert!(ux_policy["properties"].get("outputRoot").is_some());

    let ux_report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(repo.join("schemas/ux-qa.schema.json")).unwrap())
            .unwrap();
    assert_eq!(
        ux_report["$id"],
        "https://jankurai.dev/schemas/ux-qa.schema.json"
    );
    let ur_required = ux_report["required"].as_array().unwrap();
    assert!(ur_required.iter().any(|value| value == "reports"));
    assert!(
        ux_report["$defs"]["uxQaReport"]["properties"]["schemaVersion"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "1.2.0")
    );
    assert!(
        ux_report["$defs"]["uxQaReport"]["properties"]["schemaVersion"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "1.3.0")
    );
    assert!(
        ux_report["$defs"]["uxQaReport"]["properties"]["schemaVersion"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "1.4.0")
    );
    assert!(ux_report["$defs"].get("uxQaAccessibilitySummary").is_some());
    assert!(ux_report["$defs"].get("uxQaArtifactCoverage").is_some());
    assert!(ux_report["$defs"]
        .get("uxQaVisualBaselineSummary")
        .is_some());
    assert!(ux_report["$defs"]["uxQaArtifact"]["properties"]
        .get("sha256")
        .is_some());

    let tool_adoption_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/tool-adoption.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        tool_adoption_schema["$id"],
        "https://jankurai.dev/schemas/tool-adoption.schema.json"
    );
    assert!(tool_adoption_schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "tools"));
    let tool_fixture = serde_json::json!({
        "schema_version": "1.0.0",
        "tools": [
            { "id": "audit-ci", "mode": "auto" },
            { "id": "security", "mode": "required" }
        ]
    });
    validation::validate_value(&repo, ArtifactSchema::ToolAdoption, &tool_fixture).unwrap();

    let repo_score: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/repo-score.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        repo_score["$id"],
        "https://jankurai.dev/schemas/repo-score.schema.json"
    );
    let rs_required = repo_score["required"].as_array().unwrap();
    for key in [
        "paper_edition",
        "target_stack_id",
        "target_stack",
        "repo",
        "scope",
        "caps_applied",
        "hard_rules",
        "profile_structure",
        "decision",
        "git",
        "policy",
        "tool_adoption",
    ] {
        assert!(
            rs_required.iter().any(|value| value == key),
            "repo-score schema must require `{key}` for audit contract parity"
        );
    }
    assert_eq!(
        repo_score["properties"]["decision"]["$ref"],
        "#/$defs/reportDecision"
    );
    assert!(repo_score["$defs"].get("dimensionResult").is_some());
    assert!(repo_score["$defs"].get("scope").is_some());
    assert!(repo_score["properties"].get("ux_qa").is_some());
    assert!(repo_score["properties"].get("profile_structure").is_some());
    assert_eq!(
        repo_score["properties"]["profile_structure"]["$ref"],
        "#/$defs/profileStructureReadiness"
    );
    assert!(repo_score["$defs"].get("profileStructureCell").is_some());
    assert!(repo_score["$defs"]
        .get("profileStructureReadiness")
        .is_some());
    assert_eq!(
        repo_score["properties"]["ux_qa"]["$ref"],
        "#/$defs/uxQaReadiness"
    );
    assert!(repo_score["properties"].get("tool_adoption").is_some());
    assert_eq!(
        repo_score["properties"]["tool_adoption"]["$ref"],
        "#/$defs/toolAdoptionReadiness"
    );
    let ux_ready = &repo_score["$defs"]["uxQaReadiness"];
    assert!(ux_ready["properties"].get("artifact").is_some());
    let ux_art = &repo_score["$defs"]["uxQaReportArtifactSummary"];
    assert!(ux_art["properties"]
        .get("artifact_counts_by_kind")
        .is_some());
    assert!(ux_art["properties"]
        .get("accessibility_violation_total")
        .is_some());
    assert!(ux_art["properties"]
        .get("artifact_fingerprint_count")
        .is_some());
    assert!(ux_art["properties"]
        .get("visual_baseline_missing")
        .is_some());
    assert!(ux_art["properties"]
        .get("visual_baseline_changed")
        .is_some());
    assert!(ux_art["properties"].get("visual_baseline_review").is_some());
    assert!(ux_art["properties"].get("visual_baseline_block").is_some());
    let tool_ready = &repo_score["$defs"]["toolAdoptionReadiness"];
    assert!(tool_ready["properties"].get("items").is_some());
    assert!(tool_ready["properties"].get("missing").is_some());
    assert_eq!(
        repo_score["$defs"]["toolAdoptionItem"]["properties"]["status"]["enum"],
        serde_json::json!([
            "not_applicable",
            "missing",
            "configured",
            "ci_evidence",
            "artifact_verified"
        ])
    );
    assert!(repo_score["properties"].get("security_evidence").is_some());
    assert_eq!(
        repo_score["properties"]["security_evidence"]["$ref"],
        "#/$defs/securityEvidenceReadiness"
    );
    let sec_ready = &repo_score["$defs"]["securityEvidenceReadiness"];
    assert!(sec_ready["properties"].get("artifact").is_some());
    assert!(repo_score["properties"].get("boundaries").is_some());
    assert_eq!(
        repo_score["properties"]["boundaries"]["$ref"],
        "#/$defs/boundariesReadiness"
    );
    let b_ready = &repo_score["$defs"]["boundariesReadiness"];
    assert!(b_ready["properties"].get("artifact").is_some());
    assert!(b_ready["properties"].get("reclassifications").is_some());
    assert!(repo_score["$defs"]
        .get("boundaryReclassification")
        .is_some());
}

#[test]
fn vibe_coverage_schemas_parse_and_source_validates() {
    let repo = repo_root();
    let source_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/vibe-coverage-source.schema.json")).unwrap(),
    )
    .unwrap();
    let report_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/vibe-coverage-report.schema.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(
        source_schema["$id"],
        "https://jankurai.dev/schemas/vibe-coverage-source.schema.json"
    );
    assert_eq!(
        report_schema["$id"],
        "https://jankurai.dev/schemas/vibe-coverage-report.schema.json"
    );
    assert_eq!(
        report_schema["properties"]["issues"]["items"]["$ref"],
        "vibe-coverage-source.schema.json#/$defs/issue"
    );
    assert_eq!(
        source_schema["properties"]["schema_version"]["const"],
        "1.9.0"
    );
    let issue_required = source_schema["$defs"]["issue"]["required"]
        .as_array()
        .unwrap();
    for key in [
        "canonical_group",
        "source_issue_kind",
        "detector_status",
        "evidence_status",
        "reviewed",
    ] {
        assert!(
            issue_required.iter().any(|value| value == key),
            "vibe coverage source issue must require `{key}`"
        );
    }
    for key in [
        "canonical_group_counts",
        "detector_status_counts",
        "evidence_status_counts",
    ] {
        assert!(
            report_schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == key),
            "vibe coverage report must require `{key}`"
        );
    }

    let source = fs::read_to_string(repo.join("agent/vibe-coverage.toml")).unwrap();
    let value = validation::validate_vibe_coverage_source_toml_text(&repo, &source).unwrap();
    validation::validate_value(&repo, ArtifactSchema::VibeCoverageSource, &value).unwrap();
}

#[test]
fn repair_run_schema_requires_execution_mode() {
    let repo = repo_root();
    let mut run: serde_json::Value = serde_json::json!({
        "schema_version": "1.0.0",
        "repo": ".",
        "plan": "plan.json",
        "generated_at": "0",
        "status": "complete",
        "dry_run": true,
        "auto_pr_requested": false,
        "auto_pr_status": "not-requested",
        "max_risk": "medium",
        "planned_packets": 0,
        "risk_summary": {"low": 0, "medium": 0, "high": 0, "critical": 0},
        "blocked_packets": [],
        "applied_edits": [],
        "skipped_edits": [],
        "files_written": [],
        "proof_lanes": [],
        "notes": []
    });
    run.as_object_mut().unwrap().remove("execution_mode");

    let error = jankurai::validation::validate_value(&repo, ArtifactSchema::RepairRun, &run)
        .expect_err("missing execution_mode should fail validation");
    let message = error.to_string();
    assert!(
        message.contains("execution_mode"),
        "expected execution_mode validation failure, got {message}"
    );
}

#[test]
fn repair_run_examples_validate_across_execution_modes() {
    let repo = repo_root();

    let base_run = || {
        serde_json::json!({
            "schema_version": "1.0.0",
            "repo": ".",
            "plan": "plan.json",
            "generated_at": "0",
            "status": "complete",
            "execution_mode": "dry-run",
            "dry_run": true,
            "auto_pr_requested": false,
            "auto_pr_status": "not-requested",
            "max_risk": "medium",
            "planned_packets": 1,
            "risk_summary": {"low": 1, "medium": 0, "high": 0, "critical": 0},
            "blocked_packets": [],
            "applied_edits": [],
            "skipped_edits": [],
            "files_written": [],
            "proof_lanes": ["audit"],
            "notes": ["fixture receipt"]
        })
    };

    let dry_run = base_run();
    validation::validate_value(&repo, ArtifactSchema::RepairRun, &dry_run).unwrap();

    let mut fixture_apply = base_run();
    fixture_apply["execution_mode"] = serde_json::json!("fixture-apply");
    fixture_apply["dry_run"] = serde_json::json!(false);
    fixture_apply["proof_evidence_index"] =
        serde_json::json!("target/jankurai/p13-fixture-evidence-index.json");
    fixture_apply["applied_edits"] = serde_json::json!([{
        "finding_fingerprint": "sha256:fixture",
        "path": "docs/notes.md",
        "apply_strategy": "append-text",
        "before_sha256": "sha256:before",
        "after_sha256": "sha256:after",
        "status": "applied"
    }]);
    fixture_apply["files_written"] = serde_json::json!(["docs/notes.md"]);
    validation::validate_value(&repo, ArtifactSchema::RepairRun, &fixture_apply).unwrap();

    let mut real_apply = base_run();
    real_apply["execution_mode"] = serde_json::json!("real-apply");
    real_apply["dry_run"] = serde_json::json!(false);
    real_apply["proof_evidence_index"] =
        serde_json::json!("target/jankurai/p13-real-evidence-index.json");
    real_apply["applied_edits"] = serde_json::json!([{
        "finding_fingerprint": "sha256:real",
        "path": "docs/notes.md",
        "apply_strategy": "append-text",
        "before_sha256": "sha256:before",
        "after_sha256": "sha256:after",
        "status": "applied"
    }]);
    real_apply["files_written"] = serde_json::json!(["docs/notes.md"]);
    validation::validate_value(&repo, ArtifactSchema::RepairRun, &real_apply).unwrap();

    let mut real_apply_git = real_apply.clone();
    real_apply_git["git_mutation"] = serde_json::json!({
        "status": "committed",
        "base_branch": "main",
        "head_branch": "jankurai/repair/abc123",
        "base_sha": "abc123",
        "head_sha": "def456",
        "commit_sha": "def456",
        "commit_title": "Repair docs",
        "files_committed": ["docs/notes.md"],
        "rollback_command": "git -C . reset --hard abc123",
        "remote": "origin",
        "pushed": true
    });
    validation::validate_value(&repo, ArtifactSchema::RepairRun, &real_apply_git).unwrap();

    let mut real_apply_github = real_apply_git.clone();
    real_apply_github["auto_pr_requested"] = serde_json::json!(true);
    real_apply_github["auto_pr_status"] = serde_json::json!("created");
    real_apply_github["github_pr"] = serde_json::json!({
        "status": "created",
        "draft": true,
        "url": "https://example.test/pr/42",
        "remote": "origin",
        "base_branch": "main",
        "head_branch": "jankurai/repair/abc123",
        "command": ["gh", "pr", "create", "--draft"]
    });
    validation::validate_value(&repo, ArtifactSchema::RepairRun, &real_apply_github).unwrap();
}

#[test]
fn update_plan_and_receipt_schemas_expose_optional_version_fields() {
    let repo = repo_root();
    let update_plan: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/update-plan.schema.json")).unwrap(),
    )
    .unwrap();
    let update_receipt: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/update-receipt.schema.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(
        update_plan["$id"],
        "https://jankurai.dev/schemas/update-plan.schema.json"
    );
    assert_eq!(
        update_receipt["$id"],
        "https://jankurai.dev/schemas/update-receipt.schema.json"
    );

    let plan_required = update_plan["required"].as_array().unwrap();
    assert!(
        !plan_required.iter().any(|key| key == "latest_version"),
        "update plan must keep latest_version optional"
    );
    for key in ["latest_version", "warnings", "actions", "artifacts"] {
        assert!(
            update_plan["properties"].get(key).is_some(),
            "update plan schema must expose optional `{key}`"
        );
    }
    for key in [
        "resolved_source",
        "reexec_command",
        "post_upgrade_score_command",
        "post_upgrade_score_mode",
        "post_upgrade_score_json",
        "post_upgrade_score_md",
    ] {
        assert!(
            update_plan["properties"].get(key).is_some(),
            "update plan schema must expose optional `{key}`"
        );
    }

    let receipt_required = update_receipt["required"].as_array().unwrap();
    assert!(
        !receipt_required.iter().any(|key| key == "latest_version"),
        "update receipt must keep latest_version optional"
    );
    for key in [
        "latest_version",
        "actions",
        "commands_run",
        "next_command",
        "residual_risk",
        "artifacts",
        "resolved_source",
        "reexec_command",
        "post_upgrade_score_command",
        "post_upgrade_score_mode",
        "post_upgrade_score_json",
        "post_upgrade_score_md",
    ] {
        assert!(
            update_receipt["properties"].get(key).is_some(),
            "update receipt schema must expose optional `{key}`"
        );
    }

    let plan = serde_json::json!({
        "schema_version": "1.0.0",
        "command": "jankurai update",
        "status": "outdated",
        "generated_at": "2026-05-06T00:00:00Z",
        "repo_root": repo.display().to_string(),
        "current_version": env!("CARGO_PKG_VERSION"),
        "latest_version": "0.8.12",
        "standard_version": STANDARD_VERSION,
        "auditor_version": AUDITOR_VERSION,
        "schema_contract_version": SCHEMA_VERSION,
        "paper_edition": PAPER_EDITION,
        "target_stack_id": TARGET_STACK_ID,
        "update_channel": "stable",
        "source": "auto",
        "offline": true,
        "client_start": false,
        "self_update_requested": true,
        "self_update_available": true,
        "install_state": "installed",
        "install_manifest_path": "agent/jankurai-install.toml",
        "state_path": "target/jankurai/update/state.json",
        "plan_path": "target/jankurai/update/update-plan.json",
        "md_path": "target/jankurai/update/update-plan.md",
        "resolved_source": {
            "requested_source": "auto",
            "resolved_source": "local",
            "source_url": "https://example.test/jankurai.git",
            "latest_version": "0.8.12",
            "install_command": [
                "cargo",
                "install",
                "--path",
                "crates/jankurai",
                "--locked",
                "--force"
            ],
            "install_root": ".",
            "reason": "local checkout is newer"
        },
        "reexec_command": "cargo run -p jankurai -- update . --offline",
        "post_upgrade_score_command": "cargo run -p jankurai -- score . --json agent/repo-score.json --md agent/repo-score.md",
        "post_upgrade_score_mode": "standard",
        "post_upgrade_score_json": "agent/repo-score.json",
        "post_upgrade_score_md": "agent/repo-score.md",
        "warnings": ["manual review queued"],
        "actions": [{
            "path": "agent/jankurai-install.toml",
            "action": "update",
            "reason": "refresh version pins",
            "current_hash": "sha256:current",
            "desired_hash": "sha256:desired",
            "merge_policy": "keep-existing"
        }],
        "artifacts": [
            "target/jankurai/update/update-plan.json",
            "target/jankurai/update/update-plan.md",
            "target/jankurai/update/state.json"
        ]
    });
    validation::validate_value(&repo, ArtifactSchema::UpdatePlan, &plan).unwrap();

    let receipt = serde_json::json!({
        "schema_version": "1.1.0",
        "command": "jankurai update",
        "created_at": "2026-05-06T00:00:00Z",
        "repo_root": repo.display().to_string(),
        "current_version": env!("CARGO_PKG_VERSION"),
        "latest_version": "0.8.12",
        "update_channel": "stable",
        "source": "auto",
        "self_update_requested": true,
        "self_update_applied": false,
        "repo_update_applied": true,
        "resolved_source": {
            "requested_source": "auto",
            "resolved_source": "local",
            "source_url": "https://example.test/jankurai.git",
            "latest_version": "0.8.12",
            "install_command": [
                "cargo",
                "install",
                "--path",
                "crates/jankurai",
                "--locked",
                "--force"
            ],
            "install_root": ".",
            "reason": "local checkout is newer"
        },
        "reexec_command": "cargo run -p jankurai -- update . --offline",
        "post_upgrade_score_command": "cargo run -p jankurai -- score . --json agent/repo-score.json --md agent/repo-score.md",
        "post_upgrade_score_mode": "standard",
        "post_upgrade_score_json": "agent/repo-score.json",
        "post_upgrade_score_md": "agent/repo-score.md",
        "actions": [{
            "path": "agent/jankurai-install.toml",
            "action": "update",
            "reason": "refresh version pins",
            "current_hash": "sha256:current",
            "desired_hash": "sha256:desired",
            "merge_policy": "keep-existing"
        }],
        "commands_run": ["cargo install --path crates/jankurai --locked --force"],
        "next_command": "cargo install --path crates/jankurai --locked --force",
        "residual_risk": ["local checkout still needs a follow-up refresh"],
        "artifacts": [
            "target/jankurai/update/update-plan.json",
            "target/jankurai/update/update-plan.md",
            "target/jankurai/update/state.json"
        ]
    });
    validation::validate_value(&repo, ArtifactSchema::UpdateReceipt, &receipt).unwrap();
}

#[test]
fn agent_control_plane_schemas_parse_and_repo_fixtures_validate() {
    let repo = repo_root();
    for (path, load) in [
        (
            "schemas/owner-map.schema.json",
            "https://jankurai.dev/schemas/owner-map.schema.json",
        ),
        (
            "schemas/test-map.schema.json",
            "https://jankurai.dev/schemas/test-map.schema.json",
        ),
        (
            "schemas/generated-zones.schema.json",
            "https://jankurai.dev/schemas/generated-zones.schema.json",
        ),
        (
            "schemas/proof-lanes.schema.json",
            "https://jankurai.dev/schemas/proof-lanes.schema.json",
        ),
        (
            "schemas/standard-version.schema.json",
            "https://jankurai.dev/schemas/standard-version.schema.json",
        ),
        (
            "schemas/audit-policy.schema.json",
            "https://jankurai.dev/schemas/audit-policy.schema.json",
        ),
        (
            "schemas/finding.schema.json",
            "https://jankurai.dev/schemas/finding.schema.json",
        ),
        (
            "schemas/repair-queue.schema.json",
            "https://jankurai.dev/schemas/repair-queue.schema.json",
        ),
    ] {
        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(repo.join(path)).unwrap()).unwrap();
        assert_eq!(v["$id"], load);
    }

    let owner = fs::read_to_string(repo.join("agent/owner-map.json")).unwrap();
    jankurai::validation::validate_owner_map_json_text(&repo, &owner).unwrap();
    let tests = fs::read_to_string(repo.join("agent/test-map.json")).unwrap();
    jankurai::validation::validate_test_map_json_text(&repo, &tests).unwrap();
    let zones = fs::read_to_string(repo.join("agent/generated-zones.toml")).unwrap();
    jankurai::validation::validate_generated_zones_toml_text(&repo, &zones).unwrap();
    let lanes = fs::read_to_string(repo.join("agent/proof-lanes.toml")).unwrap();
    jankurai::validation::validate_proof_lanes_toml_text(&repo, &lanes).unwrap();
    let lanes_json: toml::Value = toml::from_str(&lanes).unwrap();
    let first_lane = lanes_json["lane"].as_array().unwrap().first().unwrap();
    assert!(first_lane.get("command_id").is_some());
    assert!(first_lane.get("kind").is_some());
    assert!(first_lane.get("cost").is_some());
    assert!(first_lane.get("rules_covered").is_some());
    assert!(first_lane.get("required_artifacts").is_some());
    let standard = fs::read_to_string(repo.join("agent/standard-version.toml")).unwrap();
    jankurai::validation::validate_standard_version_toml_text(&repo, &standard).unwrap();
    let audit = fs::read_to_string(repo.join("agent/audit-policy.toml")).unwrap();
    jankurai::validation::validate_audit_policy_toml_text(&repo, &audit).unwrap();
}

#[test]
fn migration_prompt_verification_schema_validates() {
    let repo = repo_root();
    let value = serde_json::json!({
        "schema_version": "1.0.0",
        "command": "jankurai migrate verify-prompt",
        "status": "complete",
        "decision": "pass",
        "repo": ".",
        "document": "prompt.md",
        "claims_total": 1,
        "claims_verified": 1,
        "claims_invalid": 0,
        "claims_review": 0,
        "claims": [{
            "claim_type": "path-line",
            "claim": "src/lib.rs:1",
            "decision": "verified",
            "evidence": ["src/lib.rs:1"],
            "note": "ok"
        }]
    });
    validation::validate_value(&repo, ArtifactSchema::MigrationPromptVerification, &value).unwrap();
}

#[test]
fn migration_slice_risk_schema_validates() {
    let repo = repo_root();
    let value = serde_json::json!({
        "schema_version": "1.0.0",
        "command": "jankurai migrate slice-risk",
        "status": "complete",
        "decision": "block",
        "repo": ".",
        "plan": "plan.json",
        "slice_id": "demo",
        "slice_status": "candidate",
        "risk_level": "high",
        "check_env": true,
        "signals_total": 1,
        "critical_signals": 1,
        "high_signals": 0,
        "medium_signals": 0,
        "low_signals": 0,
        "env_checks": [{
            "name": "MODEL_HMAC_KEY",
            "present": false
        }],
        "signals": [{
            "kind": "torch-load-without-weights-only",
            "severity": "critical",
            "decision": "block",
            "evidence": ["src/model.py:10"],
            "recommendation": "add explicit weights_only=True",
            "path": "src/model.py",
            "line": 10
        }],
        "recommendations": ["add shadow/equivalence gate before cutover"]
    });
    validation::validate_value(&repo, ArtifactSchema::MigrationSliceRisk, &value).unwrap();
}
