use super::helpers::*;
use super::scan;

pub struct CapSpec {
    pub key: &'static str,
    pub max_score: i32,
    pub rule_id: Option<&'static str>,
    pub hardness: &'static str,
}

pub const CAP_SPECS: &[CapSpec] = &[
    CapSpec {
        key: "no-root-agent-instructions",
        max_score: 75,
        rule_id: Some("HLT-015-CONTEXT-SETUP-GAP"),
        hardness: "soft",
    },
    CapSpec {
        key: "no-one-command-setup-or-validation",
        max_score: 70,
        rule_id: Some("HLT-004-UNMAPPED-PROOF"),
        hardness: "soft",
    },
    CapSpec {
        key: "no-deterministic-fast-lane",
        max_score: 65,
        rule_id: Some("HLT-004-UNMAPPED-PROOF"),
        hardness: "soft",
    },
    CapSpec {
        key: "no-security-lane-on-high-risk-repo",
        max_score: 60,
        rule_id: Some("HLT-009-GENERATED-SECURITY"),
        hardness: "hard",
    },
    CapSpec {
        key: "generated-contracts-or-public-api-drift-untested",
        max_score: 80,
        rule_id: Some("HLT-007-HANDWRITTEN-CONTRACT"),
        hardness: "hard",
    },
    CapSpec {
        key: "python-direct-product-truth-or-db-ownership",
        max_score: 72,
        rule_id: Some("HLT-005-PYTHON-PRODUCT-TRUTH"),
        hardness: "hard",
    },
    CapSpec {
        key: "no-secret-or-dependency-scanning-in-ci",
        max_score: 78,
        rule_id: Some("HLT-016-SUPPLY-CHAIN-DRIFT"),
        hardness: "hard",
    },
    CapSpec {
        key: "no-jankurai-audit-lane-in-ci",
        max_score: 82,
        rule_id: Some("HLT-020-CI-HARDENING-GAP"),
        hardness: "hard",
    },
    CapSpec {
        key: "jankurai-required-tool-ci-evidence-gap",
        max_score: 88,
        rule_id: None,
        hardness: "soft",
    },
    CapSpec {
        key: "non-optimal-product-language-found",
        max_score: 74,
        rule_id: None,
        hardness: "soft",
    },
    CapSpec {
        key: "too-much-python-in-product-surface",
        max_score: 72,
        rule_id: None,
        hardness: "soft",
    },
    CapSpec {
        key: "boundary-reclassification-evidence-gap",
        max_score: 72,
        rule_id: Some("HLT-028-BOUNDARY-EVIDENCE-GAP"),
        hardness: "hard",
    },
    CapSpec {
        key: "vibe-placeholders-in-product-code",
        max_score: 68,
        rule_id: None,
        hardness: "soft",
    },
    CapSpec {
        key: "fallback-soup-in-product-code",
        max_score: 70,
        rule_id: None,
        hardness: "soft",
    },
    CapSpec {
        key: "future-hostile-dead-language-in-product-code",
        max_score: 64,
        rule_id: Some("HLT-001-DEAD-MARKER"),
        hardness: "hard",
    },
    CapSpec {
        key: "severe-duplication-in-product-code",
        max_score: 70,
        rule_id: None,
        hardness: "soft",
    },
    CapSpec {
        key: "generated-zone-mutation-risk",
        max_score: 76,
        rule_id: Some("HLT-002-GENERATED-MUTATION"),
        hardness: "hard",
    },
    CapSpec {
        key: "direct-db-access-from-wrong-layer",
        max_score: 66,
        rule_id: Some("HLT-006-DIRECT-DB-WRONG-LAYER"),
        hardness: "hard",
    },
    CapSpec {
        key: "missing-web-e2e-lane",
        max_score: 82,
        rule_id: None,
        hardness: "soft",
    },
    CapSpec {
        key: "missing-rendered-ux-qa-lane",
        max_score: 84,
        rule_id: Some("HLT-013-RENDERED-UX-GAP"),
        hardness: "hard",
    },
    CapSpec {
        key: "prompt-injection-risk",
        max_score: 78,
        rule_id: Some("HLT-011-PROMPT-INJECTION"),
        hardness: "hard",
    },
    CapSpec {
        key: "overbroad-agent-agency",
        max_score: 65,
        rule_id: Some("HLT-012-OVERBROAD-AGENCY"),
        hardness: "hard",
    },
    CapSpec {
        key: "secret-like-content-detected",
        max_score: 60,
        rule_id: Some("HLT-010-SECRET-SPRAWL"),
        hardness: "hard",
    },
    CapSpec {
        key: "false-green-test-risk",
        max_score: 76,
        rule_id: Some("HLT-008-FALSE-GREEN-RISK"),
        hardness: "hard",
    },
    CapSpec {
        key: "destructive-migration-risk",
        max_score: 70,
        rule_id: Some("HLT-021-DESTRUCTIVE-MIGRATION"),
        hardness: "hard",
    },
    CapSpec {
        key: "authz-or-data-isolation-gap",
        max_score: 78,
        rule_id: Some("HLT-022-AUTHZ-ISOLATION-GAP"),
        hardness: "hard",
    },
    CapSpec {
        key: "input-boundary-gap",
        max_score: 78,
        rule_id: Some("HLT-023-INPUT-BOUNDARY-GAP"),
        hardness: "hard",
    },
    CapSpec {
        key: "agent-tool-supply-chain-gap",
        max_score: 78,
        rule_id: Some("HLT-024-AGENT-TOOL-SUPPLY-GAP"),
        hardness: "hard",
    },
    CapSpec {
        key: "release-readiness-gap",
        max_score: 80,
        rule_id: Some("HLT-025-RELEASE-READINESS-GAP"),
        hardness: "hard",
    },
    CapSpec {
        key: "missing-rust-property-or-integration-tests",
        max_score: 82,
        rule_id: None,
        hardness: "soft",
    },
    CapSpec {
        key: "no-agent-friendly-exception-pattern",
        max_score: 76,
        rule_id: None,
        hardness: "soft",
    },
    CapSpec {
        key: "missing-agent-readable-docs",
        max_score: 80,
        rule_id: None,
        hardness: "soft",
    },
    CapSpec {
        key: "streaming-runtime-drift",
        max_score: 78,
        rule_id: Some("HLT-019-STREAMING-RUNTIME-DRIFT"),
        hardness: "hard",
    },
    CapSpec {
        key: "rust-bad-behavior",
        max_score: 72,
        rule_id: Some("HLT-029-RUST-BAD-BEHAVIOR"),
        hardness: "hard",
    },
    CapSpec {
        key: "sql-bad-behavior",
        max_score: 72,
        rule_id: Some("HLT-030-SQL-BAD-BEHAVIOR"),
        hardness: "hard",
    },
    CapSpec {
        key: "typescript-bad-behavior",
        max_score: 72,
        rule_id: Some("HLT-031-TYPESCRIPT-BAD-BEHAVIOR"),
        hardness: "hard",
    },
    CapSpec {
        key: "docker-bad-behavior",
        max_score: 72,
        rule_id: Some("HLT-032-DOCKER-BAD-BEHAVIOR"),
        hardness: "hard",
    },
    CapSpec {
        key: "python-bad-behavior",
        max_score: 72,
        rule_id: Some("HLT-033-PYTHON-BAD-BEHAVIOR"),
        hardness: "hard",
    },
    CapSpec {
        key: "ci-bad-behavior",
        max_score: 70,
        rule_id: Some("HLT-034-CI-BAD-BEHAVIOR"),
        hardness: "hard",
    },
    CapSpec {
        key: "git-bad-behavior",
        max_score: 70,
        rule_id: Some("HLT-035-GIT-BAD-BEHAVIOR"),
        hardness: "hard",
    },
    CapSpec {
        key: "gittools-bad-behavior",
        max_score: 70,
        rule_id: Some("HLT-036-GITTOOLS-BAD-BEHAVIOR"),
        hardness: "hard",
    },
    CapSpec {
        key: "release-bad-behavior",
        max_score: 70,
        rule_id: Some("HLT-037-RELEASE-BAD-BEHAVIOR"),
        hardness: "hard",
    },
    CapSpec {
        key: "web-security-bad-behavior",
        max_score: 68,
        rule_id: Some("HLT-039-WEB-SECURITY-BAD-BEHAVIOR"),
        hardness: "hard",
    },
    CapSpec {
        key: "repo-rot-bad-behavior",
        max_score: 88,
        rule_id: Some("HLT-040-REPO-ROT-BAD-BEHAVIOR"),
        hardness: "soft",
    },
    CapSpec {
        key: "comment-hygiene-dangerous-residue",
        max_score: 72,
        rule_id: Some("HLT-041-COMMENT-HYGIENE"),
        hardness: "hard",
    },
    CapSpec {
        key: "ci-local-parity",
        max_score: 70,
        rule_id: Some("HLT-042-CI-LOCAL-PARITY"),
        hardness: "hard",
    },
];

pub const CAPS: &[(&str, i32)] = &[
    ("no-root-agent-instructions", 75),
    ("no-one-command-setup-or-validation", 70),
    ("no-deterministic-fast-lane", 65),
    ("no-security-lane-on-high-risk-repo", 60),
    ("generated-contracts-or-public-api-drift-untested", 80),
    ("python-direct-product-truth-or-db-ownership", 72),
    ("no-secret-or-dependency-scanning-in-ci", 78),
    ("no-jankurai-audit-lane-in-ci", 82),
    ("jankurai-required-tool-ci-evidence-gap", 88),
    ("non-optimal-product-language-found", 74),
    ("too-much-python-in-product-surface", 72),
    ("boundary-reclassification-evidence-gap", 72),
    ("vibe-placeholders-in-product-code", 68),
    ("fallback-soup-in-product-code", 70),
    ("future-hostile-dead-language-in-product-code", 64),
    ("severe-duplication-in-product-code", 70),
    ("generated-zone-mutation-risk", 76),
    ("direct-db-access-from-wrong-layer", 66),
    ("missing-web-e2e-lane", 82),
    ("missing-rendered-ux-qa-lane", 84),
    ("prompt-injection-risk", 78),
    ("overbroad-agent-agency", 65),
    ("secret-like-content-detected", 60),
    ("false-green-test-risk", 76),
    ("destructive-migration-risk", 70),
    ("authz-or-data-isolation-gap", 78),
    ("input-boundary-gap", 78),
    ("agent-tool-supply-chain-gap", 78),
    ("release-readiness-gap", 80),
    ("missing-rust-property-or-integration-tests", 82),
    ("no-agent-friendly-exception-pattern", 76),
    ("missing-agent-readable-docs", 80),
    ("streaming-runtime-drift", 78),
    ("rust-bad-behavior", 72),
    ("sql-bad-behavior", 72),
    ("typescript-bad-behavior", 72),
    ("docker-bad-behavior", 72),
    ("python-bad-behavior", 72),
    ("ci-bad-behavior", 70),
    ("git-bad-behavior", 70),
    ("gittools-bad-behavior", 70),
    ("release-bad-behavior", 70),
    ("web-security-bad-behavior", 68),
    ("repo-rot-bad-behavior", 88),
    ("comment-hygiene-dangerous-residue", 72),
    ("ci-local-parity", 70),
];

pub fn caps_applied(ctx: &AuditContext, has_destructive_migration_sql: bool) -> Vec<String> {
    let mut caps = Vec::new();
    if !has_root_agents(ctx) {
        caps.push("no-root-agent-instructions".into());
    }
    if !has_one_command(ctx) {
        caps.push("no-one-command-setup-or-validation".into());
    }
    if !has_fast_lane(ctx) {
        caps.push("no-deterministic-fast-lane".into());
    }
    if is_high_risk_repo(ctx) && !has_security_lane(ctx) {
        caps.push("no-security-lane-on-high-risk-repo".into());
    }
    if (has_contract_surface(ctx) || has_polyglot_boundary(ctx))
        && !(has_generated_contracts(ctx) || has_api_drift_checks(ctx))
    {
        caps.push("generated-contracts-or-public-api-drift-untested".into());
    }
    let bad_python = bad_python_path_hits(ctx);
    if !bad_python.is_empty() && !all_files_suppressed_for_cap(ctx, &bad_python, PYTHON_DIRECT_CAP)
    {
        caps.push("python-direct-product-truth-or-db-ownership".into());
    }
    if is_high_risk_repo(ctx) && !has_secret_or_dependency_scans(ctx) {
        caps.push("no-secret-or-dependency-scanning-in-ci".into());
    }
    if is_high_risk_repo(ctx) && !has_jankurai_audit_ci_lane(ctx) {
        caps.push("no-jankurai-audit-lane-in-ci".into());
    }
    if !crate::audit::analyzers::tool_adoption::missing_required_ci_tools(ctx).is_empty() {
        caps.push("jankurai-required-tool-ci-evidence-gap".into());
    }
    let non_optimal = non_optimal_language_hits(ctx);
    if !non_optimal.is_empty()
        && !all_files_suppressed_for_cap(ctx, &non_optimal, NON_OPTIMAL_LANGUAGE_CAP)
    {
        caps.push("non-optimal-product-language-found".into());
    }
    if python_ratio(ctx) > 0.15 && !python_ratio_cap_suppressed(ctx) {
        caps.push("too-much-python-in-product-surface".into());
    }
    if has_boundary_reclassification_gap(ctx) {
        caps.push("boundary-reclassification-evidence-gap".into());
    }
    if !scan::todo_hits(ctx).is_empty() {
        caps.push("vibe-placeholders-in-product-code".into());
    }
    if scan::fallback_hits(ctx).len() > 1 {
        caps.push("fallback-soup-in-product-code".into());
    }
    if !scan::future_hostile_hits(ctx).is_empty() {
        caps.push("future-hostile-dead-language-in-product-code".into());
    }
    if ctx
        .copy_code
        .as_ref()
        .is_some_and(|report| report.summary.hard_classes > 0)
    {
        caps.push("severe-duplication-in-product-code".into());
    }
    if !scan::generated_zone_issues(ctx).is_empty()
        || !scan::generated_zone_manifest_metadata_issues(ctx).is_empty()
    {
        caps.push("generated-zone-mutation-risk".into());
    }
    if !scan::wrong_layer_db_hits(ctx).is_empty() {
        caps.push("direct-db-access-from-wrong-layer".into());
    }
    if has_web_surface(ctx) && !has_playwright_e2e(ctx) {
        caps.push("missing-web-e2e-lane".into());
    }
    if has_web_surface(ctx) && !super::analyzers::ux_qa_status(ctx).has_rendered_ux_lane {
        caps.push("missing-rendered-ux-qa-lane".into());
    }
    if !scan::prompt_injection_hits(ctx).is_empty() {
        caps.push("prompt-injection-risk".into());
    }
    if !scan::agency_hits(ctx).is_empty() {
        caps.push("overbroad-agent-agency".into());
    }
    if !scan::secret_hits(ctx).is_empty() {
        caps.push("secret-like-content-detected".into());
    }
    if !scan::false_green_hits(ctx).is_empty() {
        caps.push("false-green-test-risk".into());
    }
    if has_destructive_migration_sql {
        caps.push("destructive-migration-risk".into());
    }
    if !scan::authz_isolation_hits(ctx).is_empty() {
        caps.push("authz-or-data-isolation-gap".into());
    }
    if !scan::input_boundary_hits(ctx).is_empty() {
        caps.push("input-boundary-gap".into());
    }
    if !scan::agent_tool_supply_hits(ctx).is_empty()
        || crate::audit::zyal::summary(ctx).hard_findings > 0
    {
        caps.push("agent-tool-supply-chain-gap".into());
    }
    if !scan::release_readiness_hits(ctx).is_empty() {
        caps.push("release-readiness-gap".into());
    }
    if has_rust_surface(ctx) && (!has_rust_property_tests(ctx) || !has_rust_integration_tests(ctx))
    {
        caps.push("missing-rust-property-or-integration-tests".into());
    }
    if !has_agent_friendly_exceptions(ctx) && !product_code_files(ctx).is_empty() {
        caps.push("no-agent-friendly-exception-pattern".into());
    }
    if !missing_core_docs(ctx).is_empty() {
        caps.push("missing-agent-readable-docs".into());
    }
    if !scan::streaming_runtime_hits(ctx).is_empty() {
        caps.push("streaming-runtime-drift".into());
    }
    if crate::audit::language_rules::rust::summary(ctx).hard_findings > 0 {
        caps.push("rust-bad-behavior".into());
    }
    if crate::audit::language_rules::sql::summary(ctx).hard_findings > 0 {
        caps.push("sql-bad-behavior".into());
    }
    if crate::audit::language_rules::typescript::summary(ctx).hard_findings > 0 {
        caps.push("typescript-bad-behavior".into());
    }
    if crate::audit::language_rules::docker::summary(ctx).hard_findings > 0 {
        caps.push("docker-bad-behavior".into());
    }
    if crate::audit::language_rules::python::summary(ctx).hard_findings > 0 {
        caps.push("python-bad-behavior".into());
    }
    if crate::audit::language_rules::ci::summary(ctx).hard_findings > 0 {
        caps.push("ci-bad-behavior".into());
    }
    if crate::audit::language_rules::git::summary(ctx).hard_findings > 0 {
        caps.push("git-bad-behavior".into());
    }
    if crate::audit::language_rules::gittools::summary(ctx).hard_findings > 0 {
        caps.push("gittools-bad-behavior".into());
    }
    if crate::audit::language_rules::release::summary(ctx).hard_findings > 0 {
        caps.push("release-bad-behavior".into());
    }
    if crate::audit::web_security::summary(ctx).hard_findings > 0 {
        caps.push("web-security-bad-behavior".into());
    }
    if crate::audit::repo_rot::summary(ctx).hard_findings > 0 {
        caps.push("repo-rot-bad-behavior".into());
    }
    if crate::audit::language_rules::comments::summary(ctx).hard_findings > 0 {
        caps.push("comment-hygiene-dangerous-residue".into());
    }
    if crate::audit::ci_local_parity::summary(ctx).hard_findings > 0 {
        caps.push("ci-local-parity".into());
    }
    caps
}
