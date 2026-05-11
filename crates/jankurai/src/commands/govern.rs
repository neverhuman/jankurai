use crate::commands::release_data::load_release_data;
use crate::validation::{self, ArtifactSchema};
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct GovernArgs {
    pub repo: PathBuf,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GovernancePolicy {
    pub schema_version: String,
    pub standard_version: String,
    pub effective_at: String,
    pub minimum_score: i32,
    pub fail_on: Vec<String>,
    pub advisory_on: Vec<String>,
    pub update_channel: String,
    pub rule_change_policy: RuleChangePolicy,
    pub deprecation_policy: DeprecationPolicy,
    pub exception_policy: ExceptionPolicy,
    pub security_advisory_policy: SecurityAdvisoryPolicy,
    pub rfc_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleChangePolicy {
    pub version_bump: String,
    pub migration_notes_required: bool,
    pub reviewers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeprecationPolicy {
    pub notice_period_days: i32,
    pub supported_versions: Vec<String>,
    pub removal_requires: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExceptionPolicy {
    pub timebox_days: i32,
    pub owner_required: bool,
    pub proof_required: bool,
    pub rfc_required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityAdvisoryPolicy {
    pub severity_threshold: String,
    pub response_sla_days: i32,
    pub patch_channels: Vec<String>,
}

pub fn run(args: GovernArgs) -> Result<()> {
    let policy = build_governance_policy(&args.repo)?;
    if let Some(path) = args.out.as_deref() {
        validation::write_json(&args.repo, ArtifactSchema::GovernancePolicy, path, &policy)?;
    } else {
        validation::validate_serializable(&args.repo, ArtifactSchema::GovernancePolicy, &policy)?;
        println!("{}", serde_json::to_string_pretty(&policy)?);
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&policy))?;
    }
    Ok(())
}

pub fn build_governance_policy(repo: &Path) -> Result<GovernancePolicy> {
    let release = load_release_data(repo)?;
    Ok(GovernancePolicy {
        schema_version: release.schema_version,
        standard_version: release.standard_version,
        effective_at: release.published.unwrap_or_else(now_string),
        minimum_score: 85,
        fail_on: vec!["critical".to_string(), "high".to_string()],
        advisory_on: vec!["medium".to_string(), "low".to_string()],
        update_channel: "stable".to_string(),
        rule_change_policy: RuleChangePolicy {
            version_bump: "minor-for-new-advisory-major-for-new-hard-gate".to_string(),
            migration_notes_required: true,
            reviewers: vec![
                "standard editor".to_string(),
                "auditor maintainer".to_string(),
                "security reviewer".to_string(),
            ],
        },
        deprecation_policy: DeprecationPolicy {
            notice_period_days: 90,
            supported_versions: vec!["0.4.x".to_string()],
            removal_requires: vec![
                "migration notes".to_string(),
                "release note".to_string(),
                "replacement path".to_string(),
            ],
        },
        exception_policy: ExceptionPolicy {
            timebox_days: 90,
            owner_required: true,
            proof_required: true,
            rfc_required: false,
        },
        security_advisory_policy: SecurityAdvisoryPolicy {
            severity_threshold: "high".to_string(),
            response_sla_days: 7,
            patch_channels: vec!["stable".to_string(), "lts".to_string()],
        },
        rfc_path: "docs/release-plan.md".to_string(),
    })
}

fn render_markdown(policy: &GovernancePolicy) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Governance Policy");
    let _ = writeln!(out);
    let _ = writeln!(out, "- standard version: `{}`", policy.standard_version);
    let _ = writeln!(out, "- effective at: `{}`", policy.effective_at);
    let _ = writeln!(out, "- minimum score: `{}`", policy.minimum_score);
    let _ = writeln!(out, "- fail on: `{}`", policy.fail_on.join(", "));
    let _ = writeln!(out, "- advisory on: `{}`", policy.advisory_on.join(", "));
    let _ = writeln!(out, "- update channel: `{}`", policy.update_channel);
    let _ = writeln!(
        out,
        "- exception timebox: `{}` days",
        policy.exception_policy.timebox_days
    );
    let _ = writeln!(
        out,
        "- deprecation notice: `{}` days",
        policy.deprecation_policy.notice_period_days
    );
    let _ = writeln!(out, "- RFC path: `{}`", policy.rfc_path);
    out
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
