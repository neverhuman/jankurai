use jankurai::validation::{self, ArtifactSchema};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn seed_repo_score_artifacts(repo: &Path) {
    let state_dir = repo.join(".jankurai");
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(state_dir.join("repo-score.json"), "{\"score\":0}\n").unwrap();
    fs::write(state_dir.join("repo-score.md"), "# score\n").unwrap();
}

fn run_vibe_coverage(repo: &Path, out_dir: &Path) -> Value {
    let json = out_dir.join("vibe-coverage.json");
    let md = out_dir.join("vibe-coverage.md");
    let tex = out_dir.join("vibe-coverage.tex");
    let output = Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("vibe")
        .arg("coverage")
        .arg(repo)
        .arg("--source")
        .arg("agent/vibe-coverage.toml")
        .arg("--tips")
        .arg("tips/vibe_coding")
        .arg("--json")
        .arg(&json)
        .arg("--md")
        .arg(&md)
        .arg("--tex")
        .arg(&tex)
        .output()
        .expect("spawn jankurai vibe coverage");

    assert!(
        output.status.success(),
        "vibe coverage failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();
    let md_text = fs::read_to_string(md).unwrap();
    assert!(md_text.contains("# Vibe Coding Coverage"));
    assert!(md_text.contains("- Unmapped source rows: `0`"));
    let tex_text = fs::read_to_string(tex).unwrap();
    assert!(tex_text.contains("Green = detector-backed"));
    assert!(tex_text.contains("\\scriptsize"));
    assert!(tex_text.contains("\\setlength{\\tabcolsep}{2pt}"));
    assert!(tex_text.contains("\\rowcolor{green!18}"));
    assert!(tex_text.contains("\\rowcolor{yellow!28}"));
    for row in tex_text.lines().filter(|line| line.contains("\\rowcolor")) {
        assert!(
            !row.contains("HLT-022-AUTHZ-ISOLATION-GAP"),
            "generated table rows should use short rule labels; full IDs belong in the legend"
        );
    }
    report
}

#[test]
fn validates_all_source_rows_and_report_schema() {
    let repo = repo_root();
    seed_repo_score_artifacts(&repo);
    let tmp = tempfile::tempdir().unwrap();
    let report = run_vibe_coverage(&repo, tmp.path());

    validation::validate_value(&repo, ArtifactSchema::VibeCoverageReport, &report).unwrap();
    assert_eq!(report["issue_count"], 260);
    assert_eq!(report["source_ref_count"], 260);
    assert_eq!(report["expected_source_row_count"], 260);
    assert_eq!(report["unmapped_source_rows"], 0);
    assert!(report["duplicate_source_refs"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(report["missing_source_refs"].as_array().unwrap().is_empty());
    assert!(report["unexpected_source_refs"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(
        report["coverage_counts"]["detector-backed"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(report["coverage_counts"]["partial"].as_u64().unwrap() > 0);
    assert_eq!(report["coverage_counts"]["none"].as_u64().unwrap_or(0), 0);
    assert_eq!(
        report["canonical_group_counts"].as_object().unwrap().len(),
        14
    );
    assert_eq!(
        report["detector_status_counts"]["detector-backed"]
            .as_u64()
            .unwrap(),
        report["coverage_counts"]["detector-backed"]
            .as_u64()
            .unwrap()
    );
}

#[test]
fn validate_subcommand_fails_no_rows() {
    let repo = repo_root();
    seed_repo_score_artifacts(&repo);
    let output = Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("vibe")
        .arg("validate")
        .arg(&repo)
        .arg("--source")
        .arg("agent/vibe-coverage.toml")
        .arg("--tips")
        .arg("tips/vibe_coding")
        .output()
        .expect("spawn jankurai vibe validate");
    assert!(
        output.status.success(),
        "validate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
