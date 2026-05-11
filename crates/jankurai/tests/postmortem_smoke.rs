use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

use jankurai::validation::{self, ArtifactSchema};

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/postmortem")
        .join(name)
}

fn run_postmortem_report(
    repo: &PathBuf,
    args: &[&str],
) -> (std::process::Output, tempfile::TempDir, PathBuf, PathBuf) {
    let out_dir = tempdir().unwrap();
    let json_path = out_dir.path().join("postmortem.json");
    let md_path = out_dir.path().join("postmortem.md");
    let mut cmd = Command::new(binary_path());
    cmd.arg("postmortem").arg(repo);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg("--out").arg(&json_path).arg("--md").arg(&md_path);
    let output = cmd.output().unwrap();
    (output, out_dir, json_path, md_path)
}

#[test]
fn postmortem_record_writes_durable_toml() {
    let repo = tempdir().unwrap();
    fs::write(
        repo.path().join("alpha.toml"),
        fs::read_to_string(fixture("alpha.toml")).unwrap(),
    )
    .unwrap();

    let out_dir = tempdir().unwrap();
    let md_path = out_dir.path().join("postmortem.md");
    let output = Command::new(binary_path())
        .arg("postmortem")
        .arg(repo.path())
        .arg("record")
        .arg("alpha.toml")
        .arg("--md")
        .arg(&md_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let record_path = repo.path().join(".jankurai/postmortems/alpha.toml");
    let record_text = fs::read_to_string(&record_path).unwrap();
    let record_value: toml::Value = toml::from_str(&record_text).unwrap();
    let record_json = serde_json::to_value(record_value).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::Postmortem, &record_json).unwrap();
    assert!(record_text.contains("failure_mode = \"env-prerequisite\""));
    assert!(record_text.contains("postmortem_id = \"alpha\""));
    assert!(fs::read_to_string(&md_path)
        .unwrap()
        .starts_with("# jankurai Postmortem"));
}

#[test]
fn postmortem_list_show_and_read_are_read_only() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join(".jankurai/postmortems")).unwrap();
    fs::write(
        repo.path().join(".jankurai/postmortems/alpha.toml"),
        fs::read_to_string(fixture("alpha.toml")).unwrap(),
    )
    .unwrap();
    fs::write(
        repo.path().join(".jankurai/postmortems/beta.toml"),
        fs::read_to_string(fixture("beta.toml")).unwrap(),
    )
    .unwrap();
    let before = fs::read_dir(repo.path().join(".jankurai/postmortems"))
        .unwrap()
        .count();

    let (list_output, _dir, list_json, list_md) =
        run_postmortem_report(&repo.path().to_path_buf(), &["list"]);
    assert!(list_output.status.success());
    let list: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&list_json).unwrap()).unwrap();
    assert_eq!(list["records_total"], 2);
    assert!(list["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|record| record["postmortem_id"] == "alpha"));
    assert!(fs::read_to_string(&list_md)
        .unwrap()
        .starts_with("# jankurai Postmortem List"));

    let (show_output, _dir, show_json, _) = run_postmortem_report(
        &repo.path().to_path_buf(),
        &["show", "--postmortem-id", "alpha"],
    );
    assert!(show_output.status.success());
    let show: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&show_json).unwrap()).unwrap();
    assert_eq!(show["record"]["postmortem_id"], "alpha");
    assert_eq!(show["record"]["failure_mode"], "env-prerequisite");

    let beta_path = repo.path().join(".jankurai/postmortems/beta.toml");
    let (read_output, _dir, read_json, _) = run_postmortem_report(
        &repo.path().to_path_buf(),
        &["read", beta_path.to_str().unwrap()],
    );
    assert!(read_output.status.success());
    let read: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&read_json).unwrap()).unwrap();
    assert_eq!(read["record"]["postmortem_id"], "beta");
    assert_eq!(read["record"]["failure_mode"], "equivalence-gap");

    let after = fs::read_dir(repo.path().join(".jankurai/postmortems"))
        .unwrap()
        .count();
    assert_eq!(before, after);
}
