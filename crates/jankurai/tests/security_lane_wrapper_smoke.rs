use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use jankurai::validation::{self, ArtifactSchema};
use tempfile::tempdir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn write_policy(repo: &std::path::Path) {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/security-policy.toml"),
        r#"
schema_version = "1.0.0"
enabled_tools = ["gitleaks", "cargo audit"]
required_tools = ["gitleaks"]
advisory_tools = ["cargo audit"]

[severity_thresholds]
fail_lane_on = "high"
"#,
    )
    .unwrap();
}

#[test]
fn required_tool_failure_exits_nonzero_and_records_real_exit_code() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("tools")).unwrap();
    write_policy(repo.path());

    let bin_dir = tempdir().unwrap();
    let gitleaks = bin_dir.path().join("gitleaks");
    fs::write(
        &gitleaks,
        "#!/usr/bin/env bash\necho gitleaks-boom >&2\nexit 7\n",
    )
    .unwrap();
    let mut perms = fs::metadata(&gitleaks).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&gitleaks, perms).unwrap();

    fs::write(
        repo.path().join("tools/security-lane.sh"),
        fs::read_to_string(repo_root().join("tools/security-lane.sh")).unwrap(),
    )
    .unwrap();

    let evidence_path = repo.path().join("target/jankurai/security/evidence.json");
    let output = Command::new(binary_path())
        .current_dir(repo.path())
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin_dir.path().display(),
                env::var("PATH").unwrap_or_default()
            ),
        )
        .args([
            "security",
            "run",
            ".",
            "--script",
            "tools/security-lane.sh",
            "--out",
            evidence_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "security run unexpectedly succeeded"
    );

    let text = fs::read_to_string(&evidence_path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::SecurityEvidence, &value).unwrap();
    assert_eq!(value["exit_code"], 7);
    assert_eq!(value["commands"][0]["status"], "failed");
    assert_eq!(value["commands"][0]["exit_code"], 7);

    let log_rel = value["log_path"].as_str().unwrap();
    let log_text = fs::read_to_string(repo.path().join(log_rel)).unwrap();
    assert!(log_text.contains("gitleaks-boom"));
}
