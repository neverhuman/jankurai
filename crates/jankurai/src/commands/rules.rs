use crate::audit::rules::{ConfidencePolicy, RepairEligibility, RepairRisk, RuleStatus};
use crate::validation::{self, ArtifactSchema};
use anyhow::Result;
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ExportArgs {
    pub repo: PathBuf,
    pub out: String,
    pub md: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerifyArgs {
    pub repo: PathBuf,
    pub out: String,
    pub md: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleRegistryReport {
    pub schema_version: String,
    pub command: String,
    pub generated_at: String,
    pub standard_version: String,
    pub auditor_version: String,
    pub rules: Vec<RuleRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleRecord {
    pub id: String,
    pub name: String,
    pub category: String,
    pub tlr: String,
    pub lane: String,
    pub docs_url: String,
    pub owner_hint: String,
    pub evidence_kind: String,
    pub severity: String,
    pub repairable: bool,
    pub repair_eligibility: String,
    pub repair_risk: String,
    pub repair_reason: String,
    pub status: String,
    pub standard_section: String,
    pub cap_key: Option<String>,
    pub confidence_policy: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleVerifyReport {
    pub schema_version: String,
    pub command: String,
    pub status: String,
    pub known_rule_count: usize,
    pub scanned_files: usize,
    pub references: Vec<RuleReference>,
    pub unknown_references: Vec<RuleReference>,
    pub missing_from_standard: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleReference {
    pub rule_id: String,
    pub path: String,
    pub line: usize,
}

pub fn run_export(args: ExportArgs) -> Result<()> {
    let report = build_registry_report();
    validation::write_json(&args.repo, ArtifactSchema::RuleRegistry, &args.out, &report)?;
    if let Some(md) = args.md.as_deref() {
        crate::render::write_markdown(md, &render_registry_markdown(&report))?;
    }
    Ok(())
}

pub fn run_verify(args: VerifyArgs) -> Result<()> {
    let report = build_verify_report(&args.repo)?;
    validation::write_json(&args.repo, ArtifactSchema::RuleVerify, &args.out, &report)?;
    if let Some(md) = args.md.as_deref() {
        crate::render::write_markdown(md, &render_verify_markdown(&report))?;
    }
    if report.status != "pass" {
        anyhow::bail!("rule reference verification failed");
    }
    Ok(())
}

pub fn build_registry_report() -> RuleRegistryReport {
    RuleRegistryReport {
        schema_version: "1.0.0".into(),
        command: "jankurai rules export".into(),
        generated_at: unix_seconds(),
        standard_version: crate::model::STANDARD_VERSION.into(),
        auditor_version: crate::model::AUDITOR_VERSION.into(),
        rules: crate::audit::rules::all()
            .iter()
            .map(|rule| RuleRecord {
                id: rule.id.into(),
                name: rule.name.into(),
                category: rule.category.into(),
                tlr: rule.tlr.into(),
                lane: rule.lane.into(),
                docs_url: rule.docs_url.into(),
                owner_hint: rule.owner_hint.into(),
                evidence_kind: rule.evidence_kind.into(),
                severity: rule.severity.into(),
                repairable: rule.repairable,
                repair_eligibility: repair_eligibility(rule.repair_eligibility).into(),
                repair_risk: repair_risk(rule.repair_risk).into(),
                repair_reason: rule.repair_reason.into(),
                status: rule_status(rule.status).into(),
                standard_section: rule.standard_section.into(),
                cap_key: rule.cap_key.map(ToString::to_string),
                confidence_policy: confidence_policy(rule.confidence_policy).into(),
            })
            .collect(),
    }
}

pub fn build_verify_report(repo: &Path) -> Result<RuleVerifyReport> {
    let known: BTreeSet<String> = crate::audit::rules::all()
        .iter()
        .map(|rule| rule.id.to_string())
        .collect();
    let references = collect_rule_references(repo)?;
    let pseudo_rule_ids = ["HLT-000-SCORE-DIMENSION"];
    let unknown_references: Vec<RuleReference> = references
        .iter()
        .filter(|reference| {
            !known.contains(&reference.rule_id)
                && !pseudo_rule_ids.contains(&reference.rule_id.as_str())
        })
        .cloned()
        .collect();
    let standard = fs::read_to_string(repo.join("agent/JANKURAI_STANDARD.md")).unwrap_or_default();
    let native = fs::read_to_string(repo.join("docs/agent-native-standard.md")).unwrap_or_default();
    let missing_from_standard: Vec<String> = known
        .iter()
        .filter(|rule_id| {
            !standard.contains(rule_id.as_str()) || !native.contains(rule_id.as_str())
        })
        .cloned()
        .collect();
    let scanned_files = references
        .iter()
        .map(|reference| reference.path.clone())
        .collect::<HashSet<_>>()
        .len();
    let status = if unknown_references.is_empty() && missing_from_standard.is_empty() {
        "pass"
    } else {
        "fail"
    };
    Ok(RuleVerifyReport {
        schema_version: "1.0.0".into(),
        command: "jankurai rules verify".into(),
        status: status.into(),
        known_rule_count: known.len(),
        scanned_files,
        references,
        unknown_references,
        missing_from_standard,
    })
}

fn collect_rule_references(repo: &Path) -> Result<Vec<RuleReference>> {
    let regex = Regex::new(r"HLT-\d{3}-[A-Z0-9-]+")?;
    let roots = ["agent", "docs", "paper"];
    let mut out = Vec::new();
    for root in roots {
        let path = repo.join(root);
        if !path.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&path)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(repo)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            if rel.starts_with("target/") || rel.ends_with(".pdf") {
                continue;
            }
            let Ok(text) = fs::read_to_string(entry.path()) else {
                continue;
            };
            for (line_index, line) in text.lines().enumerate() {
                for matched in regex.find_iter(line) {
                    out.push(RuleReference {
                        rule_id: matched.as_str().to_string(),
                        path: rel.clone(),
                        line: line_index + 1,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| {
        a.rule_id
            .cmp(&b.rule_id)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.line.cmp(&b.line))
    });
    Ok(out)
}

fn render_registry_markdown(report: &RuleRegistryReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Rule Registry");
    let _ = writeln!(out);
    let _ = writeln!(out, "- rules: `{}`", report.rules.len());
    let _ = writeln!(out);
    let _ = writeln!(out, "| Rule | Lane | Severity | Status |");
    let _ = writeln!(out, "| --- | --- | --- | --- |");
    for rule in &report.rules {
        let _ = writeln!(
            out,
            "| `{}` | `{}` | `{}` | `{}` |",
            rule.id, rule.lane, rule.severity, rule.status
        );
    }
    out
}

fn render_verify_markdown(report: &RuleVerifyReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Rule Verification");
    let _ = writeln!(out);
    let _ = writeln!(out, "- status: `{}`", report.status);
    let _ = writeln!(out, "- known rules: `{}`", report.known_rule_count);
    let _ = writeln!(out, "- scanned files: `{}`", report.scanned_files);
    let _ = writeln!(
        out,
        "- unknown references: `{}`",
        report.unknown_references.len()
    );
    let _ = writeln!(
        out,
        "- missing from standard: `{}`",
        report.missing_from_standard.join(", ")
    );
    if !report.unknown_references.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Unknown References");
        for reference in &report.unknown_references {
            let _ = writeln!(
                out,
                "- `{}` at `{}`:{}",
                reference.rule_id, reference.path, reference.line
            );
        }
    }
    out
}

fn repair_eligibility(value: RepairEligibility) -> &'static str {
    value.as_str()
}

fn repair_risk(value: RepairRisk) -> &'static str {
    value.as_str()
}

fn rule_status(value: RuleStatus) -> &'static str {
    match value {
        RuleStatus::Stable => "stable",
        RuleStatus::Experimental => "experimental",
        RuleStatus::Deprecated => "deprecated",
    }
}

fn confidence_policy(value: ConfidencePolicy) -> &'static str {
    match value {
        ConfidencePolicy::High => "high",
        ConfidencePolicy::Medium => "medium",
        ConfidencePolicy::Low => "low",
    }
}

fn unix_seconds() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
