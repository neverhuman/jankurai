use jankurai::audit::caps;
use jankurai::audit::rules;
use regex::Regex;

#[test]
fn every_rule_id_matches_hlt_format() {
    let re = Regex::new(r"^HLT-\d{3}-[A-Z0-9-]+$").unwrap();
    for rule in rules::all() {
        assert!(
            re.is_match(rule.id),
            "Rule ID {} does not match HLT format",
            rule.id
        );
    }
}

#[test]
fn every_rule_has_required_non_empty_metadata() {
    for rule in rules::all() {
        assert!(!rule.name.is_empty(), "Rule {} missing name", rule.id);
        assert!(
            !rule.category.is_empty(),
            "Rule {} missing category",
            rule.id
        );
        assert!(!rule.tlr.is_empty(), "Rule {} missing TLR", rule.id);
        assert!(!rule.lane.is_empty(), "Rule {} missing lane", rule.id);
        assert!(
            !rule.docs_url.is_empty(),
            "Rule {} missing docs_url",
            rule.id
        );
        assert!(
            !rule.severity.is_empty(),
            "Rule {} missing severity",
            rule.id
        );
        assert!(
            !rule.repair_reason.is_empty(),
            "Rule {} missing repair_reason",
            rule.id
        );
        assert!(
            !rule.standard_section.is_empty(),
            "Rule {} missing standard_section",
            rule.id
        );
    }
}

#[test]
fn every_rule_severity_is_valid() {
    for rule in rules::all() {
        assert!(
            ["critical", "high", "medium", "low"].contains(&rule.severity),
            "Rule {} has invalid severity: {}",
            rule.id,
            rule.severity
        );
    }
}

#[test]
fn every_rule_lane_is_valid() {
    for rule in rules::all() {
        assert!(
            [
                "fast",
                "contract",
                "db",
                "db-migration-analyze",
                "web",
                "e2e",
                "security",
                "observability",
                "audit",
                "copy-code",
                "release"
            ]
            .contains(&rule.lane),
            "Rule {} has invalid lane: {}",
            rule.id,
            rule.lane
        );
    }
}

/// Rules that are intentionally still Experimental. HLT-044 and HLT-045 are the
/// advisory governance guards (worktree sprawl, generated-zone hand-edits), and
/// HLT-046/047/048 are the advisory Jankurai-pillar guards (unnecessary variety,
/// canonical README, canonical CI) that stay Experimental until each is promoted
/// with its own cap.
const EXPERIMENTAL_RULES: &[&str] = &[
    "HLT-044-WORKTREE-SPRAWL",
    "HLT-045-GENERATED-ZONE-GOVERNANCE",
    "HLT-046-UNNECESSARY-VARIETY",
    "HLT-047-CANONICAL-README",
    "HLT-048-CANONICAL-CI-GAP",
];

#[test]
fn every_rule_status_is_stable() {
    for rule in rules::all() {
        if EXPERIMENTAL_RULES.contains(&rule.id) {
            assert_eq!(
                rule.status,
                rules::RuleStatus::Experimental,
                "Rule {} is allow-listed as Experimental",
                rule.id
            );
            continue;
        }
        assert_eq!(
            rule.status,
            rules::RuleStatus::Stable,
            "Rule {} should be Stable",
            rule.id
        );
    }
}

#[test]
fn every_rule_with_cap_key_has_matching_cap_spec() {
    for rule in rules::all() {
        if let Some(cap_key) = rule.cap_key {
            assert!(
                caps::CAP_SPECS.iter().any(|c| c.key == cap_key),
                "Rule {} references unknown cap_key: {}",
                rule.id,
                cap_key
            );
        }
    }
}

#[test]
fn every_cap_spec_with_rule_id_has_matching_rule() {
    for cap in caps::CAP_SPECS {
        if let Some(rule_id) = cap.rule_id {
            assert!(
                rules::all().iter().any(|r| r.id == rule_id),
                "Cap {} references unknown rule_id: {}",
                cap.key,
                rule_id
            );
        }
    }
}

#[test]
fn every_high_or_critical_rule_has_cap_key() {
    for rule in rules::all() {
        if ["critical", "high"].contains(&rule.severity) {
            // HLT-014-A11Y-GAP is an exception as there is no specific cap for a11y yet.
            // HLT-003-OWNERLESS-PATH is dimension-driven, not cap-driven.
            // HLT-004-UNMAPPED-PROOF is dimension-driven (and soft capped).
            // HLT-017-OPAQUE-OBSERVABILITY is dimension-driven.
            // HLT-044-WORKTREE-SPRAWL is an advisory Experimental governance guard with no cap yet.
            if ![
                "HLT-014-A11Y-GAP",
                "HLT-003-OWNERLESS-PATH",
                "HLT-004-UNMAPPED-PROOF",
                "HLT-017-OPAQUE-OBSERVABILITY",
                "HLT-044-WORKTREE-SPRAWL",
            ]
            .contains(&rule.id)
            {
                assert!(
                    rule.cap_key.is_some(),
                    "High/Critical Rule {} must have a cap_key",
                    rule.id
                );
            }
        }
    }
}

#[test]
fn no_orphan_caps() {
    for cap in caps::CAP_SPECS {
        if cap.rule_id.is_none() {
            // Document the legacy non-rule caps here
            assert!(
                [
                    "non-optimal-product-language-found",
                    "too-much-python-in-product-surface",
                    "vibe-placeholders-in-product-code",
                    "fallback-soup-in-product-code",
                    "severe-duplication-in-product-code",
                    "missing-web-e2e-lane",
                    "jankurai-required-tool-ci-evidence-gap",
                    "missing-rust-property-or-integration-tests",
                    "no-agent-friendly-exception-pattern",
                    "missing-agent-readable-docs",
                    "ci-bad-behavior",
                    "git-bad-behavior"
                ]
                .contains(&cap.key),
                "Cap {} has no rule_id and is not a known legacy cap",
                cap.key
            );
        }
    }
}

#[test]
fn confidence_policy_is_consistent_with_severity() {
    for rule in rules::all() {
        match rule.severity {
            "critical" => assert_eq!(
                rule.confidence_policy,
                rules::ConfidencePolicy::High,
                "Critical rule {} must have High confidence",
                rule.id
            ),
            "high" => assert!(
                [
                    rules::ConfidencePolicy::High,
                    rules::ConfidencePolicy::Medium
                ]
                .contains(&rule.confidence_policy),
                "High severity rule {} must have High or Medium confidence",
                rule.id
            ),
            _ => {}
        }
    }
}

#[test]
fn rule_count_matches_expected() {
    let stable = rules::all()
        .iter()
        .filter(|r| r.status == rules::RuleStatus::Stable)
        .count();
    assert_eq!(stable, 43, "Expected exactly 43 stable rules");
    assert_eq!(
        rules::all().len(),
        48,
        "Expected exactly 48 rules (43 stable + 5 experimental: 2 governance guards + 3 Jankurai-pillar guards)"
    );
}

#[test]
fn cap_count_matches_expected() {
    assert_eq!(caps::CAP_SPECS.len(), 46, "Expected exactly 46 caps");
}
