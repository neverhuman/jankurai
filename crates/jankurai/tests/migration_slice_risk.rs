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
        .join("tests/fixtures/migration/slice-risk")
        .join(name)
}

fn run_slice_risk(
    repo: &PathBuf,
    slice_id: &str,
    check_env: bool,
) -> (std::process::Output, tempfile::TempDir, PathBuf, PathBuf) {
    let out_dir = tempdir().unwrap();
    let json_path = out_dir.path().join("slice-risk.json");
    let md_path = out_dir.path().join("slice-risk.md");
    let mut cmd = Command::new(binary_path());
    cmd.arg("migrate")
        .arg(repo)
        .arg("slice-risk")
        .arg("--plan")
        .arg("plan.json")
        .arg("--slice-id")
        .arg(slice_id)
        .arg("--out")
        .arg(&json_path)
        .arg("--md")
        .arg(&md_path);
    if check_env {
        cmd.arg("--check-env");
    }
    let output = cmd.output().unwrap();
    (output, out_dir, json_path, md_path)
}

#[test]
fn slice_risk_blocks_risky_slice() {
    let repo = fixture("repo");
    let (output, _dir, json_path, md_path) = run_slice_risk(&repo, "model-port", true);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    validation::validate_value(
        &fixture("repo"),
        ArtifactSchema::MigrationSliceRisk,
        &report,
    )
    .unwrap();
    assert_eq!(report["decision"], "block");
    assert!(report["signals_total"].as_u64().unwrap() >= 6);
    assert!(report["critical_signals"].as_u64().unwrap() >= 1);
    assert!(report["high_signals"].as_u64().unwrap() >= 1);
    assert!(report["recommendations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value.as_str() == Some("add shadow/equivalence gate before cutover")));
    assert!(report["env_checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value["name"] == "MODEL_HMAC_KEY"));
    assert!(fs::read_to_string(&md_path)
        .unwrap()
        .starts_with("# jankurai Migration Slice Risk"));
}

#[test]
fn slice_risk_passes_clean_slice() {
    let repo = fixture("repo");
    let (output, _dir, json_path, _) = run_slice_risk(&repo, "docs-cleanup", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    validation::validate_value(
        &fixture("repo"),
        ArtifactSchema::MigrationSliceRisk,
        &report,
    )
    .unwrap();
    assert_eq!(report["decision"], "pass");
    assert_eq!(report["signals_total"], 0);
    assert_eq!(report["critical_signals"], 0);
    assert_eq!(report["high_signals"], 0);
    assert_eq!(report["medium_signals"], 0);
}

#[test]
fn slice_risk_reports_missing_selected_paths_without_whole_repo_fallback() {
    let repo_dir = tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("unrelated")).unwrap();
    fs::write(
        repo_dir.path().join("unrelated/model.py"),
        "import torch\n\n\ndef load(path):\n    return torch.load(path)\n",
    )
    .unwrap();
    fs::write(
        repo_dir.path().join("plan.json"),
        plan_json("missing-slice", r#"["missing/"]"#, "\"notes\""),
    )
    .unwrap();

    let (output, _dir, json_path, _) =
        run_slice_risk(&repo_dir.path().to_path_buf(), "missing-slice", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(report["decision"], "review");
    assert_eq!(report["medium_signals"], 1);
    assert_eq!(report["high_signals"], 0);
    assert_eq!(report["critical_signals"], 0);
    assert!(report["signals"]
        .as_array()
        .unwrap()
        .iter()
        .any(|signal| signal["kind"] == "slice-path-missing"));
}

#[test]
fn slice_risk_keeps_docs_only_hmac_prerequisites_non_blocking() {
    let repo_dir = tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("docs")).unwrap();
    fs::write(
        repo_dir.path().join("docs/notes.md"),
        "env MODEL_HMAC_KEY required before release\n",
    )
    .unwrap();
    fs::write(
        repo_dir.path().join("plan.json"),
        plan_json(
            "docs-hmac",
            r#"["docs/"]"#,
            "\"env MODEL_HMAC_KEY required\"",
        ),
    )
    .unwrap();

    let (output, _dir, json_path, _) =
        run_slice_risk(&repo_dir.path().to_path_buf(), "docs-hmac", true);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_ne!(report["decision"], "block");
    assert_eq!(report["high_signals"], 0);
    assert!(report["env_checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value["name"] == "MODEL_HMAC_KEY"));
}

#[test]
fn slice_risk_flags_thread_count_env_and_prior_failure_hooks() {
    let repo_dir = tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("docs")).unwrap();
    fs::write(
        repo_dir.path().join("docs/notes.md"),
        "OMP_NUM_THREADS=2 for reproducibility\nretry after prior failure in the hook\n",
    )
    .unwrap();
    fs::write(
        repo_dir.path().join("plan.json"),
        plan_json(
            "thread-hooks",
            r#"["docs/"]"#,
            "\"OMP_NUM_THREADS=2; retry after prior failure\"",
        ),
    )
    .unwrap();

    let (output, _dir, json_path, _) =
        run_slice_risk(&repo_dir.path().to_path_buf(), "thread-hooks", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(report["decision"], "pass");
    assert!(report["signals"]
        .as_array()
        .unwrap()
        .iter()
        .any(|signal| signal["kind"] == "thread-count-env"));
    assert!(report["signals"]
        .as_array()
        .unwrap()
        .iter()
        .any(|signal| signal["kind"] == "prior-failure-hook"));
}

#[test]
fn slice_risk_extracts_prose_env_names_for_check_env() {
    let repo_dir = tempdir().unwrap();
    fs::write(
        repo_dir.path().join("plan.json"),
        plan_json("env-prose", "[]", "\"env MODEL_HMAC_KEY required\""),
    )
    .unwrap();

    let (output, _dir, json_path, _) =
        run_slice_risk(&repo_dir.path().to_path_buf(), "env-prose", true);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert!(report["env_checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value["name"] == "MODEL_HMAC_KEY"));
}

#[test]
fn slice_risk_prunes_skipped_directories_during_whole_repo_scan() {
    let repo_dir = tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("target")).unwrap();
    fs::create_dir_all(repo_dir.path().join("build")).unwrap();
    fs::write(
        repo_dir.path().join("target/noise.py"),
        "import torch\n\n\ndef load(path):\n    return torch.load(path)\n",
    )
    .unwrap();
    fs::write(repo_dir.path().join("build/noise.py"), "MODEL = {}\n").unwrap();
    fs::write(
        repo_dir.path().join("plan.json"),
        plan_json("whole-repo", "[]", "\"notes\""),
    )
    .unwrap();

    let (output, _dir, json_path, _) =
        run_slice_risk(&repo_dir.path().to_path_buf(), "whole-repo", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(report["decision"], "pass");
    assert_eq!(report["signals_total"], 0);
}

fn plan_json(slice_id: &str, allowed_paths: &str, notes: &str) -> String {
    format!(
        r#"{{
  "schema_version": "1.0.0",
  "command": "jankurai migrate",
  "status": "complete",
  "generated_at": "0",
  "source_report": "target/jankurai/migration-report.json",
  "target_stack": "rust-python-postgres",
  "plan_mode": "dry-run",
  "slices": [
    {{
      "slice_id": "{slice_id}",
      "owner": "tools",
      "status": "candidate",
      "risk_level": "low",
      "dependency_order": 1,
      "human_approval_required": false,
      "allowed_paths": {allowed_paths},
      "forbidden_paths": [],
      "contracts": [],
      "tests": [],
      "proof_lanes": [],
      "rollback_notes": [],
      "notes": {notes}
    }}
  ],
  "human_approval_requirements": []
}}"#
    )
}
