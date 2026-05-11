use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[derive(Debug, Deserialize)]
struct Source {
    issues: Vec<Issue>,
}

#[derive(Debug, Deserialize)]
struct Issue {
    id: String,
    source_refs: Vec<String>,
    coverage: String,
    canonical_group: String,
    rule_ids: Vec<String>,
    reviewed: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedFixture {
    expect: Vec<Expected>,
}

#[derive(Debug, Deserialize)]
struct Expected {
    source_ref: String,
    canonical_group: String,
    rule_id: String,
    coverage: Option<String>,
}

#[test]
fn reviewed_mapping_has_expected_counts_and_no_gaps() {
    let repo = repo_root();
    let source: Source =
        toml::from_str(&fs::read_to_string(repo.join("agent/vibe-coverage.toml")).unwrap())
            .unwrap();

    let mut seen = BTreeMap::<String, usize>::new();
    for issue in &source.issues {
        assert!(issue.reviewed, "{} must be reviewed", issue.id);
        assert_ne!(
            issue.coverage, "none",
            "{} still has none coverage",
            issue.id
        );
        for source_ref in &issue.source_refs {
            *seen.entry(source_ref.clone()).or_insert(0) += 1;
        }
    }

    let duplicate_refs = seen
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(source_ref, _)| source_ref.clone())
        .collect::<Vec<_>>();
    assert_eq!(seen.len(), 260);
    assert!(duplicate_refs.is_empty(), "duplicates: {duplicate_refs:?}");

    let expected_refs = (1..=5)
        .flat_map(|tip| {
            let text =
                fs::read_to_string(repo.join(format!("tips/vibe_coding/tip{tip}.txt"))).unwrap();
            text.lines()
                .filter_map(move |line| {
                    line.strip_prefix('|')
                        .and_then(|rest| rest.split('|').next())
                        .and_then(|cell| cell.trim().parse::<usize>().ok())
                        .map(move |row| format!("tip{tip}:{row}"))
                })
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_refs.len(), 260);
    assert_eq!(seen.keys().cloned().collect::<BTreeSet<_>>(), expected_refs);
}

#[test]
fn high_risk_rows_keep_reviewed_canonical_groups() {
    let repo = repo_root();
    let source: Source =
        toml::from_str(&fs::read_to_string(repo.join("agent/vibe-coverage.toml")).unwrap())
            .unwrap();
    let fixture: ExpectedFixture = toml::from_str(
        &fs::read_to_string(
            repo.join("crates/jankurai/tests/fixtures/vibe_coverage/expected-mapping.toml"),
        )
        .unwrap(),
    )
    .unwrap();

    for expected in fixture.expect {
        let issue = source
            .issues
            .iter()
            .find(|issue| {
                issue
                    .source_refs
                    .iter()
                    .any(|source_ref| source_ref == &expected.source_ref)
            })
            .unwrap_or_else(|| panic!("missing {}", expected.source_ref));
        assert_eq!(
            issue.canonical_group, expected.canonical_group,
            "{} group drifted",
            expected.source_ref
        );
        assert!(
            issue
                .rule_ids
                .iter()
                .any(|rule_id| rule_id == &expected.rule_id),
            "{} missing rule {}",
            expected.source_ref,
            expected.rule_id
        );
        if let Some(coverage) = expected.coverage {
            assert_eq!(
                issue.coverage, coverage,
                "{} coverage drifted",
                expected.source_ref
            );
        }
    }
}
