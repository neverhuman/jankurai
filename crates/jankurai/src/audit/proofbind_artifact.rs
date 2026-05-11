use crate::validation::{self, ArtifactSchema};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ProofBindAuditSummary {
    pub path: String,
    pub mode: String,
    pub changed_surface_count: usize,
    pub missing: usize,
    pub high_or_critical_missing: usize,
    pub verdict: String,
    pub missing_obligations: Vec<ProofBindMissingObligation>,
}

#[derive(Debug, Clone)]
pub struct ProofBindMissingObligation {
    pub obligation_id: String,
    pub path: String,
    pub surface_type: String,
    pub severity: String,
    pub rule_ids: Vec<String>,
    pub repair_task: String,
}

#[derive(Debug, Deserialize, Default)]
struct ProofBindObligationsFile {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    obligations: Vec<ProofBindObligation>,
    #[serde(default)]
    summary: ProofBindSummary,
}

#[derive(Debug, Deserialize, Default)]
struct ProofBindSummary {
    #[serde(default)]
    missing: usize,
    #[serde(default)]
    high_or_critical_missing: usize,
    #[serde(default)]
    changed_surface_count: usize,
    #[serde(default)]
    verdict: String,
}

#[derive(Debug, Deserialize, Default)]
struct ProofBindObligation {
    #[serde(default)]
    obligation_id: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    surface_type: String,
    #[serde(default)]
    severity: String,
    #[serde(default)]
    rule_ids: Vec<String>,
    #[serde(default)]
    repair_task: String,
    #[serde(default)]
    satisfied: bool,
}

pub fn load_summary(root: &Path) -> Option<ProofBindAuditSummary> {
    let path = root.join("target/jankurai/proofbind/obligations.json");
    let text = fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    validation::validate_value(root, ArtifactSchema::ProofBindObligations, &value).ok()?;
    let parsed: ProofBindObligationsFile = serde_json::from_value(value).ok()?;
    let missing_obligations = parsed
        .obligations
        .into_iter()
        .filter(|obligation| !obligation.satisfied)
        .filter(|obligation| matches!(obligation.severity.as_str(), "high" | "critical"))
        .map(|obligation| ProofBindMissingObligation {
            obligation_id: obligation.obligation_id,
            path: obligation.path,
            surface_type: obligation.surface_type,
            severity: obligation.severity,
            rule_ids: obligation.rule_ids,
            repair_task: obligation.repair_task,
        })
        .collect();
    Some(ProofBindAuditSummary {
        path: "target/jankurai/proofbind/obligations.json".into(),
        mode: if parsed.mode.is_empty() {
            "advisory".into()
        } else {
            parsed.mode
        },
        changed_surface_count: parsed.summary.changed_surface_count,
        missing: parsed.summary.missing,
        high_or_critical_missing: parsed.summary.high_or_critical_missing,
        verdict: if parsed.summary.verdict.is_empty() {
            "review".into()
        } else {
            parsed.summary.verdict
        },
        missing_obligations,
    })
}
