use crate::score_history::{self, HistorySource};
use crate::validation::{self, ArtifactSchema};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DiffArgs {
    pub base: PathBuf,
    pub head: PathBuf,
    pub out: String,
    pub md: String,
}

#[derive(Debug, Clone)]
pub struct TrendArgs {
    pub history: PathBuf,
    pub window: usize,
    pub out: String,
    pub md: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreDiffReport {
    pub schema_version: String,
    pub command: String,
    pub base_report: String,
    pub head_report: String,
    pub base_score: i32,
    pub head_score: i32,
    pub score_delta: i32,
    pub base_raw_score: i32,
    pub head_raw_score: i32,
    pub raw_score_delta: i32,
    pub caps_added: Vec<String>,
    pub caps_removed: Vec<String>,
    pub new_findings: Vec<FindingSummary>,
    pub resolved_findings: Vec<FindingSummary>,
    pub carried_findings: Vec<FindingSummary>,
    pub new_high_or_critical: usize,
    pub resolved_high_or_critical: usize,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreTrendReport {
    pub schema_version: String,
    pub command: String,
    pub history: String,
    pub window: usize,
    pub source: HistorySource,
    pub repo_id: Option<String>,
    pub history_bytes: usize,
    pub sample_count: usize,
    pub first_score: Option<i32>,
    pub latest_score: Option<i32>,
    pub score_delta: Option<i32>,
    pub best_score: Option<i32>,
    pub worst_score: Option<i32>,
    pub latest_decision: Option<String>,
    pub latest_generated_at: Option<String>,
    pub latest_commit: Option<String>,
    pub high_or_critical_latest: usize,
    pub recurrence_counts: Vec<RecurrenceCount>,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSummary {
    pub key: String,
    pub fingerprint: Option<String>,
    pub rule_id: Option<String>,
    pub severity: String,
    pub path: String,
    pub problem: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecurrenceCount {
    pub key: String,
    pub count: usize,
}

pub fn run_diff(args: DiffArgs) -> Result<()> {
    let repo = std::env::current_dir()?;
    let base = load_json(&args.base)?;
    let head = load_json(&args.head)?;
    let report = build_diff_report(&args.base, &args.head, &base, &head);
    validation::write_json(&repo, ArtifactSchema::ScoreDiff, &args.out, &report)?;
    crate::render::write_markdown(&args.md, &render_diff_markdown(&report))?;
    Ok(())
}

pub fn run_trend(args: TrendArgs) -> Result<()> {
    let repo = std::env::current_dir()?;
    let report = build_trend_report(&args.history, args.window)?;
    validation::write_json(&repo, ArtifactSchema::ScoreTrend, &args.out, &report)?;
    crate::render::write_markdown(&args.md, &render_trend_markdown(&report))?;
    Ok(())
}

pub fn build_diff_report(
    base_path: &Path,
    head_path: &Path,
    base: &Value,
    head: &Value,
) -> ScoreDiffReport {
    let base_score = int_field(base, "score");
    let head_score = int_field(head, "score");
    let base_raw_score = int_field(base, "raw_score");
    let head_raw_score = int_field(head, "raw_score");
    let base_caps = string_set(base.get("caps_applied"));
    let head_caps = string_set(head.get("caps_applied"));
    let base_findings = finding_map(base);
    let head_findings = finding_map(head);

    let caps_added = diff_set(&head_caps, &base_caps);
    let caps_removed = diff_set(&base_caps, &head_caps);
    let mut new_findings = Vec::new();
    let mut resolved_findings = Vec::new();
    let mut carried_findings = Vec::new();
    for (key, finding) in &head_findings {
        if base_findings.contains_key(key) {
            carried_findings.push(finding.clone());
        } else {
            new_findings.push(finding.clone());
        }
    }
    for (key, finding) in &base_findings {
        if !head_findings.contains_key(key) {
            resolved_findings.push(finding.clone());
        }
    }
    new_findings.sort_by(|a, b| a.key.cmp(&b.key));
    resolved_findings.sort_by(|a, b| a.key.cmp(&b.key));
    carried_findings.sort_by(|a, b| a.key.cmp(&b.key));
    let new_high_or_critical = new_findings
        .iter()
        .filter(|finding| is_high_or_critical(&finding.severity))
        .count();
    let resolved_high_or_critical = resolved_findings
        .iter()
        .filter(|finding| is_high_or_critical(&finding.severity))
        .count();
    let decision = if head_score < base_score || new_high_or_critical > 0 {
        "ratchet_fail"
    } else if !caps_added.is_empty() || !new_findings.is_empty() {
        "review"
    } else {
        "pass"
    };
    ScoreDiffReport {
        schema_version: "1.0.0".into(),
        command: "jankurai score diff".into(),
        base_report: base_path.display().to_string(),
        head_report: head_path.display().to_string(),
        base_score,
        head_score,
        score_delta: head_score - base_score,
        base_raw_score,
        head_raw_score,
        raw_score_delta: head_raw_score - base_raw_score,
        caps_added,
        caps_removed,
        new_findings,
        resolved_findings,
        carried_findings,
        new_high_or_critical,
        resolved_high_or_critical,
        decision: decision.into(),
    }
}

pub fn build_trend_report(history: &Path, window: usize) -> Result<ScoreTrendReport> {
    let report = score_history::build_trend_report(history, window)?;
    Ok(report)
}

fn render_diff_markdown(report: &ScoreDiffReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Score Diff");
    let _ = writeln!(out);
    let _ = writeln!(out, "- decision: `{}`", report.decision);
    let _ = writeln!(
        out,
        "- score: `{}` -> `{}` (`{:+}`)",
        report.base_score, report.head_score, report.score_delta
    );
    let _ = writeln!(out, "- caps added: `{}`", join_or_none(&report.caps_added));
    let _ = writeln!(
        out,
        "- caps removed: `{}`",
        join_or_none(&report.caps_removed)
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "## Findings");
    let _ = writeln!(out, "- new: `{}`", report.new_findings.len());
    let _ = writeln!(out, "- resolved: `{}`", report.resolved_findings.len());
    let _ = writeln!(out, "- carried: `{}`", report.carried_findings.len());
    for finding in report.new_findings.iter().take(10) {
        let _ = writeln!(
            out,
            "- new `{}` `{}` `{}`: {}",
            finding.severity,
            finding.rule_id.as_deref().unwrap_or("unruled"),
            finding.path,
            finding.problem
        );
    }
    out
}

fn render_trend_markdown(report: &ScoreTrendReport) -> String {
    score_history::render_history_trend_markdown(report)
}

fn load_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn int_field(value: &Value, key: &str) -> i32 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0) as i32
}

fn string_set(value: Option<&Value>) -> BTreeSet<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn diff_set(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

fn finding_map(report: &Value) -> BTreeMap<String, FindingSummary> {
    let mut out = BTreeMap::new();
    let Some(findings) = report.get("findings").and_then(Value::as_array) else {
        return out;
    };
    for finding in findings {
        let summary = finding_summary(finding);
        out.insert(summary.key.clone(), summary);
    }
    out
}

pub fn finding_summary(finding: &Value) -> FindingSummary {
    let fingerprint = string_opt(finding, "fingerprint");
    let rule_id = string_opt(finding, "rule_id");
    let severity = string_opt(finding, "severity").unwrap_or_else(|| "low".into());
    let path = string_opt(finding, "path").unwrap_or_default();
    let problem = string_opt(finding, "problem").unwrap_or_default();
    let key = fingerprint
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "{}:{}:{}",
                rule_id.clone().unwrap_or_else(|| "unruled".into()),
                path,
                problem
            )
        });
    FindingSummary {
        key,
        fingerprint,
        rule_id,
        severity,
        path,
        problem,
    }
}

fn string_opt(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn is_high_or_critical(severity: &str) -> bool {
    matches!(severity, "high" | "critical")
}

pub(crate) fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".into()
    } else {
        values.join(", ")
    }
}
