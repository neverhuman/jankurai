use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::coverage::CoverageData;
use crate::shared::{path_symbol, resolve_repo_path};
use crate::{
    ChangedUnit, CoverageSummary, MutationSummary, ObligationResult, ProofBindObligation,
    ProofMarkMode, ProofMarkSummary,
};

#[derive(Debug, Clone, Deserialize, Default)]
struct ProofBindObligations {
    #[serde(default)]
    obligations: Vec<ProofBindObligation>,
}

pub(crate) fn obligation_result(
    obligation: &ProofBindObligation,
    units: &[ChangedUnit],
    mutation: &MutationSummary,
    negative_proofs: &BTreeSet<String>,
) -> ObligationResult {
    let matching_units = units
        .iter()
        .filter(|unit| unit.path == obligation.path)
        .collect::<Vec<_>>();
    let coverage_pass = !matching_units.is_empty()
        && matching_units
            .iter()
            .all(|unit| unit.coverage_status == "pass");
    let coverage_status = if matching_units.is_empty() {
        "unavailable"
    } else if coverage_pass {
        "pass"
    } else {
        "review"
    };
    let boundary_sensitive = obligation.rule_ids.iter().any(|rule| {
        matches!(
            rule.as_str(),
            "HLT-021-DESTRUCTIVE-MIGRATION"
                | "HLT-022-AUTHZ-ISOLATION-GAP"
                | "HLT-023-INPUT-BOUNDARY-GAP"
                | "HLT-024-AGENT-TOOL-SUPPLY-GAP"
        )
    }) || obligation
        .required_receipt_kinds
        .iter()
        .any(|kind| kind == "negative-behavior-proof");
    let mutation_status = mutation.status.clone();
    let negative_proof_status = if !boundary_sensitive {
        "not_required".to_string()
    } else if negative_proofs.contains(&obligation.obligation_id.to_ascii_lowercase())
        || obligation
            .rule_ids
            .iter()
            .any(|rule| negative_proofs.contains(&rule.to_ascii_lowercase()))
        || negative_proofs.contains(&obligation.path.to_ascii_lowercase())
    {
        "present".to_string()
    } else {
        "missing".to_string()
    };
    let mutation_ok =
        mutation.status == "pass" || (!boundary_sensitive && mutation.status == "unavailable");
    let negative_ok = negative_proof_status != "missing";
    let status = if coverage_pass && mutation_ok && negative_ok {
        "pass"
    } else {
        "review"
    };
    let mut evidence = Vec::new();
    if coverage_pass {
        evidence.push("all changed Rust lines are covered by supplied coverage evidence".into());
    }
    if mutation.status == "pass" {
        evidence.push("mutation evidence has no survived in-diff mutants".into());
    }
    if negative_proof_status == "present" {
        evidence.push("negative proof marker matched obligation, rule, or path".into());
    }
    let mut residual_risk = Vec::new();
    if coverage_status != "pass" {
        residual_risk.push("changed-line coverage is missing or incomplete".into());
    }
    if mutation.status == "unavailable" {
        residual_risk.push("focused mutation evidence was not supplied".into());
    } else if mutation.status != "pass" {
        residual_risk.push("mutation evidence reports survived or timed-out mutants".into());
    }
    if negative_proof_status == "missing" {
        residual_risk.push("boundary-sensitive change lacks negative behavior proof".into());
    }
    ObligationResult {
        obligation_id: obligation.obligation_id.clone(),
        path: obligation.path.clone(),
        rule_ids: obligation.rule_ids.clone(),
        required_lanes: obligation.required_lanes.clone(),
        status: status.into(),
        coverage_status: coverage_status.into(),
        mutation_status,
        negative_proof_status,
        evidence,
        residual_risk,
    }
}

pub(crate) fn proofmark_summary(
    units: &[ChangedUnit],
    results: &[ObligationResult],
    mode: ProofMarkMode,
) -> ProofMarkSummary {
    let satisfied = results
        .iter()
        .filter(|result| result.status == "pass")
        .count();
    let review = results.len().saturating_sub(satisfied);
    let verdict = if review == 0 {
        "pass"
    } else if mode == ProofMarkMode::Required {
        "block"
    } else {
        "review"
    };
    ProofMarkSummary {
        total_obligations: results.len(),
        satisfied_obligations: satisfied,
        review_obligations: review,
        changed_units: units.len(),
        verdict: verdict.into(),
    }
}

pub(crate) fn changed_unit(
    path: &str,
    changed_lines: Option<&BTreeSet<u32>>,
    coverage: &CoverageData,
) -> ChangedUnit {
    let changed_lines = match changed_lines {
        Some(lines) => lines.clone(),
        None => BTreeSet::from([1]),
    }
    .into_iter()
    .collect::<Vec<_>>();
    let covered = coverage.covered_lines(path);
    let mut covered_changed_lines = Vec::new();
    let mut uncovered_changed_lines = Vec::new();
    for line in &changed_lines {
        if covered.contains(line) {
            covered_changed_lines.push(*line);
        } else {
            uncovered_changed_lines.push(*line);
        }
    }
    let coverage_status = if !coverage.loaded {
        "unavailable"
    } else if uncovered_changed_lines.is_empty() {
        "pass"
    } else {
        "review"
    };
    ChangedUnit {
        path: path.into(),
        unit: path_symbol(path),
        changed_lines,
        covered_changed_lines,
        uncovered_changed_lines,
        coverage_status: coverage_status.into(),
    }
}

pub(crate) fn coverage_summary(
    source: Option<&Path>,
    units: &[ChangedUnit],
    loaded: bool,
) -> CoverageSummary {
    let changed_line_count = units.iter().map(|unit| unit.changed_lines.len()).sum();
    let covered_changed_line_count = units
        .iter()
        .map(|unit| unit.covered_changed_lines.len())
        .sum();
    let uncovered_changed_line_count = units
        .iter()
        .map(|unit| unit.uncovered_changed_lines.len())
        .sum();
    let status = if !loaded {
        "unavailable"
    } else if uncovered_changed_line_count == 0 {
        "pass"
    } else {
        "review"
    };
    CoverageSummary {
        source: match source {
            Some(path) => path.display().to_string(),
            None => "unavailable".into(),
        },
        changed_line_count,
        covered_changed_line_count,
        uncovered_changed_line_count,
        status: status.into(),
    }
}

pub(crate) fn load_obligations(
    repo: &Path,
    path: Option<&Path>,
) -> Result<Vec<ProofBindObligation>> {
    let Some(path) = path else {
        return Ok(vec![]);
    };
    let path = resolve_repo_path(repo, path);
    if !path.exists() {
        return Ok(vec![]);
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read obligations {}", path.display()))?;
    let parsed: ProofBindObligations = serde_json::from_str(&text)
        .with_context(|| format!("parse obligations {}", path.display()))?;
    Ok(parsed.obligations)
}

pub(crate) fn load_mutation(repo: &Path, path: Option<&Path>) -> Result<MutationSummary> {
    let Some(path) = path else {
        return Ok(MutationSummary {
            source: "unavailable".into(),
            status: "unavailable".into(),
            killed: 0,
            survived: 0,
            timeout: 0,
        });
    };
    let path = resolve_repo_path(repo, path);
    let text =
        fs::read_to_string(&path).with_context(|| format!("read mutation {}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .with_context(|| format!("parse mutation {}", path.display()))?;
    let killed = number_at(&value, &["killed", "caught", "success"]);
    let survived = number_at(&value, &["survived", "missed", "unmutated"]);
    let timeout = number_at(&value, &["timeout", "timed_out"]);
    Ok(MutationSummary {
        source: path.display().to_string(),
        status: if survived == 0 && timeout == 0 {
            "pass"
        } else {
            "review"
        }
        .into(),
        killed,
        survived,
        timeout,
    })
}

fn number_at(value: &Value, keys: &[&str]) -> usize {
    for key in keys {
        if let Some(number) = value.get(*key).and_then(Value::as_u64) {
            return number as usize;
        }
        if let Some(number) = value
            .get("summary")
            .and_then(|summary| summary.get(*key))
            .and_then(Value::as_u64)
        {
            return number as usize;
        }
    }
    0
}
