use std::fs;
use std::path::PathBuf;
use std::process::Command;

use jankurai::audit::run_audit;
use jankurai::validation::{self, ArtifactSchema};
use tempfile::tempdir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

#[test]
fn badge_command_emits_readme_schema_valid_json() {
    let repo = tempdir().unwrap();
    fs::create_dir_all(repo.path().join("agent")).unwrap();
    fs::write(
        repo.path().join("AGENTS.md"),
        "Read agent/JANKURAI_STANDARD.md first.\n",
    )
    .unwrap();
    fs::write(repo.path().join("README.md"), "# fixture\n").unwrap();
    fs::write(
        repo.path().join("Justfile"),
        "fast:\n    echo ok\ncheck:\n    echo ok\n",
    )
    .unwrap();
    fs::write(
        repo.path().join("agent/JANKURAI_STANDARD.md"),
        "Standard version: `0.8.0`\n",
    )
    .unwrap();
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(
        repo.path().join("docs/agent-native-standard.md"),
        "Standard version: `0.8.0`\n",
    )
    .unwrap();
    let report = run_audit(repo.path(), &[]).unwrap();
    let mut report_value = serde_json::to_value(&report).unwrap();
    report_value["score"] = serde_json::json!(100);
    report_value["raw_score"] = serde_json::json!(100);
    report_value["dirty_worktree"] = serde_json::json!(false);
    report_value["findings"] = serde_json::json!([]);
    report_value["caps_applied"] = serde_json::json!([]);
    report_value["decision"]["status"] = serde_json::json!("pass");
    report_value["decision"]["passed"] = serde_json::json!(true);
    report_value["decision"]["hard_findings"] = serde_json::json!(0);
    report_value["decision"]["soft_findings"] = serde_json::json!(0);
    fs::write(
        repo.path().join("agent/repo-score.json"),
        serde_json::to_string_pretty(&report_value).unwrap(),
    )
    .unwrap();

    let status = Command::new(binary_path())
        .current_dir(repo.path())
        .args(["badge", ".", "--no-readme"])
        .status()
        .unwrap();
    assert!(status.success(), "badge command failed");

    let badge_json = repo.path().join("agent/jankurai-badge.json");
    let badge: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&badge_json).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::ReadmeBadge, &badge).unwrap();
    assert_eq!(badge["standard"], "jankurai");
    assert_eq!(badge["score"], 100);

    let badge_svg = fs::read_to_string(repo.path().join("agent/jankurai-badge.svg")).unwrap();
    assert!(badge_svg.contains(">100/100<"));
    assert!(!badge_svg.contains("100/100 pass"));
}
