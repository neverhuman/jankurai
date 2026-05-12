use jankurai::audit::run_audit;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_jankurai")
}

fn write_pass_repo(repo: &Path) {
    fs::write(
        repo.join("AGENTS.md"),
        "Read agent/JANKURAI_STANDARD.md first.\n",
    )
    .unwrap();
    fs::write(repo.join("README.md"), "# fixture\n").unwrap();
    fs::write(
        repo.join("Justfile"),
        "fast:\n    echo ok\ncheck:\n    echo ok\n",
    )
    .unwrap();
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/JANKURAI_STANDARD.md"),
        "Standard version: `0.9.0`\n",
    )
    .unwrap();
    fs::create_dir_all(repo.join("docs")).unwrap();
    fs::write(
        repo.join("docs/agent-native-standard.md"),
        "Standard version: `0.9.0`\n",
    )
    .unwrap();
}

fn write_report_baseline(repo: &Path, path: &Path) -> serde_json::Value {
    let report = run_audit(repo, &[]).unwrap();
    let value = serde_json::to_value(&report).unwrap();
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
    )
    .unwrap();
    value
}

fn ratchet(repo: &Path, baseline: &Path) -> std::process::Output {
    Command::new(binary_path())
        .arg("audit")
        .arg(repo)
        .arg("--mode")
        .arg("ratchet")
        .arg("--baseline")
        .arg(baseline)
        .arg("--json")
        .arg(repo.join("target/jankurai/repo-score.json"))
        .arg("--md")
        .arg(repo.join("target/jankurai/repo-score.md"))
        .arg("--no-score-history")
        .output()
        .unwrap()
}

#[test]
fn missing_baseline_in_ratchet_errors() {
    let repo = tempdir().unwrap();
    write_pass_repo(repo.path());

    let output = Command::new(binary_path())
        .arg("audit")
        .arg(repo.path())
        .arg("--mode")
        .arg("ratchet")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn baseline_missing_score_errors_instead_of_falling_back() {
    let repo = tempdir().unwrap();
    write_pass_repo(repo.path());
    let baseline = repo.path().join("baseline.json");
    fs::write(&baseline, "{}\n").unwrap();

    let output = ratchet(repo.path(), &baseline);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing required integer `score`"));
}

#[test]
fn score_regression_fails_ratchet() {
    let repo = tempdir().unwrap();
    write_pass_repo(repo.path());
    let baseline = repo.path().join("baseline.json");
    let mut value = write_report_baseline(repo.path(), &baseline);
    value["score"] = serde_json::json!(101);
    fs::write(
        &baseline,
        format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
    )
    .unwrap();

    let output = ratchet(repo.path(), &baseline);

    assert!(!output.status.success());
}

#[test]
fn new_cap_fails_ratchet_even_at_same_score() {
    let repo = tempdir().unwrap();
    write_pass_repo(repo.path());
    fs::remove_file(repo.path().join("Justfile")).unwrap();
    let baseline = repo.path().join("baseline.json");
    let mut value = write_report_baseline(repo.path(), &baseline);
    value["caps_applied"] = serde_json::json!([]);
    fs::write(
        &baseline,
        format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
    )
    .unwrap();

    let output = ratchet(repo.path(), &baseline);

    assert!(!output.status.success());
}

#[test]
fn policy_fingerprint_drift_fails_ratchet() {
    let repo = tempdir().unwrap();
    write_pass_repo(repo.path());
    let baseline = repo.path().join("baseline.json");
    let mut value = write_report_baseline(repo.path(), &baseline);
    value["policy_fingerprint"] = serde_json::json!(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111"
    );
    fs::write(
        &baseline,
        format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
    )
    .unwrap();

    let output = ratchet(repo.path(), &baseline);

    assert!(!output.status.success());
}
