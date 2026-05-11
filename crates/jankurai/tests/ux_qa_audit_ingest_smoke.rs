use jankurai::audit::run_audit;
use jankurai::render::render_markdown;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn thin_repo(dir: &std::path::Path) {
    fs::create_dir_all(dir.join("agent")).unwrap();
    fs::write(
        dir.join("AGENTS.md"),
        "Read `agent/JANKURAI_STANDARD.md` first.\n",
    )
    .unwrap();
    fs::write(dir.join("README.md"), "# thin repo\n").unwrap();
    fs::write(dir.join("Justfile"), "check:\n    cargo test\n").unwrap();
    fs::create_dir_all(dir.join("docs")).unwrap();
    fs::write(dir.join("docs/architecture.md"), "# architecture\n").unwrap();
    fs::write(dir.join("docs/testing.md"), "# testing\n").unwrap();
    fs::write(
        dir.join("agent/JANKURAI_STANDARD.md"),
        "Standard version: `0.5.0`\n",
    )
    .unwrap();
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn tuiwright_fixture(name: &str) -> PathBuf {
    repo_root()
        .join("crates/jankurai/tests/fixtures/tuiwright")
        .join(name)
}

fn copy_tree(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let ty = entry.file_type().unwrap();
        let dest_path = dest.join(entry.file_name());
        if ty.is_dir() {
            copy_tree(&entry.path(), &dest_path);
        } else if ty.is_file() {
            fs::create_dir_all(dest_path.parent().unwrap()).unwrap();
            fs::copy(entry.path(), &dest_path).unwrap();
        }
    }
}

fn seed_tuiwright_fixture(repo: &Path, name: &str) {
    thin_repo(repo);
    copy_tree(&tuiwright_fixture(name), repo);
}

fn one_report(decision: &str, summary: (u64, u64)) -> serde_json::Value {
    let (errors, warnings) = summary;
    serde_json::json!({
        "schemaVersion": "1.2.0",
        "toolVersion": "0.5.0",
        "url": "about:blank",
        "checkedAt": "2026-05-02T12:00:00.000Z",
        "viewport": { "width": 1280, "height": 720 },
        "metrics": {
            "scrollWidth": 1280,
            "clientWidth": 1280,
            "scrollHeight": 720,
            "clientHeight": 720
        },
        "elements": [],
        "violations": [],
        "artifacts": [],
        "summary": { "errors": errors, "warnings": warnings, "byRule": {} },
        "decision": decision
    })
}

fn evidence_report() -> serde_json::Value {
    let mut report = one_report("block", (1, 0));
    report["schemaVersion"] = serde_json::json!("1.4.0");
    report["artifacts"] = serde_json::json!([
        {
            "kind": "screenshot",
            "path": "target/jankurai/ux-qa/local.png",
            "sha256": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            "viewport": { "width": 1280, "height": 720 }
        },
        {
            "kind": "accessibility",
            "path": "target/jankurai/ux-qa/local.a11y.json",
            "viewport": { "width": 1280, "height": 720 }
        }
    ]);
    report["visualBaseline"] = serde_json::json!({
        "mode": "block",
        "status": "changed",
        "decision": "block",
        "actualPath": "target/jankurai/ux-qa/local.png",
        "baselinePath": "target/jankurai/ux-qa/baseline.png",
        "diffPath": "target/jankurai/ux-qa/diff.png",
        "actualSha256": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "baselineSha256": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
        "owner": "design",
        "approvedBy": "ux",
        "approvedAt": "2026-05-02T12:00:00.000Z",
        "approvalNote": "fixture"
    });
    report["stateCoverage"] = serde_json::json!({
        "required": ["loading", "success"],
        "declared": ["success"],
        "missing": ["loading"]
    });
    report["artifactCoverage"] = serde_json::json!({
        "required": ["screenshot", "aria-snapshot", "accessibility"],
        "present": ["screenshot", "accessibility"],
        "missing": ["aria-snapshot"]
    });
    report["accessibility"] = serde_json::json!({
        "violations": 2,
        "incomplete": 1,
        "passes": 7,
        "artifactPath": "target/jankurai/ux-qa/local.a11y.json"
    });
    report
}

fn ux_qa_envelope(reports: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({ "reports": reports })
}

#[test]
fn audit_ingests_valid_ux_qa_json_artifact_summary() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai")).unwrap();
    let env = ux_qa_envelope(vec![one_report("pass", (0, 0))]);
    fs::write(
        dir.path().join("target/jankurai/ux-qa.json"),
        serde_json::to_string_pretty(&env).unwrap(),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let art = report.ux_qa.artifact.as_ref().expect("artifact summary");
    assert_eq!(art.path, "target/jankurai/ux-qa.json");
    assert_eq!(art.report_count, 1);
    assert_eq!(art.worst_decision, "pass");
    assert_eq!(art.total_violations, 0);
    assert_eq!(art.summary_errors, 0);
    assert_eq!(art.summary_warnings, 0);
    assert_eq!(art.reports_missing_required_states, 0);
    assert!(art.artifact_counts_by_kind.is_empty());
    assert_eq!(art.accessibility_violation_total, 0);
}

#[test]
fn audit_ux_qa_worst_decision_orders_block_over_pass() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai")).unwrap();
    let env = ux_qa_envelope(vec![
        one_report("pass", (0, 0)),
        one_report("block", (0, 0)),
    ]);
    fs::write(
        dir.path().join("target/jankurai/ux-qa.json"),
        serde_json::to_string(&env).unwrap(),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let art = report.ux_qa.artifact.as_ref().unwrap();
    assert_eq!(art.report_count, 2);
    assert_eq!(art.worst_decision, "block");
}

#[test]
fn audit_ux_qa_aggregates_summary_counts() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai")).unwrap();
    let env = ux_qa_envelope(vec![
        one_report("warn", (2, 5)),
        one_report("review", (1, 0)),
    ]);
    fs::write(
        dir.path().join("target/jankurai/ux-qa.json"),
        serde_json::to_string(&env).unwrap(),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let art = report.ux_qa.artifact.as_ref().unwrap();
    assert_eq!(art.summary_errors, 3);
    assert_eq!(art.summary_warnings, 5);
    assert_eq!(art.worst_decision, "review");
}

#[test]
fn audit_ux_qa_aggregates_state_artifact_and_accessibility_evidence() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai")).unwrap();
    let env = ux_qa_envelope(vec![evidence_report()]);
    fs::write(
        dir.path().join("target/jankurai/ux-qa.json"),
        serde_json::to_string(&env).unwrap(),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let art = report.ux_qa.artifact.as_ref().unwrap();
    assert_eq!(art.reports_missing_required_states, 1);
    assert_eq!(art.missing_state_names, vec!["loading".to_string()]);
    assert_eq!(art.artifact_counts_by_kind.get("screenshot"), Some(&1));
    assert_eq!(art.artifact_counts_by_kind.get("accessibility"), Some(&1));
    assert_eq!(art.reports_missing_required_artifacts, 1);
    assert_eq!(
        art.missing_artifact_kinds,
        vec!["aria-snapshot".to_string()]
    );
    assert_eq!(art.reports_missing_required_accessibility_artifact, 0);
    assert_eq!(art.accessibility_violation_total, 2);
    assert_eq!(art.accessibility_incomplete_total, 1);
    assert_eq!(art.accessibility_pass_total, 7);
    assert_eq!(art.artifact_fingerprint_count, 1);
    assert_eq!(art.visual_baseline_missing, 0);
    assert_eq!(art.visual_baseline_changed, 1);
    assert_eq!(art.visual_baseline_review, 0);
    assert_eq!(art.visual_baseline_block, 1);
}

#[test]
fn audit_adds_findings_for_incomplete_validated_ux_evidence() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai")).unwrap();
    let mut report_value = evidence_report();
    report_value["artifactCoverage"]["missing"] =
        serde_json::json!(["aria-snapshot", "accessibility"]);
    let env = ux_qa_envelope(vec![report_value]);
    fs::write(
        dir.path().join("target/jankurai/ux-qa.json"),
        serde_json::to_string(&env).unwrap(),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.rule_id.as_deref() == Some("HLT-013-RENDERED-UX-GAP")));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.problem.contains("visual baseline gaps")));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.rule_id.as_deref() == Some("HLT-014-A11Y-GAP")));
}

#[test]
fn audit_ux_qa_aggregates_visual_baseline_review_and_missing_counts() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai")).unwrap();
    let mut review_report = evidence_report();
    review_report["visualBaseline"]["status"] = serde_json::json!("missing-baseline");
    review_report["visualBaseline"]["decision"] = serde_json::json!("review");
    review_report["visualBaseline"]["mode"] = serde_json::json!("review");
    review_report["artifacts"][0]["sha256"] = serde_json::json!(
        "sha256:3333333333333333333333333333333333333333333333333333333333333333"
    );
    let env = ux_qa_envelope(vec![evidence_report(), review_report]);
    fs::write(
        dir.path().join("target/jankurai/ux-qa.json"),
        serde_json::to_string(&env).unwrap(),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let art = report.ux_qa.artifact.as_ref().unwrap();
    assert_eq!(art.report_count, 2);
    assert_eq!(art.artifact_fingerprint_count, 2);
    assert_eq!(art.visual_baseline_missing, 1);
    assert_eq!(art.visual_baseline_changed, 1);
    assert_eq!(art.visual_baseline_review, 1);
    assert_eq!(art.visual_baseline_block, 1);
}

#[test]
fn audit_invalid_ux_qa_json_leaves_artifact_none() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai")).unwrap();
    fs::write(dir.path().join("target/jankurai/ux-qa.json"), "{}\n").unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.ux_qa.artifact.is_none());
}

#[test]
fn audit_without_ux_qa_file_leaves_artifact_none() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.ux_qa.artifact.is_none());
}

#[test]
fn audit_collects_tuiwright_evidence_and_renders_a_summary_line() {
    let dir = tempdir().unwrap();
    seed_tuiwright_fixture(dir.path(), "full");

    let report = run_audit(dir.path(), &[]).unwrap();
    let tuiwright = report
        .ux_qa
        .evidence
        .get("tuiwright")
        .and_then(|value| value.as_object())
        .expect("tuiwright evidence");
    assert_eq!(
        tuiwright
            .get("surface_detected")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        tuiwright.get("flow_count").and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        tuiwright
            .get("test_files")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(1)
    );
    assert!(
        tuiwright
            .get("assertion_count")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            >= 4,
        "expected at least four assertion signals"
    );
    assert!(
        tuiwright
            .get("action_count")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            >= 4,
        "expected at least four action signals"
    );
    let artifact_counts = tuiwright
        .get("artifact_counts")
        .and_then(|value| value.as_object())
        .expect("artifact counts");
    assert!(
        artifact_counts
            .get("screenshot")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            >= 1
    );
    assert!(
        artifact_counts
            .get("stop_recording_gif")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            >= 1
    );
    assert!(
        artifact_counts
            .get("trace_path")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            >= 1
    );

    let md = render_markdown(&report);
    assert!(md.contains("Tuiwright TUI flows: `2` flow(s) across `1` file(s)"));
    assert!(md.contains("artifacts=`"));
    assert!(md.contains("screenshot="));
}

#[test]
fn audit_ignores_helper_code_and_assertion_free_tuiwright_mentions() {
    let dir = tempdir().unwrap();
    seed_tuiwright_fixture(dir.path(), "gap");

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.ux_qa.evidence.get("tuiwright").is_none());
    let md = render_markdown(&report);
    assert!(!md.contains("Tuiwright TUI flows:"));
}
