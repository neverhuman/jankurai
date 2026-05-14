//! Golden-ish tests for the denial-feedback formatting in
//! `jankurai_guard::feedback`: the markdown/JSON failure report carries the
//! required fields and sections, and the PTY banner renders the path, the
//! findings and the re-run command.

use jankurai_guard::audit_client::{
    AdvisoryFindings, AuditClient, BlockingFindings, GuardDecision, GuardFinding, MockAuditClient,
    Verdict,
};
use jankurai_guard::feedback::banner::format_banner;
use jankurai_guard::feedback::report::{format_report_md, write_failure_report};
use std::path::Path;
use tempfile::tempdir;

/// Builds a block decision with one blocking and one pre-existing finding.
fn decision_with_preexisting() -> GuardDecision {
    GuardDecision {
        schema: "jankurai-save-gate/1".to_string(),
        verdict: Verdict::Block,
        exit_code: 3,
        path: "src/foo.rs".to_string(),
        mode: "save-gate".to_string(),
        candidate_score: Some(70),
        baseline_score: Some(88),
        summary: "1 new hard finding".to_string(),
        blocking: BlockingFindings {
            new_hard_findings: vec![GuardFinding {
                severity: "high".to_string(),
                category: "audit".to_string(),
                path: "src/foo.rs".to_string(),
                problem: "function exceeds the line budget".to_string(),
                agent_fix: "split the function into smaller units".to_string(),
                line: Some(88),
                rule_id: "HLT-029".to_string(),
                check_id: "HLT-029:audit".to_string(),
                evidence: vec!["fn render spans 142 lines".to_string()],
            }],
            ..BlockingFindings::default()
        },
        advisory: AdvisoryFindings::default(),
        preexisting_findings: vec![GuardFinding {
            severity: "low".to_string(),
            category: "audit".to_string(),
            path: "src/foo.rs".to_string(),
            problem: "a pre-existing nit".to_string(),
            agent_fix: String::new(),
            line: Some(3),
            rule_id: "HLT-101".to_string(),
            check_id: "HLT-101:audit".to_string(),
            evidence: vec![],
        }],
        rerun_command: "jankurai audit-file . --path src/foo.rs --candidate - --mode save-gate"
            .to_string(),
    }
}

#[test]
fn report_md_has_all_required_fields() {
    let decision = decision_with_preexisting();
    let md = format_report_md(&decision);
    assert!(md.starts_with("# JANKURAI GUARD: BLOCKED src/foo.rs"));
    assert!(md.contains("1 new hard finding"));
    assert!(md.contains("Candidate score 70 vs baseline 88."));
    assert!(md.contains("## Blocking findings"));
    assert!(md.contains("[BLOCK] HLT-029  function exceeds the line budget"));
    assert!(md.contains("line 88"));
    assert!(md.contains("fix: split the function into smaller units"));
    assert!(md.contains("> fn render spans 142 lines"));
    assert!(md.contains("## Pre-existing issues (not blocking)"));
    assert!(md.contains("HLT-101"));
    assert!(md.contains("Re-run after fixing: `jankurai audit-file . --path src/foo.rs"));
}

#[test]
fn report_md_handles_empty_preexisting() {
    let decision = MockAuditClient::always_block()
        .audit(Path::new("."), Path::new("src/foo.rs"), b"x")
        .unwrap();
    let md = format_report_md(&decision);
    assert!(md.contains("## Pre-existing issues (not blocking)\n\n_None._"));
}

#[test]
fn write_failure_report_writes_md_json_and_last_failure() {
    let dir = tempdir().unwrap();
    let decision = decision_with_preexisting();
    let md_path = write_failure_report(dir.path(), &decision).unwrap();
    assert!(md_path.exists());
    assert!(md_path.extension().map(|e| e == "md").unwrap_or(false));

    let json_path = md_path.with_extension("json");
    assert!(json_path.exists());
    let parsed: GuardDecision =
        serde_json::from_str(&std::fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(parsed.path, "src/foo.rs");

    let last = dir.path().join(".jankurai/guard/LAST_FAILURE.md");
    assert!(last.exists());
    let last_text = std::fs::read_to_string(&last).unwrap();
    assert!(last_text.contains("JANKURAI GUARD: BLOCKED src/foo.rs"));
}

#[test]
fn banner_renders_path_findings_and_rerun() {
    let decision = decision_with_preexisting();
    let banner = format_banner(&decision);
    assert!(banner.contains("JANKURAI GUARD: BLOCKED"));
    assert!(banner.contains("src/foo.rs"));
    assert!(banner.contains("[BLOCK] HLT-029"));
    assert!(banner.contains("fix: split the function into smaller units"));
    assert!(banner.contains("re-run:"));
    // The banner is a box: it has the box-drawing corners.
    assert!(banner.contains('┌') && banner.contains('┘'));
}

#[test]
fn banner_tolerates_a_decision_with_no_individual_findings() {
    let mut decision = decision_with_preexisting();
    decision.blocking = BlockingFindings::default();
    let banner = format_banner(&decision);
    assert!(banner.contains("no individual findings reported"));
}
