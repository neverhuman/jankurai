//! Semantic guards for audit JSON, sidecar exports, findings, and issues JSONL.
//! See `tips/phases_feedback/01-standard/tip2.txt` and `docs/phases-feedback-status.md`.

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

fn run_full_audit_export(repo: &Path, out_dir: &Path) -> Value {
    let json = out_dir.join("repo-score.json");
    let md = out_dir.join("repo-score.md");
    let sarif = out_dir.join("jankurai.sarif");
    let junit = out_dir.join("jankurai.junit.xml");
    let summary = out_dir.join("summary.md");
    let repair_queue = out_dir.join("repair-queue.jsonl");
    let score_history = out_dir.join("score-history.jsonl");
    let score_history_csv = out_dir.join("score-history.csv");

    let output = Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("audit")
        .arg(repo)
        .arg("--full")
        .arg("--json")
        .arg(&json)
        .arg("--md")
        .arg(&md)
        .arg("--sarif")
        .arg(&sarif)
        .arg("--junit")
        .arg(&junit)
        .arg("--github-step-summary")
        .arg(&summary)
        .arg("--repair-queue-jsonl")
        .arg(&repair_queue)
        .arg("--score-history")
        .arg(&score_history)
        .arg("--score-history-csv")
        .arg(&score_history_csv)
        .output()
        .expect("spawn jankurai audit");

    assert!(
        output.status.success(),
        "audit failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let text = fs::read_to_string(&json).expect("read repo-score json");
    serde_json::from_str(&text).expect("repo-score json parses")
}

#[test]
fn repo_score_json_validates_and_matches_committed_standard_versions() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_root();
    let report = run_full_audit_export(&repo, tmp.path());

    validation::validate_value(&repo, ArtifactSchema::RepoScore, &report).unwrap();

    let standard = fs::read_to_string(repo.join("agent/standard-version.toml"))
        .expect("read agent/standard-version.toml");
    let std_value =
        jankurai::validation::validate_standard_version_toml_text(&repo, &standard).unwrap();

    assert_eq!(report["standard"], "jankurai");
    assert_eq!(
        report["standard_version"], std_value["standard_version"],
        "report standard_version should track agent/standard-version.toml"
    );
    assert_eq!(
        report["paper_edition"], std_value["paper_edition"],
        "report paper_edition should track agent/standard-version.toml"
    );
    assert_eq!(
        report["schema_version"], std_value["schema_version"],
        "report schema_version should track agent/standard-version.toml"
    );
    assert_eq!(
        report["auditor_version"], std_value["auditor_version"],
        "report auditor_version should track agent/standard-version.toml"
    );
    assert_eq!(
        report["target_stack_id"], std_value["target_stack"],
        "report target_stack_id should track agent/standard-version.toml target_stack"
    );
    assert_eq!(report["schema_url"], "schemas/repo-score.schema.json");
    assert!(report["profile_structure"].as_object().is_some());
    assert!(report["profile_structure"]["cells"].as_array().is_some());

    let scope = report["scope"].as_object().expect("scope object");
    assert!(
        scope.get("mode").is_some() && scope.get("paths").is_some(),
        "scope must expose mode and paths"
    );
    assert!(report["dimensions"].as_array().is_some());
    assert!(report["hard_rules"].as_array().is_some());
}

#[test]
fn sidecar_report_exports_stay_semantically_parseable() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_root();
    let report = run_full_audit_export(&repo, tmp.path());

    let sarif_path = tmp.path().join("jankurai.sarif");
    let sarif: Value =
        serde_json::from_str(&fs::read_to_string(&sarif_path).unwrap()).expect("sarif json");
    assert_eq!(sarif["version"], "2.1.0");
    let runs = sarif["runs"].as_array().expect("sarif runs");
    assert!(!runs.is_empty(), "sarif must include at least one run");
    assert_eq!(runs[0]["tool"]["driver"]["name"], "jankurai");
    assert!(runs[0]["results"].is_array());

    let junit_text = fs::read_to_string(tmp.path().join("jankurai.junit.xml")).unwrap();
    assert!(
        junit_text.contains("<testsuite") && junit_text.contains("</testsuite>"),
        "junit export should be a testsuite envelope"
    );
    assert!(
        junit_text.contains("<testcase"),
        "junit export should include testcase rows"
    );

    let md_text = fs::read_to_string(tmp.path().join("repo-score.md")).unwrap();
    assert!(
        md_text.contains("# jankurai Repo Score"),
        "markdown score should keep a stable title"
    );
    if let Some(tuiwright) = report["ux_qa"]["evidence"]["tuiwright"].as_object() {
        assert!(
            md_text.contains("Tuiwright TUI flows:"),
            "markdown score should surface Tuiwright proof when present"
        );
        assert_eq!(tuiwright["surface_detected"], true);
        assert!(
            tuiwright["flow_count"].as_u64().unwrap_or(0) > 0,
            "when present, Tuiwright proof should expose at least one flow"
        );
    } else {
        assert!(
            !md_text.contains("Tuiwright TUI flows:"),
            "markdown score should not invent Tuiwright proof when the evidence is absent"
        );
    }

    let summary = fs::read_to_string(tmp.path().join("summary.md")).unwrap();
    assert!(
        summary.contains("### jankurai") && summary.contains("score:"),
        "github step summary should keep stable headings"
    );

    if let Some(findings) = report["findings"].as_array() {
        for (i, finding) in findings.iter().enumerate() {
            validation::validate_value(&repo, ArtifactSchema::Finding, finding).unwrap_or_else(
                |e| {
                    panic!("repo-score finding[{i}] failed finding.schema.json: {e}");
                },
            );
        }
    }

    let rq_path = tmp.path().join("repair-queue.jsonl");
    let rq_text = fs::read_to_string(&rq_path).expect("repair-queue jsonl");
    for (line_no, line) in rq_text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let item: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("repair-queue line {}: {e}", line_no + 1));
        validation::validate_value(&repo, ArtifactSchema::RepairQueueItem, &item).unwrap();
    }

    if let Some(queue) = report["agent_fix_queue"].as_array() {
        assert_eq!(
            rq_text.lines().filter(|l| !l.trim().is_empty()).count(),
            queue.len(),
            "jsonl line count should match agent_fix_queue items"
        );
    }
}

#[test]
fn repo_score_markdown_keeps_stable_sections() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_root();
    let report = run_full_audit_export(&repo, tmp.path());
    let md = fs::read_to_string(tmp.path().join("repo-score.md")).unwrap();
    for needle in [
        "# jankurai Repo Score",
        "## Hard Rule Caps",
        "## Copy-Code Redundancy",
        "## Dimensions",
        "## Reference Profile Structure",
        "## Rendered UX QA",
        "## Tool Adoption",
        "## Boundary Reclassifications",
        "## Findings",
        "## Agent Fix Queue",
    ] {
        assert!(
            md.contains(needle),
            "repo-score.md missing stable section `{needle}`"
        );
    }

    if report["vibe_coverage"].is_object() {
        assert!(
            md.contains("## Vibe Coding Coverage"),
            "repo-score.md should include vibe coverage when the report carries it"
        );
    } else {
        assert!(
            !md.contains("## Vibe Coding Coverage"),
            "repo-score.md should not invent vibe coverage when the report omits it"
        );
    }
    if report["coverage_evidence"].is_object() {
        assert!(
            md.contains("## Coverage Evidence"),
            "repo-score.md should include coverage evidence when the report carries it"
        );
    } else {
        assert!(
            !md.contains("## Coverage Evidence"),
            "repo-score.md should not invent coverage evidence when the report omits it"
        );
    }
}

#[test]
fn repo_score_schema_keeps_coverage_evidence_optional() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_root();
    let mut report = run_full_audit_export(&repo, tmp.path());
    report.as_object_mut().unwrap().remove("coverage_evidence");
    validation::validate_value(&repo, ArtifactSchema::RepoScore, &report).unwrap();

    report.as_object_mut().unwrap().insert(
        "coverage_evidence".into(),
        serde_json::json!({
            "artifact": "target/jankurai/coverage/coverage-audit.json",
            "status": "warn",
            "sources_total": 2,
            "sources_present": 1,
            "hard_findings": 0,
            "soft_findings": 1
        }),
    );
    validation::validate_value(&repo, ArtifactSchema::RepoScore, &report).unwrap();
}

#[test]
fn issues_export_jsonl_each_line_validates_as_finding() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_root();
    let out = tmp.path().join("issues.jsonl");
    assert!(
        Command::new(env!("CARGO_BIN_EXE_jankurai"))
            .arg("issues")
            .arg("export")
            .arg(&repo)
            .arg("--format")
            .arg("jsonl")
            .arg("--out")
            .arg(&out)
            .status()
            .unwrap()
            .success(),
        "issues export jsonl failed"
    );
    let text = fs::read_to_string(&out).unwrap();
    for (line_no, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("issues jsonl line {}: {e}", line_no + 1));
        validation::validate_value(&repo, ArtifactSchema::Finding, &value).unwrap();
    }
}
