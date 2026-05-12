use crate::audit::helpers::*;
use crate::audit::scan;
use crate::model::DimensionResult;

pub fn analyze(ctx: &AuditContext) -> DimensionResult {
    let files = product_code_files(ctx);
    if files.is_empty() {
        return make_dim(
            "Code shape and semantic surface",
            if ctx.self_audit { 65 } else { 90 },
            vec!["no authored adopter product code files in scope".into()],
            vec![],
        );
    }
    let mut score = 55;
    let mut evidence = vec![];
    let mut notes = vec![];
    let shape_files: Vec<_> = files
        .iter()
        .filter(|f| !(f.suffix == ".py" && python_scoring_exempt(ctx, &f.rel_path)))
        .cloned()
        .collect();
    if let Some(file) = largest_file(&shape_files) {
        evidence.push(format!(
            "largest authored code file: {} ({} LOC)",
            file.rel_path, file.line_count
        ));
    }
    if let Some(max) = max_loc(&shape_files) {
        if max > 500 {
            score -= 15;
            evidence.push("code file exceeds 500 LOC".into());
        }
        if max > 1000 {
            score -= 20;
            evidence.push("code file exceeds 1000 LOC".into());
        }
    }
    if files.len() >= 5
        && files.iter().filter(|f| f.line_count <= 300).count() * 10 / files.len() >= 7
    {
        score += 10;
        evidence.push("most code files stay under 300 LOC".into());
    }
    if !scan::duplicate_blocks(ctx).is_empty() {
        score -= 18;
        evidence.push("duplicate code block marker found".into());
    }
    if !scan::todo_hits(ctx).is_empty() {
        score -= 20;
        evidence.push("TODO/stub marker found".into());
    }
    if scan::fallback_hits(ctx).len() > 1 {
        score -= 18;
        evidence.push("fallback soup marker found".into());
    }
    if !scan::future_hostile_hits(ctx).is_empty() {
        score -= 24;
        evidence.push("future-hostile/dead-language marker found".into());
        notes.push("product/runtime code contains future-hostile or dead-language terms".into());
    }
    if !weak_name_hits(ctx).is_empty() {
        score -= 10;
        evidence.push("weak name marker found".into());
    }
    if !domain_io_hits(ctx).is_empty() {
        score -= 10;
        evidence.push("IO markers found in domain/core files".into());
    }
    if max_loc(&files).is_some_and(|max| max <= 350)
        && scan::duplicate_blocks(ctx).is_empty()
        && scan::todo_hits(ctx).is_empty()
        && scan::fallback_hits(ctx).is_empty()
        && scan::future_hostile_hits(ctx).is_empty()
        && weak_name_hits(ctx).is_empty()
        && domain_io_hits(ctx).is_empty()
    {
        score += 20;
        evidence.push("authored code stays below hard LOC limits with no shape markers".into());
    }
    let rust_summary = crate::audit::language_rules::rust::summary(ctx);
    let mut hard_language_findings = rust_summary.hard_findings;
    if rust_summary.hard_findings > 0 {
        evidence.push(format!(
            "rust bad-behavior hard findings: {}",
            rust_summary.hard_findings
        ));
        notes.push("rust hard behavior is already captured by the language-rule catalog".into());
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
            "comments",
            crate::audit::language_rules::comments::summary(ctx).hard_findings,
            crate::audit::language_rules::comments::summary(ctx).advisory_signals,
        ),
    ] {
        hard_language_findings += hard;
        if hard > 0 {
            evidence.push(format!("{label} bad-behavior hard findings: {hard}"));
        } else if advisory > 0 {
            evidence.push(format!("{label} bad-behavior advisory signals: {advisory}"));
        }
    }
    if hard_language_findings == 0 {
        score += 15;
        evidence
            .push("no hard bad-behavior findings across detector-backed language families".into());
    }
    make_dim("Code shape and semantic surface", score, evidence, notes)
}
