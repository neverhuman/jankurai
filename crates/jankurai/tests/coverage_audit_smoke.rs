use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_jankurai")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn fixture(name: &str) -> PathBuf {
    repo_root()
        .join("crates/jankurai/tests/fixtures/coverage")
        .join(name)
}

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn seed_changed_repo(repo: &Path) -> String {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::create_dir_all(repo.join("crates/demo/src")).unwrap();
    git(repo, &["init"]);
    git(repo, &["config", "user.email", "jankurai@example.test"]);
    git(repo, &["config", "user.name", "Jankurai Test"]);
    fs::write(
        repo.join("crates/demo/src/lib.rs"),
        "pub fn value() -> i32 {\n    1\n}\n",
    )
    .unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-m", "base"]);
    let base = git(repo, &["rev-parse", "HEAD"]);
    fs::write(
        repo.join("crates/demo/src/lib.rs"),
        "pub fn value() -> i32 {\n    2\n}\n",
    )
    .unwrap();
    base
}

fn write_config(repo: &Path, body: &str) {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(repo.join("agent/coverage-sources.toml"), body).unwrap();
}

fn copy_fixture(repo: &Path, name: &str, dest: &str) {
    let dest_path = repo.join(dest);
    fs::create_dir_all(dest_path.parent().unwrap()).unwrap();
    fs::copy(fixture(name), dest_path).unwrap();
}

fn run_coverage(repo: &Path, extra: &[&str]) -> (Output, Value) {
    let out_dir = repo.join("target/jankurai/coverage");
    fs::create_dir_all(&out_dir).unwrap();
    let json = out_dir.join("coverage-audit.json");
    let md = out_dir.join("coverage-audit.md");
    let output = Command::new(binary_path())
        .arg("coverage")
        .arg("audit")
        .arg(repo)
        .arg("--config")
        .arg("agent/coverage-sources.toml")
        .arg("--json")
        .arg(&json)
        .arg("--md")
        .arg(&md)
        .args(extra)
        .current_dir(repo)
        .output()
        .expect("spawn coverage audit");
    let value = if json.is_file() {
        serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap()
    } else {
        Value::Null
    };
    (output, value)
}

fn lcov_config(mode: &str, artifact: &str, extra: &str) -> String {
    format!(
        r#"version = 1

[[source]]
id = "fixture-lcov"
kind = "line_coverage"
format = "lcov"
mode = "{mode}"
owner = "tools"
lane = "coverage-audit"
artifacts = ["{artifact}"]
applies_to = ["crates/**/*.rs"]
rules = ["HLT-008-FALSE-GREEN-RISK"]
hard_changed_line_coverage = 0.90
{extra}
"#
    )
}

#[test]
fn valid_lcov_required_source_produces_no_findings() {
    let repo = tempdir().unwrap();
    let base = seed_changed_repo(repo.path());
    copy_fixture(repo.path(), "valid_lcov.info", "coverage/lcov.info");
    write_config(
        repo.path(),
        &lcov_config("required", "coverage/lcov.info", ""),
    );

    let (output, audit) = run_coverage(repo.path(), &["--changed-from", &base]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(audit["summary"]["hard_findings"], 0);
    assert!(audit["findings"].as_array().unwrap().is_empty());
}

#[test]
fn missing_required_artifact_for_changed_path_is_actionable_hlt008() {
    let repo = tempdir().unwrap();
    let base = seed_changed_repo(repo.path());
    write_config(
        repo.path(),
        &lcov_config("required", "coverage/missing.info", ""),
    );

    let (output, audit) = run_coverage(repo.path(), &["--changed-from", &base]);
    assert!(output.status.success());
    assert_eq!(audit["summary"]["hard_findings"], 1);
    assert_eq!(audit["findings"][0]["rule_id"], "HLT-008-FALSE-GREEN-RISK");
    assert!(audit["findings"][0]["repair"]
        .as_str()
        .unwrap()
        .contains("rerun"));
}

#[test]
fn missing_advisory_artifact_is_not_hard() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("agent")).unwrap();
    write_config(
        repo.path(),
        &lcov_config("advisory", "coverage/missing.info", ""),
    );

    let (output, audit) = run_coverage(repo.path(), &[]);
    assert!(output.status.success());
    assert_eq!(audit["summary"]["hard_findings"], 0);
}

#[test]
fn malformed_lcov_produces_parser_finding() {
    let repo = tempdir().unwrap();
    copy_fixture(repo.path(), "malformed_lcov.info", "coverage/lcov.info");
    write_config(
        repo.path(),
        &lcov_config("required", "coverage/lcov.info", ""),
    );

    let (output, audit) = run_coverage(repo.path(), &[]);
    assert!(output.status.success());
    assert_eq!(audit["summary"]["hard_findings"], 1);
    assert!(audit["findings"][0]["message"]
        .as_str()
        .unwrap()
        .contains("could not be parsed"));
}

#[test]
fn uncovered_changed_lcov_line_routes_to_hlt008() {
    let repo = tempdir().unwrap();
    let base = seed_changed_repo(repo.path());
    copy_fixture(
        repo.path(),
        "uncovered_changed_line.info",
        "coverage/lcov.info",
    );
    write_config(
        repo.path(),
        &lcov_config("required", "coverage/lcov.info", ""),
    );

    let (output, audit) = run_coverage(repo.path(), &["--changed-from", &base]);
    assert!(output.status.success());
    assert_eq!(audit["findings"][0]["rule_id"], "HLT-008-FALSE-GREEN-RISK");
    assert_eq!(audit["findings"][0]["line"], 2);
}

#[test]
fn low_total_lcov_is_soft_when_changed_line_is_covered() {
    let repo = tempdir().unwrap();
    let base = seed_changed_repo(repo.path());
    fs::create_dir_all(repo.path().join("coverage")).unwrap();
    fs::write(
        repo.path().join("coverage/lcov.info"),
        "TN:\nSF:crates/demo/src/lib.rs\nDA:1,1\nDA:2,1\nDA:3,0\nDA:4,0\nend_of_record\n",
    )
    .unwrap();
    write_config(
        repo.path(),
        &lcov_config(
            "required",
            "coverage/lcov.info",
            "soft_total_line_coverage = 0.90",
        ),
    );

    let (output, audit) = run_coverage(repo.path(), &["--changed-from", &base]);
    assert!(output.status.success());
    assert_eq!(audit["summary"]["hard_findings"], 0);
    assert_eq!(audit["summary"]["soft_findings"], 1);
}

#[test]
fn cargo_mutants_survivor_and_clean_reports_normalize() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "cargo_mutants_survivor.json",
        "coverage/mutants.json",
    );
    write_config(
        repo.path(),
        r#"version = 1
[[source]]
id = "mutants"
kind = "mutation"
format = "cargo-mutants-json"
mode = "required"
artifacts = ["coverage/mutants.json"]
rules = ["HLT-008-FALSE-GREEN-RISK"]
hard_survivors_on_changed_paths = 1
"#,
    );
    let (output, audit) = run_coverage(repo.path(), &[]);
    assert!(output.status.success());
    assert_eq!(audit["findings"][0]["rule_id"], "HLT-008-FALSE-GREEN-RISK");

    copy_fixture(
        repo.path(),
        "cargo_mutants_clean.json",
        "coverage/mutants.json",
    );
    let (output, audit) = run_coverage(repo.path(), &[]);
    assert!(output.status.success());
    assert!(audit["findings"].as_array().unwrap().is_empty());
}

#[test]
fn stryker_survivor_routes_to_hlt008() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "stryker_survivor.json",
        "coverage/stryker.json",
    );
    write_config(
        repo.path(),
        r#"version = 1
[[source]]
id = "stryker"
kind = "mutation"
format = "stryker-json"
mode = "required"
artifacts = ["coverage/stryker.json"]
rules = ["HLT-008-FALSE-GREEN-RISK"]
hard_survivors_on_changed_paths = 1
"#,
    );
    let (output, audit) = run_coverage(repo.path(), &[]);
    assert!(output.status.success());
    assert_eq!(audit["findings"][0]["rule_id"], "HLT-008-FALSE-GREEN-RISK");
}

#[test]
fn trivy_and_hadolint_route_to_existing_security_rules() {
    let repo = tempdir().unwrap();
    copy_fixture(repo.path(), "trivy_critical.json", "coverage/trivy.json");
    write_config(
        repo.path(),
        r#"version = 1
[[source]]
id = "trivy"
kind = "supply_chain"
format = "trivy-json"
mode = "required"
artifacts = ["coverage/trivy.json"]
rules = ["HLT-016-SUPPLY-CHAIN-DRIFT", "HLT-032-DOCKER-BAD-BEHAVIOR"]
hard_critical_vulnerabilities = 1
"#,
    );
    let (output, audit) = run_coverage(repo.path(), &[]);
    assert!(output.status.success());
    assert_eq!(
        audit["findings"][0]["rule_id"],
        "HLT-016-SUPPLY-CHAIN-DRIFT"
    );

    copy_fixture(
        repo.path(),
        "hadolint_warning.json",
        "coverage/hadolint.json",
    );
    write_config(
        repo.path(),
        r#"version = 1
[[source]]
id = "hadolint"
kind = "container"
format = "hadolint-json"
mode = "required"
artifacts = ["coverage/hadolint.json"]
rules = ["HLT-032-DOCKER-BAD-BEHAVIOR"]
"#,
    );
    let (output, audit) = run_coverage(repo.path(), &[]);
    assert!(output.status.success());
    assert_eq!(audit["summary"]["hard_findings"], 0);
    assert_eq!(
        audit["findings"][0]["rule_id"],
        "HLT-032-DOCKER-BAD-BEHAVIOR"
    );
}

#[test]
fn generic_advisory_summary_cannot_import_hard_findings() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("coverage")).unwrap();
    fs::write(
        repo.path().join("coverage/generic.json"),
        r#"{"status":"fail","metrics":{"demo":1},"findings":[{"rule_id":"HLT-008-FALSE-GREEN-RISK","severity":"high","message":"demo","path":"src/lib.rs","repair":"add a real test","evidence":["demo"]}]}"#,
    )
    .unwrap();
    write_config(
        repo.path(),
        r#"version = 1
[[source]]
id = "generic"
kind = "jankurai_artifact"
format = "generic-json-summary"
mode = "advisory"
artifacts = ["coverage/generic.json"]
"#,
    );
    let (output, audit) = run_coverage(repo.path(), &[]);
    assert!(output.status.success());
    assert_eq!(audit["summary"]["hard_findings"], 0);
    assert_eq!(audit["findings"][0]["severity"], "medium");
}

#[test]
fn bounded_artifact_and_path_escape_failures_are_reported() {
    let repo = tempdir().unwrap();
    copy_fixture(repo.path(), "valid_lcov.info", "coverage/lcov.info");
    write_config(
        repo.path(),
        &lcov_config("advisory", "coverage/lcov.info", ""),
    );
    let (output, audit) = run_coverage(repo.path(), &["--max-artifact-bytes", "5"]);
    assert!(output.status.success());
    assert!(audit["findings"][0]["evidence"][0]
        .as_str()
        .unwrap()
        .contains("max_artifact_bytes"));

    write_config(repo.path(), &lcov_config("advisory", "../outside.info", ""));
    let (output, _) = run_coverage(repo.path(), &[]);
    assert!(!output.status.success());
}

#[test]
fn strict_exits_nonzero_on_hard_findings_but_default_writes_artifacts() {
    let repo = tempdir().unwrap();
    let base = seed_changed_repo(repo.path());
    write_config(
        repo.path(),
        &lcov_config("required", "coverage/missing.info", ""),
    );

    let (output, audit) = run_coverage(repo.path(), &["--changed-from", &base]);
    assert!(output.status.success());
    assert_eq!(audit["summary"]["hard_findings"], 1);

    let (output, audit) = run_coverage(repo.path(), &["--changed-from", &base, "--strict"]);
    assert!(!output.status.success());
    assert_eq!(audit["summary"]["hard_findings"], 1);
}
