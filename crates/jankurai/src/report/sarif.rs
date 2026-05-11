use crate::model::{Finding, Report};
use serde_json::json;

/// Stable public URL for repo-relative doc paths (SARIF `helpUri` for Git viewers and CI).
const DOCS_URI_BASE: &str = "https://github.com/jeppsontaylor/jankurai/blob/main/";

fn sarif_help_uri(docs: &Option<String>) -> Option<String> {
    let s = docs.as_ref()?.trim();
    if s.is_empty() {
        return None;
    }
    if s.starts_with("http://") || s.starts_with("https://") {
        return Some(s.to_string());
    }
    let path = s.trim_start_matches('/');
    Some(format!("{DOCS_URI_BASE}{path}"))
}

fn physical_region(finding: &Finding) -> serde_json::Value {
    let line = finding.line.unwrap_or(1) as i64;
    let mut region = json!({
        "startLine": line,
        "endLine": line,
    });
    let snippet_text = finding
        .evidence
        .first()
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(2000).collect::<String>())
        .unwrap_or_else(|| finding.problem.chars().take(400).collect());
    if !snippet_text.is_empty() {
        region["snippet"] = json!({ "text": snippet_text });
    }
    region
}

pub fn render_sarif(report: &Report) -> String {
    // Build deduplicated rules[] array keyed by rule_id
    let mut rule_ids: Vec<String> = Vec::new();
    let mut rule_index_map: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for finding in &report.findings {
        let rid = finding
            .rule_id
            .as_deref()
            .unwrap_or("HLT-000-SCORE-DIMENSION")
            .to_string();
        rule_index_map.entry(rid).or_insert_with_key(|rid| {
            let idx = rule_ids.len();
            rule_ids.push(rid.clone());
            idx
        });
    }

    let rules: Vec<serde_json::Value> = rule_ids
        .iter()
        .map(|rid| {
            // Use the first finding with this rule_id for metadata
            let representative = report
                .findings
                .iter()
                .find(|f| f.rule_id.as_deref().unwrap_or("HLT-000-SCORE-DIMENSION") == rid)
                .unwrap();
            let mut desc = serde_json::Map::new();
            desc.insert("id".into(), json!(rid));
            desc.insert("name".into(), json!(&representative.check_id));
            desc.insert(
                "shortDescription".into(),
                json!({ "text": &representative.problem }),
            );
            desc.insert(
                "defaultConfiguration".into(),
                json!({ "level": sarif_level(&representative.severity) }),
            );
            if let Some(uri) = sarif_help_uri(&representative.docs_url) {
                desc.insert("helpUri".into(), json!(uri));
            }
            serde_json::Value::Object(desc)
        })
        .collect();

    let results = report
        .findings
        .iter()
        .map(|finding| {
            let rid = finding
                .rule_id
                .as_deref()
                .unwrap_or("HLT-000-SCORE-DIMENSION");
            let rule_index = rule_index_map.get(rid).copied().unwrap_or(0);
            json!({
                "ruleId": rid,
                "ruleIndex": rule_index,
                "level": sarif_level(&finding.severity),
                "message": { "text": finding.problem },
                "fingerprints": { "jankurai": finding.fingerprint },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": finding.path },
                        "region": physical_region(finding)
                    }
                }],
                "properties": {
                    "category": finding.category,
                    "hardness": finding.hardness,
                    "confidence": finding.confidence,
                    "evidenceKind": finding.evidence_kind,
                    "rerunCommand": finding.rerun_command,
                    "owner": finding.owner,
                    "lane": finding.lane,
                }
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "jankurai",
                    "version": report.auditor_version,
                    "informationUri": "https://github.com/jeppsontaylor/jankurai",
                    "rules": rules
                }
            },
            "results": results
        }]
    }))
    .unwrap_or_else(|_| "{}".into())
}

fn sarif_level(severity: &str) -> &'static str {
    match severity {
        "critical" | "high" => "error",
        "medium" => "warning",
        _ => "note",
    }
}
