use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn run_doctor(repo: &PathBuf) -> (std::process::Output, tempfile::TempDir, PathBuf) {
    let out_dir = tempdir().unwrap();
    let json_path = out_dir.path().join("doctor.json");
    let mut cmd = Command::new(binary_path());
    cmd.arg("doctor")
        .arg(repo)
        .arg("--fail-on")
        .arg("critical")
        .arg("--json")
        .arg(&json_path);
    let output = cmd.output().unwrap();
    (output, out_dir, json_path)
}

fn severity_discipline_hits(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    report
        .as_array()
        .unwrap()
        .iter()
        .filter(|diagnostic| diagnostic["check_id"] == "severity-discipline")
        .collect()
}

#[test]
fn doctor_flags_unjustified_catastrophic_prose() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(
        repo.path().join("docs/notes.md"),
        "catastrophic outage expected during cutover\n",
    )
    .unwrap();

    let (output, _dir, json_path) = run_doctor(&repo.path().to_path_buf());
    assert!(output.status.success());

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert!(!severity_discipline_hits(&report).is_empty());
}

#[test]
fn doctor_suppresses_severity_trailers_with_env_prerequisite_blockers() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(
        repo.path().join("docs/notes.md"),
        "high reliability risk during cutover\nSeverity-Justified: yes\nBlocker-Type: env-prerequisite\n",
    )
    .unwrap();

    let (output, _dir, json_path) = run_doctor(&repo.path().to_path_buf());
    assert!(output.status.success());

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert!(severity_discipline_hits(&report).is_empty());
}

#[test]
fn doctor_ignores_toml_postmortems_as_prose() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join(".jankurai/postmortems")).unwrap();
    fs::write(
        repo.path().join(".jankurai/postmortems/alpha.toml"),
        "schema_version = \"1.0.0\"\npostmortem_id = \"alpha\"\nseverity = \"critical\"\n",
    )
    .unwrap();

    let (output, _dir, json_path) = run_doctor(&repo.path().to_path_buf());
    assert!(output.status.success());

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert!(severity_discipline_hits(&report).is_empty());
}

#[test]
fn doctor_ignores_bare_critical_lines() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "critical\n").unwrap();

    let (output, _dir, json_path) = run_doctor(&repo.path().to_path_buf());
    assert!(output.status.success());

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert!(severity_discipline_hits(&report).is_empty());
}
