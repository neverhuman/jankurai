/// Integration tests for HLT-041-COMMENT-HYGIENE detector.
///
/// Validates that:
/// 1. Risky fixtures produce findings (true positives)
/// 2. Safe fixtures produce zero findings (no false positives)
/// 3. File filtering works correctly
use jankurai::audit::language_rules::comments;
use jankurai::model::FileInfo;

fn make_file(rel_path: &str, text: &str) -> FileInfo {
    let suffix = std::path::Path::new(rel_path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{}", s))
        .unwrap_or_default();
    FileInfo {
        rel_path: rel_path.to_string(),
        name: rel_path.rsplit('/').next().unwrap_or(rel_path).to_string(),
        suffix,
        size: text.len() as u64,
        line_count: text.lines().count(),
        text: text.to_string(),
        is_generated: false,
        is_code: true,
    }
}

fn make_ctx(files: Vec<FileInfo>) -> jankurai::audit::helpers::AuditContext {
    let root = tempfile::tempdir().unwrap();
    jankurai::audit::helpers::AuditContext {
        root: root.path().to_path_buf(),
        all_files: files.clone(),
        scope_files: files,
        scope_paths: vec![],
        self_audit: false,
        boundary_reclassifications: vec![],
        copy_code: None,
    }
}

fn load_fixture(path: &str) -> String {
    let fixture_dir = format!(
        "{}/tests/fixtures/language_bad_behavior/comments",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(format!("{}/{}", fixture_dir, path))
        .unwrap_or_else(|e| panic!("failed to load fixture {}: {}", path, e))
}

// =====================================================================
// Tier 1 — Risky fixtures MUST produce findings
// =====================================================================

#[test]
fn security_bypass_produces_hard_findings() {
    let text = load_fixture("risky/security_bypass.rs");
    let file = make_file("src/security_bypass.rs", &text);
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    assert!(
        !findings.is_empty(),
        "security_bypass.rs must produce at least one hard finding"
    );
    for f in &findings {
        assert_eq!(f.rule_id, "HLT-041-COMMENT-HYGIENE");
    }
}

#[test]
fn production_confession_produces_hard_findings() {
    let text = load_fixture("risky/production_confession.rs");
    let file = make_file("src/production_confession.rs", &text);
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    assert!(
        !findings.is_empty(),
        "production_confession.rs must produce at least one hard finding"
    );
}

#[test]
fn hardcoded_secrets_produces_hard_findings() {
    let text = load_fixture("risky/hardcoded_secrets.rs");
    let file = make_file("src/hardcoded_secrets.rs", &text);
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    assert!(
        !findings.is_empty(),
        "hardcoded_secrets.rs must produce at least one hard finding"
    );
}

#[test]
fn ai_residue_produces_hard_findings() {
    let text = load_fixture("risky/ai_residue.ts");
    let file = make_file("src/ai_residue.ts", &text);
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    assert!(
        !findings.is_empty(),
        "ai_residue.ts must produce at least one hard finding"
    );
}

#[test]
fn fake_implementation_produces_hard_findings() {
    let text = load_fixture("risky/fake_implementation.py");
    let file = make_file("src/fake_implementation.py", &text);
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    assert!(
        !findings.is_empty(),
        "fake_implementation.py must produce at least one hard finding"
    );
}

#[test]
fn error_suppression_produces_hard_findings() {
    let text = load_fixture("risky/error_suppression.rs");
    let file = make_file("src/error_suppression.rs", &text);
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    assert!(
        !findings.is_empty(),
        "error_suppression.rs must produce at least one hard finding"
    );
}

// =====================================================================
// Tier 2 — Security-sensitive TODO produces advisory signals
// =====================================================================

#[test]
fn security_todo_produces_advisory_signals() {
    let text = load_fixture("risky/security_todo.rs");
    let file = make_file("src/security_todo.rs", &text);
    let ctx = make_ctx(vec![file]);
    let advisories = comments::advisory_signals(&ctx);
    assert!(
        !advisories.is_empty(),
        "security_todo.rs must produce at least one advisory signal"
    );
}

#[test]
fn security_todo_does_not_produce_hard_findings() {
    let text = load_fixture("risky/security_todo.rs");
    let file = make_file("src/security_todo.rs", &text);
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    // security_todo.rs has only tier 2 patterns, NOT tier 1
    assert!(
        findings.is_empty(),
        "security_todo.rs should NOT produce hard findings, got: {:?}",
        findings.iter().map(|f| &f.problem).collect::<Vec<_>>()
    );
}

// =====================================================================
// Safe fixtures MUST produce ZERO findings
// =====================================================================

#[test]
fn legitimate_comments_produce_zero_findings() {
    let text = load_fixture("safe/legitimate_comments.rs");
    let file = make_file("src/legit.rs", &text);
    let ctx = make_ctx(vec![file]);
    let hard = comments::findings(&ctx);
    let advisory = comments::advisory_signals(&ctx);
    assert!(
        hard.is_empty(),
        "legitimate_comments.rs must not produce hard findings, got: {:?}",
        hard.iter().map(|f| &f.problem).collect::<Vec<_>>()
    );
    assert!(
        advisory.is_empty(),
        "legitimate_comments.rs must not produce advisory signals, got: {:?}",
        advisory.iter().map(|f| &f.problem).collect::<Vec<_>>()
    );
}

#[test]
fn documentation_comments_produce_zero_findings() {
    let text = load_fixture("safe/documentation_comments.rs");
    let file = make_file("src/docs.rs", &text);
    let ctx = make_ctx(vec![file]);
    let hard = comments::findings(&ctx);
    let advisory = comments::advisory_signals(&ctx);
    assert!(
        hard.is_empty(),
        "documentation_comments.rs must not produce hard findings, got: {:?}",
        hard.iter().map(|f| &f.problem).collect::<Vec<_>>()
    );
    assert!(
        advisory.is_empty(),
        "documentation_comments.rs must not produce advisory signals, got: {:?}",
        advisory.iter().map(|f| &f.problem).collect::<Vec<_>>()
    );
}

#[test]
fn non_comment_code_produces_zero_findings() {
    let text = load_fixture("safe/non_comment_code.rs");
    let file = make_file("src/code.rs", &text);
    let ctx = make_ctx(vec![file]);
    let hard = comments::findings(&ctx);
    assert!(
        hard.is_empty(),
        "non_comment_code.rs must not produce findings from string literals, got: {:?}",
        hard.iter().map(|f| &f.problem).collect::<Vec<_>>()
    );
}

// =====================================================================
// Path filtering
// =====================================================================

#[test]
fn markdown_files_are_never_scanned() {
    let file = make_file(
        "docs/security.md",
        "skip auth check\nbypass authentication\nhardcoded password",
    );
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    assert!(
        findings.is_empty(),
        "markdown files must never produce findings"
    );
}

#[test]
fn test_files_are_never_scanned() {
    let file = make_file(
        "tests/auth_test.rs",
        "// skip auth check\n// hardcoded password\n// bypass authentication",
    );
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    assert!(
        findings.is_empty(),
        "test files must never produce findings"
    );
}

#[test]
fn fixture_files_are_never_scanned() {
    let file = make_file(
        "fixtures/bad_example.rs",
        "// skip auth check\n// hardcoded password",
    );
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    assert!(
        findings.is_empty(),
        "fixture files must never produce findings"
    );
}

#[test]
fn jankurai_self_crates_are_never_scanned() {
    let file = make_file(
        "crates/jankurai/src/scan.rs",
        "// skip auth check\n// hardcoded password",
    );
    let ctx = make_ctx(vec![file]);
    let findings = comments::findings(&ctx);
    assert!(
        findings.is_empty(),
        "jankurai's own crates must never produce findings"
    );
}
