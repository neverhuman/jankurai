use jankurai::audit::run_audit;
use std::fs;
use tempfile::tempdir;

fn thin_repo(dir: &std::path::Path) {
    fs::write(dir.join("README.md"), "# thin repo\n").unwrap();
}

fn minimal_valid_envelope() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "1.0.0",
        "standard_version": "0.5.0",
        "generated_at": "2026-05-02T12:00:00.000Z",
        "repo_root": "/tmp/x",
        "git_head": "abc123",
        "lane": "security",
        "wrapper": { "kind": "bash_script", "path": "tools/security-lane.sh", "strict": true },
        "exit_code": 0,
        "elapsed_ms": 42,
        "log_path": "target/jankurai/security/run.log",
        "policy": {
            "schema_version": "1.0.0",
            "profile": "ci",
            "enabled_tools": ["gitleaks"],
            "required_tools": ["gitleaks"],
            "advisory_tools": [],
            "require_one_of": [],
            "fail_lane_on": "high"
        },
        "commands": [
            {
                "label": "step-a",
                "shell_command": "bash tools/security-lane.sh",
                "status": "ran",
                "advisory": false,
                "required_by_policy": true,
                "blocking": false
            },
            {
                "label": "step-b",
                "shell_command": "echo skip",
                "status": "skipped",
                "advisory": false,
                "required_by_policy": true,
                "blocking": true
            },
            {
                "label": "step-c",
                "shell_command": "echo fail",
                "status": "failed",
                "advisory": true,
                "required_by_policy": false,
                "blocking": false
            }
        ]
    })
}

#[test]
fn audit_ingests_valid_security_evidence_summary() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai/security")).unwrap();
    let env = minimal_valid_envelope();
    fs::write(
        dir.path().join("target/jankurai/security/evidence.json"),
        serde_json::to_string_pretty(&env).unwrap(),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let art = report
        .security_evidence
        .artifact
        .as_ref()
        .expect("artifact summary");
    assert_eq!(art.path, "target/jankurai/security/evidence.json");
    assert_eq!(art.envelope_exit_code, 0);
    assert_eq!(art.elapsed_ms, 42);
    assert!(art.wrapper_strict);
    assert_eq!(art.commands_ran, 1);
    assert_eq!(art.commands_skipped, 1);
    assert_eq!(art.commands_failed, 1);
    assert_eq!(art.required_commands_skipped, 1);
    assert_eq!(art.required_commands_failed, 0);
    assert_eq!(art.blocking_commands, vec!["step-b".to_string()]);
    assert_eq!(art.profile, "ci");
    assert_eq!(
        art.generated_at.as_deref(),
        Some("2026-05-02T12:00:00.000Z")
    );
    assert_eq!(art.git_head.as_deref(), Some("abc123"));
}

#[test]
fn audit_invalid_security_evidence_leaves_artifact_none() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai/security")).unwrap();
    fs::write(
        dir.path().join("target/jankurai/security/evidence.json"),
        "{}\n",
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.security_evidence.artifact.is_none());
}

#[test]
fn audit_without_security_evidence_file_leaves_artifact_none() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.security_evidence.artifact.is_none());
}
