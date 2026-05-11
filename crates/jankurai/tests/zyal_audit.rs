use jankurai::audit;
use jankurai::model::Finding;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn fixture_root() -> PathBuf {
    repo_root().join("crates/jankurai/tests/fixtures/zyal")
}

fn read_fixture(rel: &str) -> String {
    fs::read_to_string(fixture_root().join(rel)).unwrap()
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn copy_fixture(repo: &Path, fixture_rel: &str, repo_rel: &str) {
    write(&repo.join(repo_rel), &read_fixture(fixture_rel));
}

fn copy_repo_file(repo: &Path, rel: &str) {
    write(
        &repo.join(rel),
        &fs::read_to_string(repo_root().join(rel)).unwrap(),
    );
}

fn seed_standard(repo: &Path) {
    for rel in [
        "AGENTS.md",
        "README.md",
        "Justfile",
        "agent/JANKURAI_STANDARD.md",
        "agent/owner-map.json",
        "agent/test-map.json",
        "docs/agent-native-standard.md",
    ] {
        copy_repo_file(repo, rel);
    }
}

fn findings_for(repo: &Path, rule_id: &str) -> Vec<Finding> {
    audit::run_audit(repo, &[])
        .unwrap()
        .findings
        .into_iter()
        .filter(|finding| finding.rule_id.as_deref() == Some(rule_id))
        .collect()
}

fn assert_path_issue(findings: &[Finding], path: &str, needle: &str) {
    let finding = findings
        .iter()
        .find(|finding| finding.path == path)
        .unwrap_or_else(|| panic!("missing finding for {path}: {findings:?}"));
    let text = finding.evidence.join("\n").to_ascii_lowercase();
    assert!(
        text.contains(&needle.to_ascii_lowercase())
            || finding
                .problem
                .to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase()),
        "finding for {path} missing `{needle}` evidence: {finding:?}"
    );
}

#[test]
fn canonical_zyal_runbooks_do_not_emit_hlt024() {
    let repo = tempdir().unwrap();
    seed_standard(repo.path());
    copy_fixture(
        repo.path(),
        "agent/zyal/minimal.zyal",
        "agent/zyal/minimal.zyal",
    );
    copy_fixture(
        repo.path(),
        "agent/zyal/advanced-research-loop.zyal",
        "agent/zyal/advanced-research-loop.zyal",
    );

    assert!(findings_for(repo.path(), "HLT-024-AGENT-TOOL-SUPPLY-GAP").is_empty());
}

#[test]
fn legacy_extension_and_wrong_location_are_flagged() {
    let repo = tempdir().unwrap();
    seed_standard(repo.path());
    copy_fixture(
        repo.path(),
        "ops/zyal/openqg-literature-radar.zyal",
        "ops/zyal/openqg-literature-radar.zyal",
    );
    copy_fixture(
        repo.path(),
        "agent/zyal/openqg-literature-radar.zyal.yml",
        "agent/zyal/openqg-literature-radar.zyal.yml",
    );

    let findings = findings_for(repo.path(), "HLT-024-AGENT-TOOL-SUPPLY-GAP");
    assert_eq!(findings.len(), 2, "{findings:?}");
    assert_path_issue(
        &findings,
        "ops/zyal/openqg-literature-radar.zyal",
        "agent/zyal",
    );
    assert_path_issue(
        &findings,
        "agent/zyal/openqg-literature-radar.zyal.yml",
        ".zyal.yml",
    );
}

#[test]
fn invalid_envelope_and_runtime_versions_are_rejected() {
    let repo = tempdir().unwrap();
    seed_standard(repo.path());
    copy_fixture(
        repo.path(),
        "agent/zyal/wrong-runtime-sentinel.zyal",
        "agent/zyal/wrong-runtime-sentinel.zyal",
    );
    copy_fixture(
        repo.path(),
        "agent/zyal/missing-arm.zyal",
        "agent/zyal/missing-arm.zyal",
    );
    copy_fixture(
        repo.path(),
        "agent/zyal/mismatched-ids.zyal",
        "agent/zyal/mismatched-ids.zyal",
    );

    let findings = findings_for(repo.path(), "HLT-024-AGENT-TOOL-SUPPLY-GAP");
    assert_eq!(findings.len(), 3, "{findings:?}");
    assert_path_issue(
        &findings,
        "agent/zyal/wrong-runtime-sentinel.zyal",
        "runtime sentinel version",
    );
    assert_path_issue(&findings, "agent/zyal/missing-arm.zyal", "arm sentinel");
    assert_path_issue(
        &findings,
        "agent/zyal/mismatched-ids.zyal",
        "does not match",
    );
}

#[test]
fn invalid_schema_and_semantics_are_rejected() {
    let repo = tempdir().unwrap();
    seed_standard(repo.path());
    for rel in [
        "agent/zyal/unknown-top-level-key.zyal",
        "agent/zyal/duplicate-key.zyal",
        "agent/zyal/duplicate-capability-rule-id.zyal",
        "agent/zyal/invalid-regex.zyal",
        "agent/zyal/invalid-research-version.zyal",
    ] {
        copy_fixture(repo.path(), rel, rel);
    }

    let findings = findings_for(repo.path(), "HLT-024-AGENT-TOOL-SUPPLY-GAP");
    assert!(
        findings.len() >= 5,
        "expected schema/semantic findings, got {findings:?}"
    );
    assert_path_issue(
        &findings,
        "agent/zyal/unknown-top-level-key.zyal",
        "Unknown ZYAL key",
    );
    assert_path_issue(
        &findings,
        "agent/zyal/duplicate-key.zyal",
        "duplicate YAML key",
    );
    assert_path_issue(
        &findings,
        "agent/zyal/duplicate-capability-rule-id.zyal",
        "duplicated",
    );
    assert_path_issue(&findings, "agent/zyal/invalid-regex.zyal", "regex");
    assert_path_issue(
        &findings,
        "agent/zyal/invalid-research-version.zyal",
        "research.version",
    );
}

#[test]
fn code_fence_mentions_are_ignored() {
    let repo = tempdir().unwrap();
    seed_standard(repo.path());
    copy_fixture(repo.path(), "docs/notes.md", "docs/notes.md");

    assert!(findings_for(repo.path(), "HLT-024-AGENT-TOOL-SUPPLY-GAP").is_empty());
}

#[test]
fn rust_source_that_mentions_zyal_is_ignored() {
    let repo = tempdir().unwrap();
    seed_standard(repo.path());
    write(
        &repo.path().join("crates/jankurai/src/audit/zyal/core.rs"),
        r#"
pub fn helper() -> &'static str {
    "<<<ZYAL v1:daemon id=example>>>"
}
"#,
    );

    assert!(findings_for(repo.path(), "HLT-024-AGENT-TOOL-SUPPLY-GAP").is_empty());
}

#[test]
fn non_runbook_code_under_canonical_root_is_flagged() {
    let repo = tempdir().unwrap();
    seed_standard(repo.path());
    write(
        &repo.path().join("agent/zyal/helper.rs"),
        r#"
pub fn helper() -> &'static str {
    "not a runbook"
}
"#,
    );

    let findings = findings_for(repo.path(), "HLT-024-AGENT-TOOL-SUPPLY-GAP");
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_path_issue(&findings, "agent/zyal/helper.rs", "agent/zyal");
}
