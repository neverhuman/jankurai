use crate::audit::coverage::{
    run_coverage_audit, write_coverage_json, write_coverage_markdown, CoverageAuditOptions,
};
pub use crate::audit::coverage::{
    DEFAULT_CONFIG_PATH, DEFAULT_JSON_PATH, DEFAULT_MAX_ARTIFACT_BYTES, DEFAULT_MAX_FINDINGS,
    DEFAULT_MD_PATH,
};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CoverageAuditArgs {
    pub repo: PathBuf,
    pub config: String,
    pub json: String,
    pub md: String,
    pub changed_from: Option<String>,
    pub strict: bool,
    pub max_artifact_bytes: u64,
    pub max_findings: usize,
}

impl Default for CoverageAuditArgs {
    fn default() -> Self {
        Self {
            repo: PathBuf::from("."),
            config: DEFAULT_CONFIG_PATH.into(),
            json: DEFAULT_JSON_PATH.into(),
            md: DEFAULT_MD_PATH.into(),
            changed_from: None,
            strict: false,
            max_artifact_bytes: DEFAULT_MAX_ARTIFACT_BYTES,
            max_findings: DEFAULT_MAX_FINDINGS,
        }
    }
}

pub fn run_audit(args: CoverageAuditArgs) -> Result<()> {
    if args.json == "-" && args.md == "-" {
        anyhow::bail!("use at most one stdout target; JSON and Markdown may not share stdout");
    }
    let repo_root = args.repo.canonicalize()?;
    let audit = run_coverage_audit(CoverageAuditOptions {
        repo_root: repo_root.clone(),
        config_path: PathBuf::from(&args.config),
        changed_from: args.changed_from.clone(),
        strict: args.strict,
        max_artifact_bytes: args.max_artifact_bytes,
        max_findings: args.max_findings,
    })?;
    write_coverage_json(&PathBuf::from(&args.json), &audit)?;
    write_coverage_markdown(&PathBuf::from(&args.md), &audit)?;
    eprintln!(
        "coverage-audit status={} sources={}/{} hard={} soft={} json={} md={}",
        audit.summary.status,
        audit.summary.sources_present,
        audit.summary.sources_total,
        audit.summary.hard_findings,
        audit.summary.soft_findings,
        args.json,
        args.md
    );
    if args.strict && audit.summary.hard_findings > 0 {
        anyhow::bail!(
            "coverage audit strict mode failed: hard_findings={}",
            audit.summary.hard_findings
        );
    }
    Ok(())
}
