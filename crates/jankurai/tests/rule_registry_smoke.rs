use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use jankurai::audit::{rule_registry, rules};

#[test]
fn rule_registry_ids_are_unique() {
    let mut seen = HashSet::new();
    for rule in rule_registry() {
        assert!(
            seen.insert(rule.id),
            "duplicate rule id in registry: {}",
            rule.id
        );
    }
}

#[test]
fn hlt014_a11y_gap_is_registered() {
    let rule = rules::lookup("HLT-014-A11Y-GAP").expect("HLT-014-A11Y-GAP must exist in registry");
    assert_eq!(rule.id, "HLT-014-A11Y-GAP");
    assert_eq!(rule.category, "ux-qa");
    assert_eq!(rule.lane, "web");
}

#[test]
fn hlt021_destructive_migration_is_registered() {
    let rule = rules::lookup("HLT-021-DESTRUCTIVE-MIGRATION")
        .expect("HLT-021-DESTRUCTIVE-MIGRATION must exist in registry");
    assert_eq!(rule.id, "HLT-021-DESTRUCTIVE-MIGRATION");
    assert_eq!(rule.category, "data");
    assert_eq!(rule.lane, "db-migration-analyze");
}

#[test]
fn vibe_coverage_rules_are_registered() {
    for (rule_id, lane) in [
        ("HLT-022-AUTHZ-ISOLATION-GAP", "db"),
        ("HLT-023-INPUT-BOUNDARY-GAP", "security"),
        ("HLT-024-AGENT-TOOL-SUPPLY-GAP", "security"),
        ("HLT-025-RELEASE-READINESS-GAP", "release"),
        ("HLT-026-COST-BUDGET-GAP", "release"),
        ("HLT-027-HUMAN-REVIEW-EVIDENCE-GAP", "audit"),
    ] {
        let rule = rules::lookup(rule_id).unwrap_or_else(|| panic!("{rule_id} must exist"));
        assert_eq!(rule.id, rule_id);
        assert_eq!(rule.lane, lane);
        assert_eq!(rule.status, rules::RuleStatus::Stable);
        assert!(!rule.tlr.trim().is_empty());
        assert!(!rule.docs_url.trim().is_empty());
    }
}

#[test]
fn boundary_evidence_gap_rule_is_registered() {
    let rule = rules::lookup("HLT-028-BOUNDARY-EVIDENCE-GAP")
        .expect("HLT-028-BOUNDARY-EVIDENCE-GAP must exist in registry");
    assert_eq!(rule.category, "boundary");
    assert_eq!(rule.cap_key, Some("boundary-reclassification-evidence-gap"));
    assert_eq!(rule.status, rules::RuleStatus::Stable);
}

#[test]
fn reference_profile_structure_rule_is_registered() {
    let rule = rules::lookup("HLT-038-REFERENCE-PROFILE-STRUCTURE-GAP")
        .expect("HLT-038-REFERENCE-PROFILE-STRUCTURE-GAP must exist in registry");
    assert_eq!(rule.category, "context");
    assert_eq!(rule.lane, "fast");
    assert_eq!(rule.cap_key, None);
    assert_eq!(rule.status, rules::RuleStatus::Stable);
}

#[test]
fn web_security_and_repo_rot_rules_are_registered() {
    let web = rules::lookup("HLT-039-WEB-SECURITY-BAD-BEHAVIOR")
        .expect("HLT-039-WEB-SECURITY-BAD-BEHAVIOR must exist in registry");
    assert_eq!(web.category, "security");
    assert_eq!(web.lane, "security");
    assert_eq!(web.cap_key, Some("web-security-bad-behavior"));
    assert_eq!(web.status, rules::RuleStatus::Stable);

    let rot = rules::lookup("HLT-040-REPO-ROT-BAD-BEHAVIOR")
        .expect("HLT-040-REPO-ROT-BAD-BEHAVIOR must exist in registry");
    assert_eq!(rot.category, "context");
    assert_eq!(rot.lane, "audit");
    assert_eq!(rot.cap_key, Some("repo-rot-bad-behavior"));
    assert_eq!(rot.status, rules::RuleStatus::Stable);
}

#[test]
fn copy_code_rule_is_registered() {
    let rule = rules::lookup("HLT-043-COPY-PASTE-BAD-BEHAVIOR")
        .expect("HLT-043-COPY-PASTE-BAD-BEHAVIOR must exist in registry");
    assert_eq!(rule.category, "copy-code");
    assert_eq!(rule.lane, "copy-code");
    assert_eq!(rule.cap_key, Some("severe-duplication-in-product-code"));
    assert_eq!(rule.docs_url, "docs/BAD_COPY.md");
    assert_eq!(rule.status, rules::RuleStatus::Stable);
}

#[test]
fn language_bad_behavior_rules_are_registered() {
    for (rule_id, category, lane, cap_key) in [
        (
            "HLT-029-RUST-BAD-BEHAVIOR",
            "security",
            "fast",
            Some("rust-bad-behavior"),
        ),
        (
            "HLT-030-SQL-BAD-BEHAVIOR",
            "data",
            "db",
            Some("sql-bad-behavior"),
        ),
        (
            "HLT-031-TYPESCRIPT-BAD-BEHAVIOR",
            "boundary",
            "fast",
            Some("typescript-bad-behavior"),
        ),
        (
            "HLT-032-DOCKER-BAD-BEHAVIOR",
            "security",
            "security",
            Some("docker-bad-behavior"),
        ),
        (
            "HLT-033-PYTHON-BAD-BEHAVIOR",
            "python",
            "contract",
            Some("python-bad-behavior"),
        ),
        (
            "HLT-034-CI-BAD-BEHAVIOR",
            "security",
            "security",
            Some("ci-bad-behavior"),
        ),
        (
            "HLT-035-GIT-BAD-BEHAVIOR",
            "agent",
            "audit",
            Some("git-bad-behavior"),
        ),
        (
            "HLT-036-GITTOOLS-BAD-BEHAVIOR",
            "agent",
            "audit",
            Some("gittools-bad-behavior"),
        ),
        (
            "HLT-037-RELEASE-BAD-BEHAVIOR",
            "release",
            "release",
            Some("release-bad-behavior"),
        ),
    ] {
        let rule = rules::lookup(rule_id).unwrap_or_else(|| panic!("{rule_id} must exist"));
        assert_eq!(rule.category, category);
        assert_eq!(rule.lane, lane);
        assert_eq!(rule.cap_key, cap_key);
        assert_eq!(rule.status, rules::RuleStatus::Stable);
        assert!(!rule.docs_url.trim().is_empty());
    }
}

#[test]
fn every_rule_has_repair_policy_metadata() {
    for rule in rules::all() {
        assert!(
            !rule.repair_reason.trim().is_empty(),
            "{} has empty repair reason",
            rule.id
        );
        assert!(
            matches!(
                rule.repair_eligibility.as_str(),
                "auto-safe" | "agent-assisted" | "never-auto"
            ),
            "{} has invalid repair eligibility",
            rule.id
        );
        assert!(
            matches!(
                rule.repair_risk.as_str(),
                "low" | "medium" | "high" | "critical"
            ),
            "{} has invalid repair risk",
            rule.id
        );
    }
}

#[test]
fn every_rule_id_is_documented_in_the_standard() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let standard = fs::read_to_string(repo.join("docs/agent-native-standard.md")).unwrap();
    let brief = fs::read_to_string(repo.join("agent/JANKURAI_STANDARD.md")).unwrap();
    for rule in rules::all() {
        assert!(
            standard.contains(rule.id),
            "{} missing from docs/agent-native-standard.md",
            rule.id
        );
        assert!(
            brief.contains(rule.id),
            "{} missing from agent/JANKURAI_STANDARD.md",
            rule.id
        );
    }
}

#[test]
fn rules_export_and_verify_emit_schema_valid_artifacts() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let out_dir = tempfile::tempdir().unwrap();
    let registry = out_dir.path().join("rule-registry.json");
    let verify = out_dir.path().join("rules-verify.json");
    assert!(Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("rules")
        .arg("export")
        .arg(&repo)
        .arg("--out")
        .arg(&registry)
        .status()
        .unwrap()
        .success());
    assert!(Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("rules")
        .arg("verify")
        .arg(&repo)
        .arg("--out")
        .arg(&verify)
        .status()
        .unwrap()
        .success());
    let registry_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(registry).unwrap()).unwrap();
    let verify_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(verify).unwrap()).unwrap();
    jankurai::validation::validate_value(
        &repo,
        jankurai::validation::ArtifactSchema::RuleRegistry,
        &registry_value,
    )
    .unwrap();
    jankurai::validation::validate_value(
        &repo,
        jankurai::validation::ArtifactSchema::RuleVerify,
        &verify_value,
    )
    .unwrap();
    assert_eq!(verify_value["status"], "pass");
}
