use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use jankurai::audit::run_audit;
use jankurai::model::Finding;
use jankurai::score_history::{
    append_score_history_with_options, compact_history_file, history_mirror_path_from_env,
    load_history_rows, repo_identity, restore_history_file, sanitize_remote_url,
    ScoreHistoryAppendOptions, ScoreHistoryPolicy,
};
use jankurai::validation::{self, ArtifactSchema};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn init_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("AGENTS.md"),
        "Read `agent/JANKURAI_STANDARD.md` first.\n",
    )
    .unwrap();
    fs::write(dir.path().join("README.md"), "# Repo\n").unwrap();
    fs::write(dir.path().join("Justfile"), "check:\n\tcargo test\n").unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("agent/JANKURAI_STANDARD.md"),
        "Standard version: `0.9.0`\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(
        dir.path().join("docs/agent-native-standard.md"),
        "Standard version: `0.9.0`\n",
    )
    .unwrap();

    Command::new("git")
        .arg("init")
        .arg(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "remote",
            "add",
            "origin",
            "https://user:token@example.com/org/repo.git",
        ])
        .output()
        .unwrap();
    dir
}

fn run_history_command(repo: &Path, args: &[&str]) -> (String, String) {
    let output = Command::new(binary_path())
        .current_dir(repo)
        .args(args)
        .output()
        .expect("run jankurai history command");
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn append_options(
    history: &Path,
    csv: Option<&Path>,
    mirror: Option<&Path>,
    required: bool,
) -> ScoreHistoryAppendOptions {
    ScoreHistoryAppendOptions {
        history_path: history.to_string_lossy().to_string(),
        csv_path: csv.map(|path| path.to_string_lossy().to_string()),
        mirror_path: mirror.map(|path| path.to_string_lossy().to_string()),
        mirror_required: required,
        policy: ScoreHistoryPolicy::default(),
    }
}

fn changed_finding() -> Finding {
    Finding {
        severity: "medium".into(),
        category: "test".into(),
        path: "src/lib.rs".into(),
        problem: "dummy".into(),
        agent_fix: "fix".into(),
        evidence: vec!["evidence".into()],
        check_id: "CHK-TEST".into(),
        hardness: "soft".into(),
        confidence: 0.5,
        evidence_kind: "text".into(),
        rerun_command: "cargo test".into(),
        fingerprint: "sha256:changed".into(),
        rule_id: Some("TEST-RULE".into()),
        tlr: None,
        lane: None,
        docs_url: None,
        owner: None,
        line: None,
        matched_term: None,
        reason: None,
    }
}

fn write_history_rows(path: &Path, rows: &[serde_json::Value]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let text = rows
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap()
        .join("\n");
    fs::write(path, format!("{text}\n")).unwrap();
}

#[test]
fn append_creates_valid_jsonl_and_csv_rows() {
    let repo = init_repo();
    let report = run_audit(repo.path(), &[]).unwrap();
    let history = repo.path().join("target/jankurai/history.jsonl");
    let csv = repo.path().join("target/jankurai/history.csv");

    let written = append_score_history_with_options(
        repo.path(),
        &report,
        "agent/repo-score.json",
        "agent/repo-score.md",
        append_options(&history, Some(&csv), None, false),
    )
    .unwrap();
    assert_eq!(written.as_deref(), Some(history.as_path()));

    let rows = load_history_rows(&history).unwrap();
    assert_eq!(rows.len(), 1);
    validation::validate_serializable(
        repo_root().as_path(),
        ArtifactSchema::ScoreHistoryEntry,
        &rows[0],
    )
    .unwrap();

    let csv_text = fs::read_to_string(&csv).unwrap();
    assert!(csv_text.starts_with("index,schema_version,standard_version"));
    assert!(csv_text.lines().count() >= 2);
    assert!(history.exists());
}

#[test]
fn history_latest_and_export_emit_stable_contracts() {
    let repo = init_repo();
    let report = run_audit(repo.path(), &[]).unwrap();
    let history = repo.path().join("target/jankurai/history.jsonl");
    append_score_history_with_options(
        repo.path(),
        &report,
        "agent/repo-score.json",
        "agent/repo-score.md",
        append_options(&history, None, None, false),
    )
    .unwrap();

    let latest_out = repo.path().join("target/jankurai/latest.json");
    run_history_command(
        repo.path(),
        &[
            "history",
            "latest",
            "--history",
            history.to_str().unwrap(),
            "--out",
            latest_out.to_str().unwrap(),
        ],
    );
    let latest = read_json(&latest_out);
    validation::validate_value(
        repo_root().as_path(),
        ArtifactSchema::ScoreHistoryEntry,
        &latest,
    )
    .unwrap();
    assert_eq!(latest["schema_version"], "1.1.0");

    let export_json = repo.path().join("target/jankurai/export.json");
    let export_md = repo.path().join("target/jankurai/export.md");
    run_history_command(
        repo.path(),
        &[
            "history",
            "export",
            "--history",
            history.to_str().unwrap(),
            "--window",
            "1",
            "--out",
            export_json.to_str().unwrap(),
            "--md",
            export_md.to_str().unwrap(),
        ],
    );
    let export = read_json(&export_json);
    validation::validate_value(
        repo_root().as_path(),
        ArtifactSchema::ScoreHistoryExport,
        &export,
    )
    .unwrap();
    assert!(fs::read_to_string(&export_md)
        .unwrap()
        .starts_with("# jankurai History Export"));
}

#[test]
fn consecutive_equivalent_rows_are_not_duplicated() {
    let repo = init_repo();
    let report = run_audit(repo.path(), &[]).unwrap();
    let history = repo.path().join("target/jankurai/history.jsonl");

    append_score_history_with_options(
        repo.path(),
        &report,
        "agent/repo-score.json",
        "agent/repo-score.md",
        append_options(&history, None, None, false),
    )
    .unwrap();
    append_score_history_with_options(
        repo.path(),
        &report,
        "agent/repo-score.json",
        "agent/repo-score.md",
        append_options(&history, None, None, false),
    )
    .unwrap();

    let rows = load_history_rows(&history).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn changed_score_and_finding_counts_create_new_rows() {
    let repo = init_repo();
    let report = run_audit(repo.path(), &[]).unwrap();
    let history = repo.path().join("target/jankurai/history.jsonl");

    append_score_history_with_options(
        repo.path(),
        &report,
        "agent/repo-score.json",
        "agent/repo-score.md",
        append_options(&history, None, None, false),
    )
    .unwrap();

    let mut changed_score = report.clone();
    changed_score.score += 1;
    changed_score.raw_score += 1;
    append_score_history_with_options(
        repo.path(),
        &changed_score,
        "agent/repo-score.json",
        "agent/repo-score.md",
        append_options(&history, None, None, false),
    )
    .unwrap();

    let mut changed_findings = report.clone();
    changed_findings.findings.push(changed_finding());
    if let Some(decision) = changed_findings.decision.as_mut() {
        decision.hard_findings += 1;
        decision.status = "review".into();
    }
    append_score_history_with_options(
        repo.path(),
        &changed_findings,
        "agent/repo-score.json",
        "agent/repo-score.md",
        append_options(&history, None, None, false),
    )
    .unwrap();

    let rows = load_history_rows(&history).unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn max_rows_and_max_bytes_drop_oldest_rows() {
    let repo = init_repo();
    let history = repo.path().join("target/jankurai/manual-history.jsonl");
    let row_a = serde_json::json!({
        "schema_version": "1.1.0",
        "standard_version": "0.9.0",
        "auditor_version": "1.2.0",
        "generated_at": "1",
        "run_id": "1",
        "repo_id": "sha256:repo",
        "repo_remote": "https://example.com/repo.git",
        "branch": "main",
        "commit": "a",
        "dirty_worktree": false,
        "scope": "full",
        "changed_paths": ["README.md"],
        "score": 10,
        "raw_score": 10,
        "finding_count": 1,
        "hard_findings": 1,
        "soft_findings": 0,
        "decision": "review",
        "minimum_score": 85,
        "caps_applied": ["cap-a"],
        "report_fingerprint": "sha256:a",
        "input_fingerprint": "sha256:ia",
        "policy_fingerprint": "sha256:pa",
        "repo_score_json_path": "agent/repo-score.json",
        "repo_score_md_path": "agent/repo-score.md"
    });
    let row_b = serde_json::json!({
        "schema_version": "1.1.0",
        "standard_version": "0.9.0",
        "auditor_version": "1.2.0",
        "generated_at": "2",
        "run_id": "2",
        "repo_id": "sha256:repo",
        "repo_remote": "https://example.com/repo.git",
        "branch": "main",
        "commit": "b",
        "dirty_worktree": false,
        "scope": "full",
        "changed_paths": ["README.md"],
        "score": 11,
        "raw_score": 11,
        "finding_count": 1,
        "hard_findings": 1,
        "soft_findings": 0,
        "decision": "review",
        "minimum_score": 85,
        "caps_applied": ["cap-a"],
        "report_fingerprint": "sha256:b",
        "input_fingerprint": "sha256:ib",
        "policy_fingerprint": "sha256:pb",
        "repo_score_json_path": "agent/repo-score.json",
        "repo_score_md_path": "agent/repo-score.md"
    });
    let row_c = serde_json::json!({
        "schema_version": "1.1.0",
        "standard_version": "0.9.0",
        "auditor_version": "1.2.0",
        "generated_at": "3",
        "run_id": "3",
        "repo_id": "sha256:repo",
        "repo_remote": "https://example.com/repo.git",
        "branch": "main",
        "commit": "c",
        "dirty_worktree": false,
        "scope": "full",
        "changed_paths": ["README.md"],
        "score": 12,
        "raw_score": 12,
        "finding_count": 1,
        "hard_findings": 1,
        "soft_findings": 0,
        "decision": "review",
        "minimum_score": 85,
        "caps_applied": ["cap-a"],
        "report_fingerprint": "sha256:c",
        "input_fingerprint": "sha256:ic",
        "policy_fingerprint": "sha256:pc",
        "repo_score_json_path": "agent/repo-score.json",
        "repo_score_md_path": "agent/repo-score.md"
    });
    write_history_rows(&history, &[row_a, row_b, row_c]);

    let compacted = compact_history_file(&history, 2, 10_000).unwrap();
    assert_eq!(compacted.len(), 2);
    let rows = load_history_rows(&history).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].generated_at, "2");
    assert_eq!(rows[1].generated_at, "3");

    let serialized = fs::read_to_string(&history).unwrap();
    assert!(serialized.ends_with('\n'));

    let bytes_limited =
        compact_history_file(&history, 2, serialized.lines().last().unwrap().len() + 2).unwrap();
    assert_eq!(bytes_limited.len(), 1);
}

#[test]
fn malformed_local_jsonl_returns_line_numbered_error() {
    let repo = init_repo();
    let history = repo.path().join("target/jankurai/bad.jsonl");
    if let Some(parent) = history.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        &history,
        serde_json::json!({
            "schema_version": "1.1.0",
            "generated_at": "1",
            "run_id": "1",
            "branch": "main",
            "commit": "abc123",
            "dirty_worktree": false,
            "scope": "full",
            "changed_paths": ["README.md"],
            "score": 1,
            "raw_score": 1,
            "finding_count": 0,
            "hard_findings": 0,
            "soft_findings": 0,
            "decision": "pass",
            "minimum_score": 85,
            "caps_applied": [],
            "report_fingerprint": "sha256:test",
            "repo_score_json_path": "agent/repo-score.json",
            "repo_score_md_path": "agent/repo-score.md"
        })
        .to_string()
            + "\nnot-json\n",
    )
    .unwrap();
    let err = load_history_rows(&history).unwrap_err().to_string();
    assert!(err.contains("line 2"), "{err}");
}

#[test]
fn mirror_sink_receives_rows_and_missing_mirror_is_advisory_only() {
    let repo = init_repo();
    let report = run_audit(repo.path(), &[]).unwrap();
    let history = repo.path().join("target/jankurai/history.jsonl");
    let mirror = repo.path().join("target/jankurai/mirror.jsonl");

    append_score_history_with_options(
        repo.path(),
        &report,
        "agent/repo-score.json",
        "agent/repo-score.md",
        append_options(&history, None, Some(&mirror), false),
    )
    .unwrap();
    let mirror_rows = load_history_rows(&mirror).unwrap();
    assert_eq!(mirror_rows.len(), 1);

    let blocked_parent = repo.path().join("blocked-parent");
    fs::write(&blocked_parent, "file-not-dir").unwrap();
    let missing_mirror = blocked_parent.join("mirror.jsonl");
    let result = append_score_history_with_options(
        repo.path(),
        &report,
        "agent/repo-score.json",
        "agent/repo-score.md",
        append_options(&history, None, Some(&missing_mirror), false),
    );
    assert!(result.is_ok());
}

#[test]
fn mirror_required_mode_fails() {
    let repo = init_repo();
    let report = run_audit(repo.path(), &[]).unwrap();
    let history = repo.path().join("target/jankurai/history.jsonl");
    let blocked_parent = repo.path().join("blocked-parent");
    fs::write(&blocked_parent, "file-not-dir").unwrap();
    let mirror = blocked_parent.join("mirror.jsonl");

    let result = append_score_history_with_options(
        repo.path(),
        &report,
        "agent/repo-score.json",
        "agent/repo-score.md",
        append_options(&history, None, Some(&mirror), true),
    );
    assert!(result.is_err());
}

#[test]
fn restore_filters_by_current_repo_id_and_writes_compact_local_history() {
    let repo = init_repo();
    let current_repo_id = repo_identity(repo.path()).repo_id;
    let mirror = repo.path().join("target/jankurai/mirror.jsonl");
    let out = repo.path().join("target/jankurai/restored.jsonl");

    let rows = vec![
        serde_json::json!({
            "schema_version": "1.1.0",
            "standard_version": "0.9.0",
            "auditor_version": "1.2.0",
            "generated_at": "1",
            "run_id": "1",
            "repo_id": current_repo_id,
            "repo_remote": null,
            "branch": "main",
            "commit": "a",
            "dirty_worktree": false,
            "scope": "full",
            "changed_paths": ["README.md"],
            "score": 10,
            "raw_score": 10,
            "finding_count": 1,
            "hard_findings": 1,
            "soft_findings": 0,
            "decision": "review",
            "minimum_score": 85,
            "caps_applied": ["cap-a"],
            "report_fingerprint": "sha256:a",
            "input_fingerprint": "sha256:ia",
            "policy_fingerprint": "sha256:pa",
            "repo_score_json_path": "agent/repo-score.json",
            "repo_score_md_path": "agent/repo-score.md"
        }),
        serde_json::json!({
            "schema_version": "1.1.0",
            "standard_version": "0.9.0",
            "auditor_version": "1.2.0",
            "generated_at": "2",
            "run_id": "2",
            "repo_id": "sha256:other",
            "repo_remote": null,
            "branch": "main",
            "commit": "b",
            "dirty_worktree": false,
            "scope": "full",
            "changed_paths": ["README.md"],
            "score": 11,
            "raw_score": 11,
            "finding_count": 1,
            "hard_findings": 1,
            "soft_findings": 0,
            "decision": "review",
            "minimum_score": 85,
            "caps_applied": ["cap-a"],
            "report_fingerprint": "sha256:b",
            "input_fingerprint": "sha256:ib",
            "policy_fingerprint": "sha256:pb",
            "repo_score_json_path": "agent/repo-score.json",
            "repo_score_md_path": "agent/repo-score.md"
        }),
        serde_json::json!({
            "schema_version": "1.1.0",
            "standard_version": "0.9.0",
            "auditor_version": "1.2.0",
            "generated_at": "3",
            "run_id": "3",
            "repo_id": current_repo_id,
            "repo_remote": null,
            "branch": "main",
            "commit": "c",
            "dirty_worktree": false,
            "scope": "full",
            "changed_paths": ["README.md"],
            "score": 12,
            "raw_score": 12,
            "finding_count": 1,
            "hard_findings": 1,
            "soft_findings": 0,
            "decision": "review",
            "minimum_score": 85,
            "caps_applied": ["cap-a"],
            "report_fingerprint": "sha256:c",
            "input_fingerprint": "sha256:ic",
            "policy_fingerprint": "sha256:pc",
            "repo_score_json_path": "agent/repo-score.json",
            "repo_score_md_path": "agent/repo-score.md"
        }),
    ];
    write_history_rows(&mirror, &rows);

    let restored = restore_history_file(&mirror, &current_repo_id, &out, 10, 10_000).unwrap();
    assert_eq!(restored.len(), 2);
    let loaded = load_history_rows(&out).unwrap();
    assert_eq!(loaded.len(), 2);
    assert!(loaded
        .iter()
        .all(|row| row.repo_id.as_deref() == Some(current_repo_id.as_str())));
}

#[test]
fn sanitized_remote_url_strips_credentials_and_repo_identity_uses_origin() {
    let repo = init_repo();
    let sanitized = sanitize_remote_url("https://user:token@example.com/org/repo.git");
    assert_eq!(sanitized, "https://example.com/org/repo.git");
    let identity = repo_identity(repo.path());
    assert!(identity.repo_id.starts_with("sha256:"));
    assert_eq!(
        identity.repo_remote.as_deref(),
        Some("https://example.com/org/repo.git")
    );
}

#[test]
fn history_mirror_env_helper_trims_values() {
    let policy = ScoreHistoryPolicy {
        mirror_env: "JANKURAI_TEST_HISTORY_MIRROR".into(),
        ..ScoreHistoryPolicy::default()
    };
    std::env::set_var(
        "JANKURAI_TEST_HISTORY_MIRROR",
        "  target/jankurai/mirror.jsonl  ",
    );
    let value = history_mirror_path_from_env(&policy);
    std::env::remove_var("JANKURAI_TEST_HISTORY_MIRROR");
    assert_eq!(value.as_deref(), Some("target/jankurai/mirror.jsonl"));
}
