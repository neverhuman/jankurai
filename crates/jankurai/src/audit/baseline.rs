use crate::model::{Report, ReportRatchet};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub fn compare_report_to_baseline(report: &Report, baseline_path: &Path) -> Result<ReportRatchet> {
    let text = std::fs::read_to_string(baseline_path)
        .with_context(|| format!("read ratchet baseline {}", baseline_path.display()))?;
    let baseline: Value = serde_json::from_str(&text)
        .with_context(|| format!("parse ratchet baseline {}", baseline_path.display()))?;

    let baseline_score = required_i32(&baseline, "score")?;
    let baseline_report_fingerprint = required_string(&baseline, "report_fingerprint")?;
    let baseline_input_fingerprint = required_string(&baseline, "input_fingerprint")?;
    let baseline_policy_fingerprint = required_string(&baseline, "policy_fingerprint")?;
    let baseline_schema_version = required_string(&baseline, "schema_version")?;
    let baseline_standard_version = required_string(&baseline, "standard_version")?;
    let baseline_caps = required_string_set(&baseline, "caps_applied")?;
    let baseline_findings = required_hard_finding_fingerprints(&baseline)?;

    if baseline.get("findings").and_then(Value::as_array).is_none() {
        bail!("ratchet baseline missing required array `findings`");
    }

    let current_caps = report.caps_applied.iter().cloned().collect::<BTreeSet<_>>();
    let current_findings = report
        .findings
        .iter()
        .filter(|finding| matches!(finding.severity.as_str(), "critical" | "high"))
        .map(|finding| finding.fingerprint.clone())
        .collect::<BTreeSet<_>>();

    let new_caps = current_caps
        .difference(&baseline_caps)
        .cloned()
        .collect::<Vec<_>>();
    let new_hard_findings = current_findings
        .difference(&baseline_findings)
        .cloned()
        .collect::<Vec<_>>();
    let policy_changed = baseline_policy_fingerprint != report.policy_fingerprint;
    let version_compatible = baseline_schema_version == report.schema_version
        && baseline_standard_version == report.standard_version;
    let score_delta = report.score - baseline_score;
    let passed = score_delta >= 0
        && new_caps.is_empty()
        && new_hard_findings.is_empty()
        && !policy_changed
        && version_compatible;

    Ok(ReportRatchet {
        baseline_score,
        allowed_drop: 0,
        passed,
        score_delta,
        baseline_report_fingerprint,
        baseline_input_fingerprint,
        baseline_policy_fingerprint,
        new_caps,
        new_hard_findings,
        policy_changed,
    })
}

fn required_i32(value: &Value, field: &str) -> Result<i32> {
    let Some(raw) = value.get(field).and_then(Value::as_i64) else {
        bail!("ratchet baseline missing required integer `{field}`");
    };
    i32::try_from(raw).with_context(|| format!("ratchet baseline `{field}` is out of range"))
}

fn required_string(value: &Value, field: &str) -> Result<String> {
    let Some(raw) = value.get(field).and_then(Value::as_str) else {
        bail!("ratchet baseline missing required string `{field}`");
    };
    if raw.trim().is_empty() {
        bail!("ratchet baseline required string `{field}` is empty");
    }
    Ok(raw.to_string())
}

fn required_string_set(value: &Value, field: &str) -> Result<BTreeSet<String>> {
    let Some(items) = value.get(field).and_then(Value::as_array) else {
        bail!("ratchet baseline missing required array `{field}`");
    };
    let mut out = BTreeSet::new();
    for item in items {
        let Some(text) = item.as_str() else {
            bail!("ratchet baseline `{field}` must contain only strings");
        };
        out.insert(text.to_string());
    }
    Ok(out)
}

fn required_hard_finding_fingerprints(value: &Value) -> Result<BTreeSet<String>> {
    let Some(items) = value.get("findings").and_then(Value::as_array) else {
        bail!("ratchet baseline missing required array `findings`");
    };
    let mut out = BTreeSet::new();
    for item in items {
        let severity = item.get("severity").and_then(Value::as_str).unwrap_or("");
        if !matches!(severity, "critical" | "high") {
            continue;
        }
        let Some(fingerprint) = item.get("fingerprint").and_then(Value::as_str) else {
            bail!("ratchet baseline hard finding missing required string `fingerprint`");
        };
        out.insert(fingerprint.to_string());
    }
    Ok(out)
}
