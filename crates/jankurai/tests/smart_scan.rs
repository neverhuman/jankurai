use jankurai::audit::smart_scan::{
    decide, git_status_changed_files, SmartScanConfig, SmartScanDecision, SmartScanState,
};
use std::path::Path;
use tempfile::TempDir;

// ── helpers ───────────────────────────────────────────────────────────────────

fn empty_git_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.local"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    // Commit a .gitignore so target/ state files don't appear as untracked.
    std::fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();
    std::process::Command::new("git")
        .args(["add", ".gitignore"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "--quiet", "-m", "gitignore"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    dir
}

fn commit_file(repo: &Path, name: &str, contents: &str) -> String {
    std::fs::write(repo.join(name), contents).unwrap();
    std::process::Command::new("git")
        .args(["add", name])
        .current_dir(repo)
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "--quiet", "-m", "test"])
        .current_dir(repo)
        .status()
        .unwrap();
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn write_state(repo: &Path, state: &SmartScanState) {
    let path = repo.join("target/jankurai/audit-state.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_string(state).unwrap()).unwrap();
}

fn clean_state(commit: &str) -> SmartScanState {
    SmartScanState {
        schema_version: "1.0.0".into(),
        last_full_scan_at: 9_999_999_999,
        last_full_scan_commit: commit.to_string(),
        last_full_hard_findings: 0,
        last_full_caps: vec![],
        last_full_auditor_version: jankurai::model::AUDITOR_VERSION.into(),
    }
}

fn always_full() -> SmartScanConfig {
    SmartScanConfig { enabled: false, interval_secs: 3600, roulette_rate: 0.0 }
}

fn smart_cfg() -> SmartScanConfig {
    SmartScanConfig { enabled: true, interval_secs: 3600, roulette_rate: 0.0 }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[test]
fn missing_state_elects_full_scan() {
    let dir = empty_git_repo();
    commit_file(dir.path(), "x.txt", "hello");
    let decision = decide(dir.path(), &smart_cfg()).unwrap();
    assert!(matches!(decision, SmartScanDecision::Full { .. }));
}

#[test]
fn full_flag_elects_full_scan() {
    let dir = empty_git_repo();
    let sha = commit_file(dir.path(), "x.txt", "hello");
    write_state(dir.path(), &clean_state(&sha));
    let decision = decide(dir.path(), &always_full()).unwrap();
    assert!(matches!(decision, SmartScanDecision::Full { reason } if reason == "--full requested"));
}

#[test]
fn head_drift_elects_full_scan() {
    let dir = empty_git_repo();
    commit_file(dir.path(), "x.txt", "hello");
    write_state(dir.path(), &clean_state("deadbeef"));
    let decision = decide(dir.path(), &smart_cfg()).unwrap();
    assert!(
        matches!(decision, SmartScanDecision::Full { reason } if reason == "HEAD moved since last full scan")
    );
}

#[test]
fn dirty_state_hard_findings_elects_full() {
    let dir = empty_git_repo();
    let sha = commit_file(dir.path(), "x.txt", "hello");
    let mut state = clean_state(&sha);
    state.last_full_hard_findings = 3;
    write_state(dir.path(), &state);
    let decision = decide(dir.path(), &smart_cfg()).unwrap();
    assert!(
        matches!(decision, SmartScanDecision::Full { reason } if reason == "prior scan had findings")
    );
}

#[test]
fn dirty_state_caps_elects_full() {
    let dir = empty_git_repo();
    let sha = commit_file(dir.path(), "x.txt", "hello");
    let mut state = clean_state(&sha);
    state.last_full_caps = vec!["CAP_SCORE_70".into()];
    write_state(dir.path(), &state);
    let decision = decide(dir.path(), &smart_cfg()).unwrap();
    assert!(
        matches!(decision, SmartScanDecision::Full { reason } if reason == "prior scan had findings")
    );
}

#[test]
fn version_change_elects_full() {
    let dir = empty_git_repo();
    let sha = commit_file(dir.path(), "x.txt", "hello");
    let mut state = clean_state(&sha);
    state.last_full_auditor_version = "0.0.0".into();
    write_state(dir.path(), &state);
    let decision = decide(dir.path(), &smart_cfg()).unwrap();
    assert!(
        matches!(decision, SmartScanDecision::Full { reason } if reason == "auditor version changed")
    );
}

#[test]
fn timer_expired_elects_full() {
    let dir = empty_git_repo();
    let sha = commit_file(dir.path(), "x.txt", "hello");
    let mut state = clean_state(&sha);
    state.last_full_scan_at = 1_000_000;
    write_state(dir.path(), &state);
    let cfg = SmartScanConfig { enabled: true, interval_secs: 1, roulette_rate: 0.0 };
    let decision = decide(dir.path(), &cfg).unwrap();
    assert!(
        matches!(decision, SmartScanDecision::Full { reason } if reason == "interval elapsed")
    );
}

#[test]
fn roulette_rate_one_always_full() {
    let dir = empty_git_repo();
    let sha = commit_file(dir.path(), "x.txt", "hello");
    write_state(dir.path(), &clean_state(&sha));
    let cfg = SmartScanConfig { enabled: true, interval_secs: 0, roulette_rate: 1.0 };
    for _ in 0..10 {
        assert!(matches!(decide(dir.path(), &cfg).unwrap(), SmartScanDecision::Full { .. }));
    }
}

#[test]
fn clean_state_no_changes_elects_skip() {
    let dir = empty_git_repo();
    let sha = commit_file(dir.path(), "x.txt", "hello");
    write_state(dir.path(), &clean_state(&sha));
    let decision = decide(dir.path(), &smart_cfg()).unwrap();
    assert!(matches!(decision, SmartScanDecision::Skip));
}

#[test]
fn clean_state_with_changes_elects_fast() {
    let dir = empty_git_repo();
    let sha = commit_file(dir.path(), "x.txt", "hello");
    write_state(dir.path(), &clean_state(&sha));
    std::fs::write(dir.path().join("new_file.rs"), "fn foo() {}").unwrap();
    let decision = decide(dir.path(), &smart_cfg()).unwrap();
    assert!(matches!(decision, SmartScanDecision::Fast { paths } if !paths.is_empty()));
}

#[test]
fn git_status_changed_files_detects_untracked() {
    let dir = empty_git_repo();
    commit_file(dir.path(), "existing.rs", "fn a() {}");
    std::fs::write(dir.path().join("new.rs"), "fn b() {}").unwrap();
    let files = git_status_changed_files(dir.path()).unwrap();
    assert!(files.iter().any(|p| p.to_str().unwrap().contains("new.rs")));
}

#[test]
fn save_and_load_roundtrip() {
    let dir = empty_git_repo();
    let sha = commit_file(dir.path(), "x.txt", "hello");
    let state = clean_state(&sha);
    let path = dir.path().join("target/jankurai/audit-state.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_string_pretty(&state).unwrap()).unwrap();
    let loaded: SmartScanState =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(loaded.last_full_scan_commit, sha);
    assert_eq!(loaded.last_full_hard_findings, 0);
    assert!(loaded.last_full_caps.is_empty());
}

#[test]
fn copy_code_report_empty_has_skipped_status() {
    let report = jankurai::audit::copy_code::CopyCodeReport::empty();
    assert_eq!(report.status, "skipped");
    assert!(report.classes.is_empty());
    assert_eq!(report.summary.hard_classes, 0);
}
