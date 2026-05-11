//! Load and summarize `target/jankurai/ux-qa.json` for audit reports.

use crate::model::UxQaReportArtifactSummary;
use crate::validation::{self, ArtifactSchema};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const UX_QA_REPORT_REL: &str = "target/jankurai/ux-qa.json";

pub fn load_report_summary(root: &Path) -> Option<UxQaReportArtifactSummary> {
    let path = root.join(UX_QA_REPORT_REL);
    if !path.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(&path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    validation::validate_value(root, ArtifactSchema::UxQaReport, &value).ok()?;
    summarize(&value)
}

/// Validate and summarize already-parsed JSON (for tests).
pub fn summarize_validated_report(root: &Path, value: &Value) -> Option<UxQaReportArtifactSummary> {
    validation::validate_value(root, ArtifactSchema::UxQaReport, value).ok()?;
    summarize(value)
}

fn summarize(value: &Value) -> Option<UxQaReportArtifactSummary> {
    let reports = value.get("reports")?.as_array()?;
    let report_count = reports.len();
    let mut total_violations = 0usize;
    let mut summary_errors = 0u64;
    let mut summary_warnings = 0u64;
    let mut worst_rank = 0u8;
    let mut worst_decision = "pass".to_string();
    let mut reports_missing_required_states = 0usize;
    let mut missing_state_names = BTreeSet::new();
    let mut artifact_counts_by_kind = BTreeMap::new();
    let mut reports_missing_required_artifacts = 0usize;
    let mut missing_artifact_kinds = BTreeSet::new();
    let mut reports_missing_required_accessibility_artifact = 0usize;
    let mut accessibility_violation_total = 0u64;
    let mut accessibility_incomplete_total = 0u64;
    let mut accessibility_pass_total = 0u64;
    let mut artifact_fingerprint_count = 0usize;
    let mut visual_baseline_missing = 0usize;
    let mut visual_baseline_changed = 0usize;
    let mut visual_baseline_review = 0usize;
    let mut visual_baseline_block = 0usize;

    for report in reports {
        if let Some(arr) = report.get("violations").and_then(Value::as_array) {
            total_violations += arr.len();
        }
        if let Some(arr) = report.get("artifacts").and_then(Value::as_array) {
            for artifact in arr {
                if artifact
                    .get("sha256")
                    .and_then(Value::as_str)
                    .map(|sha| !sha.trim().is_empty())
                    .unwrap_or(false)
                {
                    artifact_fingerprint_count += 1;
                }
            }
            for kind in arr
                .iter()
                .filter_map(|artifact| artifact.get("kind").and_then(Value::as_str))
            {
                *artifact_counts_by_kind.entry(kind.to_string()).or_insert(0) += 1;
            }
        }
        if let Some(summary) = report.get("summary") {
            summary_errors += summary.get("errors").and_then(Value::as_u64).unwrap_or(0);
            summary_warnings += summary.get("warnings").and_then(Value::as_u64).unwrap_or(0);
        }
        let missing_states = string_array_at(report, &["stateCoverage", "missing"]);
        if !missing_states.is_empty() {
            reports_missing_required_states += 1;
            missing_state_names.extend(missing_states);
        }
        let missing_artifacts = string_array_at(report, &["artifactCoverage", "missing"]);
        if !missing_artifacts.is_empty() {
            reports_missing_required_artifacts += 1;
            if missing_artifacts.iter().any(|kind| kind == "accessibility") {
                reports_missing_required_accessibility_artifact += 1;
            }
            missing_artifact_kinds.extend(missing_artifacts);
        }
        if let Some(accessibility) = report.get("accessibility") {
            accessibility_violation_total += accessibility
                .get("violations")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            accessibility_incomplete_total += accessibility
                .get("incomplete")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            accessibility_pass_total += accessibility
                .get("passes")
                .and_then(Value::as_u64)
                .unwrap_or(0);
        }
        if let Some(visual_baseline) = report.get("visualBaseline") {
            match visual_baseline
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("")
            {
                "not-configured" | "missing-baseline" => visual_baseline_missing += 1,
                "changed" => visual_baseline_changed += 1,
                _ => {}
            }
            match visual_baseline
                .get("decision")
                .and_then(Value::as_str)
                .unwrap_or("")
            {
                "review" => visual_baseline_review += 1,
                "block" => visual_baseline_block += 1,
                _ => {}
            }
        }
        let d = report
            .get("decision")
            .and_then(Value::as_str)
            .unwrap_or("pass");
        let rank = decision_rank(d);
        if rank > worst_rank {
            worst_rank = rank;
            worst_decision = d.to_string();
        }
    }

    Some(UxQaReportArtifactSummary {
        path: UX_QA_REPORT_REL.into(),
        report_count,
        worst_decision,
        total_violations,
        summary_errors,
        summary_warnings,
        reports_missing_required_states,
        missing_state_names: missing_state_names.into_iter().collect(),
        artifact_counts_by_kind,
        reports_missing_required_artifacts,
        missing_artifact_kinds: missing_artifact_kinds.into_iter().collect(),
        reports_missing_required_accessibility_artifact,
        accessibility_violation_total,
        accessibility_incomplete_total,
        accessibility_pass_total,
        artifact_fingerprint_count,
        visual_baseline_missing,
        visual_baseline_changed,
        visual_baseline_review,
        visual_baseline_block,
    })
}

fn string_array_at(value: &Value, path: &[&str]) -> Vec<String> {
    let mut current = value;
    for key in path {
        current = match current.get(key) {
            Some(next) => next,
            None => return Vec::new(),
        };
    }
    current
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn decision_rank(decision: &str) -> u8 {
    match decision {
        "block" => 4,
        "review" => 3,
        "warn" => 2,
        "pass" => 1,
        _ => 1,
    }
}
