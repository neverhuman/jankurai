use jankurai::audit::run_audit;
use jankurai::report::sarif::render_sarif;
use std::fs;
use tempfile::tempdir;

#[test]
fn audit_detects_orphaned_contract_source() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# thin\n").unwrap();
    fs::create_dir_all(dir.path().join("contracts")).unwrap();
    fs::write(
        dir.path().join("contracts/openapi.yaml"),
        "openapi: 3.0.0\ninfo:\n  title: Example\n  version: 1.0.0\n",
    )
    .unwrap();
    // No agent/generated-zones.toml → orphaned contract

    let report = run_audit(dir.path(), &[]).unwrap();
    let contract_findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| {
            f.path == "contracts/openapi.yaml"
                && f.rule_id.as_deref() == Some("HLT-007-HANDWRITTEN-CONTRACT")
        })
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "expected HLT-007 finding for orphaned contract source; findings: {:?}",
        report
            .findings
            .iter()
            .map(|f| (&f.path, &f.rule_id))
            .collect::<Vec<_>>()
    );
}

#[test]
fn audit_allows_contract_source_with_generated_zone() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# thin\n").unwrap();
    fs::create_dir_all(dir.path().join("contracts")).unwrap();
    fs::write(
        dir.path().join("contracts/openapi.yaml"),
        "openapi: 3.0.0\ninfo:\n  title: Example\n  version: 1.0.0\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("agent/generated-zones.toml"),
        r#"[[zone]]
path = "out/client.ts"
source = "contracts/openapi.yaml"
command = "npx openapi-generator generate -i contracts/openapi.yaml -g typescript-fetch -o out/"
"#,
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let orphaned_findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| {
            f.path == "contracts/openapi.yaml"
                && f.rule_id.as_deref() == Some("HLT-007-HANDWRITTEN-CONTRACT")
                && f.evidence
                    .iter()
                    .any(|e| e.contains("no generated zone entry"))
        })
        .collect();
    assert!(
        orphaned_findings.is_empty(),
        "should not flag contract source that has a matching generated zone"
    );
}

#[test]
fn audit_detects_missing_generated_zone_file() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# thin\n").unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("agent/generated-zones.toml"),
        r#"[[zone]]
path = "out/nonexistent.ts"
source = "contracts/openapi.yaml"
command = "npx openapi-generator"
"#,
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let missing: Vec<_> = report
        .findings
        .iter()
        .filter(|f| {
            f.rule_id.as_deref() == Some("HLT-002-GENERATED-MUTATION")
                && f.evidence.iter().any(|e| {
                    e.contains("does not exist on disk")
                        || e.contains("generated zone integrity violation")
                })
        })
        .collect();
    assert!(
        !missing.is_empty(),
        "expected HLT-002 finding for missing generated zone file; findings: {:?}",
        report
            .findings
            .iter()
            .map(|f| (&f.path, &f.rule_id, &f.evidence))
            .collect::<Vec<_>>()
    );
}

#[test]
fn audit_detects_generated_zone_missing_header() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# thin\n").unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::create_dir_all(dir.path().join("out")).unwrap();
    fs::write(
        dir.path().join("out/client.ts"),
        "// Client module\nexport const foo = 42;\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("agent/generated-zones.toml"),
        r#"[[zone]]
path = "out/client.ts"
source = "contracts/openapi.yaml"
command = "npx openapi-generator"
"#,
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let header_missing: Vec<_> = report
        .findings
        .iter()
        .filter(|f| {
            f.rule_id.as_deref() == Some("HLT-002-GENERATED-MUTATION")
                && f.evidence.iter().any(|e| {
                    e.contains("lacks a")
                        || e.contains("missing generated header")
                        || e.contains("generated zone integrity violation")
                })
        })
        .collect();
    assert!(
        !header_missing.is_empty(),
        "expected HLT-002 finding for missing generated header; findings: {:?}",
        report
            .findings
            .iter()
            .map(|f| (&f.path, &f.rule_id, &f.evidence))
            .collect::<Vec<_>>()
    );
}

#[test]
fn audit_detects_missing_event_contract_path() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# thin\n").unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        r#"[queues]
adapter_paths = ["crates/adapters/queues"]
event_contract_paths = ["contracts/events"]

[[streaming_exception]]
runtime = "kafka"
owner = "team"
reason = "brownfield"
classification = "brownfield"
migration_path = "migrate to tansu"
expires = "2027-12-31"
"#,
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let missing_event: Vec<_> = report
        .findings
        .iter()
        .filter(|f| {
            f.path == "agent/boundaries.toml"
                && f.rule_id.as_deref() == Some("HLT-007-HANDWRITTEN-CONTRACT")
                && f.evidence.iter().any(|e| e.contains("event contract path"))
        })
        .collect();
    assert!(
        !missing_event.is_empty(),
        "expected HLT-007 finding for missing event contract path; findings: {:?}",
        report
            .findings
            .iter()
            .map(|f| (&f.path, &f.rule_id, &f.evidence))
            .collect::<Vec<_>>()
    );
}

#[test]
fn audit_allows_existing_event_contract_path() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# thin\n").unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::create_dir_all(dir.path().join("contracts/events")).unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        r#"[queues]
adapter_paths = ["crates/adapters/queues"]
event_contract_paths = ["contracts/events"]

[[streaming_exception]]
runtime = "kafka"
owner = "team"
reason = "brownfield"
classification = "brownfield"
migration_path = "migrate to tansu"
expires = "2027-12-31"
"#,
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(
        !report.findings.iter().any(|f| {
            f.path == "agent/boundaries.toml"
                && f.rule_id.as_deref() == Some("HLT-007-HANDWRITTEN-CONTRACT")
                && f.evidence.iter().any(|e| e.contains("event contract path"))
        }),
        "{:?}",
        report
            .findings
            .iter()
            .map(|f| (&f.path, &f.rule_id, &f.evidence))
            .collect::<Vec<_>>()
    );
}

#[test]
fn sarif_rules_array_deduplicates_rule_metadata() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# thin\n").unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    let sarif_text = render_sarif(&report);
    let sarif: serde_json::Value = serde_json::from_str(&sarif_text).unwrap();

    let runs = sarif["runs"].as_array().unwrap();
    assert!(!runs.is_empty());
    let driver = &runs[0]["tool"]["driver"];
    let rules = driver["rules"].as_array().unwrap();

    // Rules should be unique by id
    let mut seen_ids = std::collections::HashSet::new();
    for rule in rules {
        let id = rule["id"].as_str().unwrap();
        assert!(
            seen_ids.insert(id.to_string()),
            "duplicate rule id `{id}` in SARIF rules[] array"
        );
        // Each rule should have defaultConfiguration
        assert!(
            rule.get("defaultConfiguration").is_some(),
            "rule `{id}` missing defaultConfiguration"
        );
    }

    // Results should reference ruleIndex
    let results = runs[0]["results"].as_array().unwrap();
    for result in results {
        assert!(
            result.get("ruleIndex").is_some(),
            "SARIF result missing ruleIndex: {:?}",
            result
        );
        let idx = result["ruleIndex"].as_u64().unwrap() as usize;
        assert!(
            idx < rules.len(),
            "ruleIndex {} out of bounds for rules[] length {}",
            idx,
            rules.len()
        );
    }
}
