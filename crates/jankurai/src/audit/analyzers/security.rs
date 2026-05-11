use crate::audit::helpers::*;
use crate::model::DimensionResult;

pub fn analyze(ctx: &AuditContext) -> DimensionResult {
    let mut score = 20;
    let mut evidence = vec![];
    let mut notes = vec![];
    if ctx.all_files.iter().any(|f| {
        [
            "Cargo.lock",
            "package-lock.json",
            "pnpm-lock.yaml",
            "yarn.lock",
            "poetry.lock",
            "uv.lock",
            "Gemfile.lock",
        ]
        .contains(&f.name.as_str())
    }) {
        score += 12;
        evidence.push("lockfile present".into());
    }
    let surface_text = command_surface_text(ctx);
    let security_text = security_lane_text(ctx);
    if [
        "gitleaks",
        "detect-secrets",
        "secret",
        "audit",
        "deny",
        "dependency-review",
    ]
    .iter()
    .any(|n| surface_text.contains(n))
    {
        score += 12;
        evidence.push("secret or dependency scan tooling found".into());
    }
    if ["syft", "grype", "slsa", "sbom", "cosign"]
        .iter()
        .any(|n| security_text.contains(n))
    {
        score += 8;
        evidence.push("provenance/SBOM tooling found".into());
    }
    if ["actionlint", "zizmor"]
        .iter()
        .any(|n| security_text.contains(n))
    {
        score += 8;
        evidence.push("workflow linting tooling found".into());
    }
    if has_security_lane(ctx) {
        score += 8;
        evidence.push("security lane present".into());
    } else {
        notes.push("no explicit security lane found".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.rel_path == "tools/security-lane.sh")
    {
        score += 6;
        evidence.push("canonical security lane wrapper present".into());
    }
    if has_jankurai_audit_ci_lane(ctx) {
        score += 6;
        evidence.push("agent-readiness audit gate found in CI".into());
    } else {
        score -= 6;
        notes.push("CI does not run the jankurai audit".into());
    }
    let rust_summary = crate::audit::language_rules::rust::summary(ctx);
    let mut hard_language_findings = rust_summary.hard_findings;
    if rust_summary.hard_findings > 0 {
        evidence.push(format!(
            "rust bad-behavior hard findings: {}",
            rust_summary.hard_findings
        ));
        notes.push("rust hard findings are scored through the language-rule catalog".into());
    } else if rust_summary.advisory_signals > 0 {
        evidence.push(format!(
            "rust bad-behavior advisory signals: {}",
            rust_summary.advisory_signals
        ));
    }
    for (label, hard, advisory) in [
        (
            "sql",
            crate::audit::language_rules::sql::summary(ctx).hard_findings,
            crate::audit::language_rules::sql::summary(ctx).advisory_signals,
        ),
        (
            "typescript",
            crate::audit::language_rules::typescript::summary(ctx).hard_findings,
            crate::audit::language_rules::typescript::summary(ctx).advisory_signals,
        ),
        (
            "docker",
            crate::audit::language_rules::docker::summary(ctx).hard_findings,
            crate::audit::language_rules::docker::summary(ctx).advisory_signals,
        ),
        (
            "python",
            crate::audit::language_rules::python::summary(ctx).hard_findings,
            crate::audit::language_rules::python::summary(ctx).advisory_signals,
        ),
        (
            "ci",
            crate::audit::language_rules::ci::summary(ctx).hard_findings,
            crate::audit::language_rules::ci::summary(ctx).advisory_signals,
        ),
        (
            "git",
            crate::audit::language_rules::git::summary(ctx).hard_findings,
            crate::audit::language_rules::git::summary(ctx).advisory_signals,
        ),
        (
            "gittools",
            crate::audit::language_rules::gittools::summary(ctx).hard_findings,
            crate::audit::language_rules::gittools::summary(ctx).advisory_signals,
        ),
        (
            "release",
            crate::audit::language_rules::release::summary(ctx).hard_findings,
            crate::audit::language_rules::release::summary(ctx).advisory_signals,
        ),
        (
            "web security",
            crate::audit::web_security::summary(ctx).hard_findings,
            crate::audit::web_security::summary(ctx).advisory_signals,
        ),
    ] {
        hard_language_findings += hard;
        if hard > 0 {
            evidence.push(format!("{label} bad-behavior hard findings: {hard}"));
        } else if advisory > 0 {
            evidence.push(format!("{label} bad-behavior advisory signals: {advisory}"));
        }
    }
    if security_text.contains("cargo audit") && security_text.contains("npm audit") {
        score += 8;
        evidence.push("Rust and npm dependency audits are operational commands".into());
    }
    if security_text.contains("gitleaks detect") {
        score += 6;
        evidence.push("secret scanning command is operational".into());
    }
    if hard_language_findings == 0
        && has_security_lane(ctx)
        && has_jankurai_audit_ci_lane(ctx)
        && security_text.contains("tools/security-lane.sh")
        && security_text.contains("cargo audit")
        && security_text.contains("npm audit")
        && security_text.contains("gitleaks detect")
        && ["syft", "grype", "slsa", "sbom", "cosign"]
            .iter()
            .any(|n| security_text.contains(n))
        && ["actionlint", "zizmor"]
            .iter()
            .any(|n| security_text.contains(n))
    {
        score += 8;
        evidence.push(
            "complete operational security command posture with zero hard language findings".into(),
        );
    }
    make_dim("Security and supply-chain posture", score, evidence, notes)
}
