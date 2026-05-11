use std::fs;
use std::process::Command;

#[test]
fn public_repo_scores_accepts_star_rank_source_shape() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("source.json"),
        r#"{
  "source_artifact": "paper/data/public-repo-scores-example.json",
  "generated_at": "2026-05-06T01:41:56Z",
  "jankurai_version": "0.8.8",
  "repo_count": 2,
  "successful": 2,
  "failed": 0,
  "rows": [
    {
      "star_rank": 2,
      "repo": "example/beta",
      "stars": "84k+",
      "score": 42,
      "issues": 7,
      "hard_findings": 6,
      "soft_findings": 1,
      "severity": {"medium": 1, "high": 5, "critical": 1},
      "top_categories": [{"name": "vibe", "count": 4}, {"name": "security", "count": 2}]
    },
    {
      "star_rank": 1,
      "repo": "example/alpha",
      "stars": "100k",
      "score": 47,
      "issues": 3,
      "hard_findings": 2,
      "soft_findings": 1,
      "severity": {"medium": 1, "high": 2},
      "top_categories": [{"name": "proof", "count": 2}, {"name": "context", "count": 1}]
    }
  ]
}"#,
    )
    .expect("write source");

    let status = Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .current_dir(dir.path())
        .args([
            "paper",
            "public-repo-scores",
            "--source",
            "source.json",
            "--out",
            "tables.tex",
        ])
        .status()
        .expect("run public repo score renderer");
    assert!(status.success());

    let rendered = fs::read_to_string(dir.path().join("tables.tex")).expect("read rendered tex");
    assert!(rendered.contains(r"\textbf{Stars}"));
    assert!(rendered.contains(r"\textbf{Hard/Soft}"));
    assert!(rendered.contains(r"\textbf{Severity}"));
    assert!(rendered.contains("84k+"));
    assert!(rendered.contains("2/1"));
    assert!(rendered.contains("med 1; high 2"));
    assert!(rendered.find("example/alpha").unwrap() < rendered.find("example/beta").unwrap());
    assert!(!rendered.contains("/Users/"));
    assert!(!rendered.contains("/home/ubuntu"));
    assert!(!rendered.contains("~/jankscore"));
}
