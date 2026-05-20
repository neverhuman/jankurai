use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::local_state;

#[derive(Debug, Clone, Deserialize)]
pub struct StandardVersionManifest {
    pub standard: String,
    pub standard_version: String,
    pub paper_edition: String,
    pub auditor_version: String,
    pub schema_version: String,
    pub target_stack: String,
    #[serde(default)]
    pub published: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReleaseData {
    pub standard_version: String,
    pub auditor_version: String,
    pub paper_edition: String,
    pub schema_version: String,
    pub target_stack_id: String,
    pub target_stack: String,
    pub published: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct FindingsSummary {
    pub critical: i32,
    pub high: i32,
    pub medium: i32,
    pub low: i32,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RepoScoreSummary {
    pub score: i32,
    pub caps: Vec<String>,
    pub findings: FindingsSummary,
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

pub fn load_release_data(repo: &Path) -> Result<ReleaseData> {
    let manifest_path = release_manifest_path(repo);
    let text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: StandardVersionManifest =
        toml::from_str(&text).with_context(|| format!("parse {}", manifest_path.display()))?;
    let standard_version = if repo.join("agent/standard-version.toml").exists() {
        manifest.standard_version
    } else {
        standard_doc_version(repo).unwrap_or(manifest.standard_version)
    };
    Ok(ReleaseData {
        standard_version,
        auditor_version: manifest.auditor_version,
        paper_edition: manifest.paper_edition,
        schema_version: manifest.schema_version,
        target_stack_id: manifest.target_stack.clone(),
        target_stack: manifest.target_stack,
        published: manifest.published,
    })
}

pub fn read_repo_score(repo: &Path) -> Result<Option<RepoScoreSummary>> {
    let score_path = local_state::preferred_repo_path(
        repo,
        local_state::SCORE_JSON,
        Some(local_state::LEGACY_SCORE_JSON),
    );
    if !score_path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&score_path)
        .with_context(|| format!("read {}", score_path.display()))?;
    let value: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", score_path.display()))?;
    Ok(Some(RepoScoreSummary {
        score: read_score(&value),
        caps: read_string_array(&value, &["hard_caps", "caps", "caps_applied"]),
        findings: read_findings_summary(&value),
    }))
}

fn release_manifest_path(repo: &Path) -> PathBuf {
    let candidate = repo.join("agent/standard-version.toml");
    if candidate.exists() {
        candidate
    } else {
        workspace_root().join("agent/standard-version.toml")
    }
}

fn standard_doc_version(root: &Path) -> Option<String> {
    for path in [
        root.join("agent/JANKURAI_STANDARD.md"),
        root.join("docs/agent-native-standard.md"),
    ] {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("Standard version: `") else {
                continue;
            };
            let Some((version, _)) = rest.split_once('`') else {
                continue;
            };
            if !version.trim().is_empty() {
                return Some(version.trim().to_string());
            }
        }
    }
    None
}

fn read_score(value: &Value) -> i32 {
    value
        .get("score")
        .and_then(Value::as_i64)
        .or_else(|| value.get("raw_score").and_then(Value::as_i64))
        .unwrap_or(0) as i32
}

fn read_string_array(value: &Value, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(items) = value.get(*key).and_then(Value::as_array) {
            return items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.to_string())
                .collect();
        }
    }
    Vec::new()
}

fn read_findings_summary(value: &Value) -> FindingsSummary {
    let mut summary = FindingsSummary::default();
    let Some(findings) = value.get("findings").and_then(Value::as_array) else {
        return summary;
    };
    for finding in findings {
        let severity = finding
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("medium")
            .to_ascii_lowercase();
        match severity.as_str() {
            "critical" => summary.critical += 1,
            "high" => summary.high += 1,
            "medium" => summary.medium += 1,
            "low" => summary.low += 1,
            _ => summary.medium += 1,
        }
    }
    if summary.critical == 0 && summary.high == 0 {
        summary
            .notes
            .push("no high or critical findings reported".to_string());
    }
    summary
}
