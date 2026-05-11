use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_jankurai")
}

fn write_base_repo(repo: &Path) {
    fs::write(
        repo.join("AGENTS.md"),
        "Read agent/JANKURAI_STANDARD.md first.\n",
    )
    .unwrap();
    fs::write(repo.join("README.md"), "# fixture\n").unwrap();
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/JANKURAI_STANDARD.md"),
        "Standard version: `0.8.0`\n",
    )
    .unwrap();
    fs::create_dir_all(repo.join("docs")).unwrap();
    fs::write(
        repo.join("docs/agent-native-standard.md"),
        "Standard version: `0.8.0`\n",
    )
    .unwrap();
}

fn audit(repo: &Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(binary_path());
    cmd.arg("audit")
        .arg(repo)
        .arg("--json")
        .arg(repo.join("target/jankurai/repo-score.json"))
        .arg("--md")
        .arg(repo.join("target/jankurai/repo-score.md"))
        .arg("--no-score-history");
    for arg in extra {
        cmd.arg(arg);
    }
    cmd.output().unwrap()
}

#[test]
fn standard_mode_fails_closed_but_writes_artifacts() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());

    let output = audit(repo.path(), &["--mode", "standard", "--fail-under", "0"]);

    assert!(!output.status.success());
    assert!(repo
        .path()
        .join("target/jankurai/repo-score.json")
        .is_file());
    assert!(repo.path().join("target/jankurai/repo-score.md").is_file());
}

#[test]
fn advisory_mode_keeps_failed_decision_nonblocking() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());

    let output = audit(repo.path(), &["--mode", "advisory", "--fail-under", "0"]);

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.path().join("target/jankurai/repo-score.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(value["decision"]["status"], "advisory");
}

#[test]
fn fail_on_policy_controls_hard_findings() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());

    let critical_only = audit(
        repo.path(),
        &[
            "--mode",
            "standard",
            "--fail-under",
            "0",
            "--fail-on",
            "critical",
        ],
    );
    assert!(critical_only.status.success());

    let medium_repo = tempdir().unwrap();
    fs::write(medium_repo.path().join("README.md"), "# fixture\n").unwrap();
    let medium = audit(
        medium_repo.path(),
        &[
            "--mode",
            "standard",
            "--fail-under",
            "0",
            "--fail-on",
            "medium",
        ],
    );
    assert!(!medium.status.success());
}

#[test]
fn invalid_policy_severity_fails_loading() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    fs::write(
        repo.path().join("agent/audit-policy.toml"),
        "minimum_score = 0\nfail_on = [\"severe\"]\nadvisory_on = []\n",
    )
    .unwrap();

    let output = audit(repo.path(), &["--mode", "standard"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid audit policy severity"));
}

#[test]
fn isolated_empty_repo_report_includes_ratchet_score_delta() {
    let repo = tempdir().unwrap();
    let home = tempdir().unwrap();
    let config = tempdir().unwrap();
    let cache = tempdir().unwrap();

    let output = Command::new(binary_path())
        .arg("audit")
        .arg(repo.path())
        .arg("--mode")
        .arg("advisory")
        .arg("--json")
        .arg(repo.path().join("repo-score.json"))
        .arg("--md")
        .arg(repo.path().join("repo-score.md"))
        .arg("--no-score-history")
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", config.path())
        .env("XDG_CACHE_HOME", cache.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(repo.path().join("repo-score.json")).unwrap())
            .unwrap();
    assert_eq!(value["decision"]["ratchet"]["score_delta"], 0);
}
