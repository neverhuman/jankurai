use crate::audit::helpers::AuditContext;
use crate::audit::language_rules::common::{finding, sort_and_cap_findings};
use crate::audit::language_rules::{LanguageFinding, ProofWindow};
use crate::model::FileInfo;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeSet;

const HLT_RULE_ID: &str = "HLT-042-CI-LOCAL-PARITY";

#[derive(Debug, Clone, Copy, Default)]
pub struct CiLocalParitySummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn summary(ctx: &AuditContext) -> CiLocalParitySummary {
    CiLocalParitySummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: 0,
    }
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    sort_and_cap_findings(hard_findings(ctx), 25)
}

fn hard_findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    if !uses_github_actions(ctx) {
        return vec![];
    }

    let mut out = Vec::new();

    for workflow in workflow_files(ctx) {
        if !calls_ops_ci_lane(workflow) {
            out.push(finding(
                HLT_RULE_ID,
                "ci.local-parity.workflow-not-thin",
                workflow,
                first_job_line(workflow),
                "workflow inlines CI commands instead of delegating to ops/ci/*.sh",
                "without a single source of truth, local runs drift from CI and breakage is only visible after push",
                "extract the workflow steps into ops/ci/<lane>.sh and call them with `bash ops/ci/<lane>.sh`",
                ProofWindow::None,
            ));
        }
        for missing in referenced_scripts_missing(ctx, workflow) {
            out.push(finding(
                HLT_RULE_ID,
                "ci.local-parity.script-missing",
                workflow,
                1,
                format!("workflow references missing script `{missing}`"),
                "missing scripts mean the local runner cannot reproduce the CI step",
                "create the referenced ops/ci script with the same commands the workflow used to run",
                ProofWindow::None,
            ));
        }
    }

    if !file_exists(ctx, "ops/ci/lib.sh") {
        out.push(finding(
            HLT_RULE_ID,
            "ci.local-parity.lib-missing",
            workflow_anchor(ctx),
            1,
            "ops/ci/lib.sh is missing",
            "ops/ci/lib.sh is the shared helper module (artifact assertions, tool pins) every lane sources",
            "add ops/ci/lib.sh defining shared helpers and tool version pins",
            ProofWindow::None,
        ));
    }

    if !file_exists(ctx, "scripts/ci-local.sh") {
        out.push(finding(
            HLT_RULE_ID,
            "ci.local-parity.runner-missing",
            workflow_anchor(ctx),
            1,
            "scripts/ci-local.sh is missing",
            "scripts/ci-local.sh is the local entry point that delegates to the same ops/ci scripts the workflows call",
            "add scripts/ci-local.sh exposing each CI lane locally",
            ProofWindow::None,
        ));
    }

    if !file_exists(ctx, "scripts/ci-doctor.sh") {
        out.push(finding(
            HLT_RULE_ID,
            "ci.local-parity.doctor-missing",
            workflow_anchor(ctx),
            1,
            "scripts/ci-doctor.sh is missing",
            "without a doctor script, developers cannot confirm their local environment matches CI",
            "add scripts/ci-doctor.sh listing every tool the ops/ci scripts depend on",
            ProofWindow::None,
        ));
    }

    if has_rust_workspace(ctx) && !file_exists(ctx, "rust-toolchain.toml") {
        out.push(finding(
            HLT_RULE_ID,
            "ci.local-parity.toolchain-not-pinned",
            workflow_anchor(ctx),
            1,
            "rust-toolchain.toml is missing",
            "without a pinned toolchain, local and CI Rust versions can drift silently",
            "add rust-toolchain.toml pinning the channel and required components",
            ProofWindow::None,
        ));
    }

    if !file_exists(ctx, "ops/git-hooks/pre-push") {
        out.push(finding(
            HLT_RULE_ID,
            "ci.local-parity.pre-push-hook-missing",
            workflow_anchor(ctx),
            1,
            "ops/git-hooks/pre-push is missing",
            "without a mandatory pre-push gate, broken code can be pushed and CI is the first place a failure shows up",
            "add ops/git-hooks/pre-push that runs `bash ops/ci/quality-gates.sh` and wire it via `git config core.hooksPath ops/git-hooks`",
            ProofWindow::None,
        ));
    }

    out
}

fn uses_github_actions(ctx: &AuditContext) -> bool {
    !workflow_files(ctx).is_empty()
}

fn workflow_files(ctx: &AuditContext) -> Vec<&FileInfo> {
    ctx.all_files
        .iter()
        .filter(|file| is_workflow(file))
        .collect()
}

fn is_workflow(file: &FileInfo) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    lower.starts_with(".github/workflows/") && (lower.ends_with(".yml") || lower.ends_with(".yaml"))
}

static OPS_CI_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)\bbash\s+ops/ci/([A-Za-z0-9_./-]+\.sh)").expect("ops/ci regex is valid")
});

fn calls_ops_ci_lane(file: &FileInfo) -> bool {
    OPS_CI_RE.is_match(&file.text)
}

fn referenced_scripts_missing(ctx: &AuditContext, file: &FileInfo) -> Vec<String> {
    let mut missing = BTreeSet::new();
    for cap in OPS_CI_RE.captures_iter(&file.text) {
        let rel = format!("ops/ci/{}", cap.get(1).map_or("", |m| m.as_str()));
        if !file_exists(ctx, &rel) {
            missing.insert(rel);
        }
    }
    missing.into_iter().collect()
}

fn first_job_line(file: &FileInfo) -> usize {
    for (idx, line) in file.text.lines().enumerate() {
        if line.trim_start().starts_with("jobs:") {
            return idx + 1;
        }
    }
    1
}

fn file_exists(ctx: &AuditContext, rel: &str) -> bool {
    ctx.all_files.iter().any(|file| file.rel_path == rel)
}

fn workflow_anchor(ctx: &AuditContext) -> &FileInfo {
    workflow_files(ctx)
        .into_iter()
        .next()
        .unwrap_or_else(|| ctx.all_files.first().expect("ctx has at least one file"))
}

fn has_rust_workspace(ctx: &AuditContext) -> bool {
    ctx.all_files
        .iter()
        .any(|file| file.rel_path == "Cargo.toml" || file.rel_path.ends_with("/Cargo.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::helpers::AuditContext;
    use crate::model::FileInfo;

    fn file(rel: &str, text: &str) -> FileInfo {
        let name = rel.rsplit('/').next().unwrap_or(rel).to_string();
        let suffix = match rel.rfind('.') {
            Some(idx) if idx > 0 => rel[idx..].to_string(),
            _ => String::new(),
        };
        FileInfo {
            rel_path: rel.to_string(),
            name,
            suffix,
            size: text.len() as u64,
            line_count: text.lines().count(),
            text: text.to_string(),
            is_generated: false,
            is_code: false,
        }
    }

    fn ctx(files: Vec<FileInfo>) -> AuditContext {
        AuditContext {
            root: std::path::PathBuf::from("."),
            scope_paths: files.iter().map(|f| f.rel_path.clone()).collect(),
            scope_files: files.clone(),
            all_files: files,
            self_audit: false,
            boundary_reclassifications: vec![],
        }
    }

    #[test]
    fn no_workflows_means_no_findings() {
        let ctx = ctx(vec![file("README.md", "")]);
        assert_eq!(summary(&ctx).hard_findings, 0);
    }

    #[test]
    fn thin_workflow_calling_ops_ci_passes() {
        let workflow = r#"jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: bash ops/ci/quality-gates.sh
"#;
        let ctx = ctx(vec![
            file(".github/workflows/ci.yml", workflow),
            file("ops/ci/lib.sh", "set -euo pipefail"),
            file("ops/ci/quality-gates.sh", "echo gates"),
            file("ops/git-hooks/pre-push", "#!/usr/bin/env bash"),
            file("scripts/ci-local.sh", "echo local"),
            file("scripts/ci-doctor.sh", "echo doctor"),
            file("rust-toolchain.toml", "[toolchain]"),
            file("Cargo.toml", "[package]"),
        ]);
        assert_eq!(summary(&ctx).hard_findings, 0, "{:#?}", findings(&ctx));
    }

    #[test]
    fn missing_pre_push_hook_flagged() {
        let workflow = r#"jobs:
  test:
    steps:
      - run: bash ops/ci/quality-gates.sh
"#;
        let ctx = ctx(vec![
            file(".github/workflows/ci.yml", workflow),
            file("ops/ci/lib.sh", ""),
            file("ops/ci/quality-gates.sh", ""),
            file("scripts/ci-local.sh", ""),
            file("scripts/ci-doctor.sh", ""),
        ]);
        let hits = findings(&ctx);
        assert!(
            hits.iter()
                .any(|h| h.matched_term == "ci.local-parity.pre-push-hook-missing"),
            "{hits:#?}"
        );
    }

    #[test]
    fn inlined_workflow_flagged() {
        let workflow = r#"jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test
"#;
        let ctx = ctx(vec![
            file(".github/workflows/ci.yml", workflow),
            file("ops/ci/lib.sh", ""),
            file("scripts/ci-local.sh", ""),
            file("scripts/ci-doctor.sh", ""),
            file("rust-toolchain.toml", ""),
        ]);
        let hits = findings(&ctx);
        assert!(
            hits.iter()
                .any(|h| h.matched_term == "ci.local-parity.workflow-not-thin"),
            "{hits:#?}"
        );
    }

    #[test]
    fn missing_referenced_script_flagged() {
        let workflow = r#"jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: bash ops/ci/quality-gates.sh
"#;
        let ctx = ctx(vec![
            file(".github/workflows/ci.yml", workflow),
            file("ops/ci/lib.sh", ""),
            file("scripts/ci-local.sh", ""),
            file("scripts/ci-doctor.sh", ""),
        ]);
        let hits = findings(&ctx);
        assert!(
            hits.iter()
                .any(|h| h.matched_term == "ci.local-parity.script-missing"
                    && h.problem.contains("quality-gates.sh")),
            "{hits:#?}"
        );
    }

    #[test]
    fn missing_doctor_flagged() {
        let workflow = r#"jobs:
  test:
    steps:
      - run: bash ops/ci/quality-gates.sh
"#;
        let ctx = ctx(vec![
            file(".github/workflows/ci.yml", workflow),
            file("ops/ci/lib.sh", ""),
            file("ops/ci/quality-gates.sh", ""),
            file("scripts/ci-local.sh", ""),
        ]);
        let hits = findings(&ctx);
        assert!(
            hits.iter()
                .any(|h| h.matched_term == "ci.local-parity.doctor-missing"),
            "{hits:#?}"
        );
    }

    #[test]
    fn rust_repo_without_toolchain_flagged() {
        let workflow = r#"jobs:
  test:
    steps:
      - run: bash ops/ci/quality-gates.sh
"#;
        let ctx = ctx(vec![
            file(".github/workflows/ci.yml", workflow),
            file("ops/ci/lib.sh", ""),
            file("ops/ci/quality-gates.sh", ""),
            file("scripts/ci-local.sh", ""),
            file("scripts/ci-doctor.sh", ""),
            file("Cargo.toml", ""),
        ]);
        let hits = findings(&ctx);
        assert!(
            hits.iter()
                .any(|h| h.matched_term == "ci.local-parity.toolchain-not-pinned"),
            "{hits:#?}"
        );
    }
}
