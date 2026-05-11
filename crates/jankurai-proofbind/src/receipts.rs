use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::ChangedSurface;

#[derive(Debug, Clone)]
pub(crate) struct ReceiptEvidence {
    pub lane: String,
    pub exit_code: i64,
    pub path: String,
    pub changed_paths: Vec<String>,
    pub rules_covered: Vec<String>,
    pub full_repository_scope: bool,
    pub satisfied_obligations: Vec<String>,
    pub proofmark_results: BTreeMap<String, String>,
}

pub(crate) fn load_receipts(repo: &Path, path: Option<&Path>) -> Result<Vec<ReceiptEvidence>> {
    let Some(path) = path else {
        return Ok(vec![]);
    };
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.join(path)
    };
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut entries = Vec::new();
    if path.is_dir() {
        for entry in fs::read_dir(&path).with_context(|| format!("read {}", path.display()))? {
            let entry = entry?;
            if entry.path().extension().and_then(|ext| ext.to_str()) == Some("json") {
                entries.push(entry.path());
            }
        }
    } else {
        entries.push(path);
    }
    entries.sort();
    let mut receipts = Vec::new();
    for entry in entries {
        let text =
            fs::read_to_string(&entry).with_context(|| format!("read {}", entry.display()))?;
        let value: Value =
            serde_json::from_str(&text).with_context(|| format!("parse {}", entry.display()))?;
        receipts.push(receipt_from_value(repo, &entry, &value));
    }
    Ok(receipts)
}

pub(crate) fn receipt_satisfies(
    obligation_id: &str,
    surface: &ChangedSurface,
    receipt: &ReceiptEvidence,
) -> bool {
    if receipt.exit_code != 0 {
        return false;
    }
    if receipt
        .satisfied_obligations
        .iter()
        .any(|id| id == obligation_id)
        || receipt
            .proofmark_results
            .get(obligation_id)
            .is_some_and(|status| status == "pass")
    {
        return true;
    }
    if receipt.lane == "proofmark-rust" {
        return false;
    }
    let lane_matches = surface
        .required_lanes
        .iter()
        .any(|lane| lane == &receipt.lane);
    if !lane_matches {
        return false;
    }
    let path_matches = (receipt.changed_paths.is_empty() && receipt.full_repository_scope)
        || receipt
            .changed_paths
            .iter()
            .any(|path| path == &surface.path || surface.path.starts_with(&format!("{path}/")));
    if !path_matches {
        return false;
    }
    if surface.required_rules.is_empty() {
        return true;
    }
    !receipt.rules_covered.is_empty()
        && surface
            .required_rules
            .iter()
            .any(|rule| receipt.rules_covered.iter().any(|covered| covered == rule))
}

fn receipt_from_value(repo: &Path, entry: &Path, value: &Value) -> ReceiptEvidence {
    let lane = if let Some(lane) = value.get("lane").and_then(Value::as_str) {
        lane.to_string()
    } else {
        "unknown".into()
    };
    let exit_code = value.get("exit_code").and_then(Value::as_i64).unwrap_or(1);
    let changed_paths = if let Some(items) = value.get("changed_paths").and_then(Value::as_array) {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut rules_covered = Vec::new();
    if let Some(items) = value.get("rules_covered").and_then(Value::as_array) {
        for item in items {
            if let Some(rule) = item.as_str() {
                rules_covered.push(rule.to_string());
            } else if let Some(rule) = item.get("rule_id").and_then(Value::as_str) {
                let status = item
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("covered");
                if !matches!(status, "covered" | "pass" | "satisfied") {
                    continue;
                }
                rules_covered.push(rule.to_string());
            }
        }
    }
    let null_value = Value::Null;
    let proofmark =
        if let Some(proofmark) = value.get("extensions").and_then(|v| v.get("proofmark")) {
            proofmark
        } else {
            &null_value
        };
    let satisfied_obligations = if let Some(items) = proofmark
        .get("satisfied_obligations")
        .and_then(Value::as_array)
    {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut proofmark_results = BTreeMap::new();
    if let Some(items) = proofmark
        .get("obligation_results")
        .and_then(Value::as_array)
    {
        for item in items {
            let Some(id) = item.get("obligation_id").and_then(Value::as_str) else {
                continue;
            };
            let status = item
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            proofmark_results.insert(id.to_string(), status.to_string());
        }
    }
    let full_repository_scope = value
        .get("extensions")
        .and_then(|extensions| extensions.get("full_repository_scope"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || value
            .get("extensions")
            .and_then(|extensions| extensions.get("scope"))
            .and_then(Value::as_str)
            .is_some_and(|scope| scope == "full-repository" || scope == "full_repository");
    ReceiptEvidence {
        lane,
        exit_code,
        path: display_rel(repo, entry),
        changed_paths,
        rules_covered,
        full_repository_scope,
        satisfied_obligations,
        proofmark_results,
    }
}

fn display_rel(repo: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(repo) {
        rel.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string_lossy().replace('\\', "/")
    }
}
