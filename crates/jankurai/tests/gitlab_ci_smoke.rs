use serde_yaml::Value as YamlValue;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).expect(path)
}

fn top_level<'a>(yaml: &'a YamlValue, key: &str) -> &'a YamlValue {
    yaml.as_mapping()
        .and_then(|mapping| {
            mapping.iter().find_map(|(entry_key, entry_value)| {
                entry_key
                    .as_str()
                    .filter(|value| *value == key)
                    .map(|_| entry_value)
            })
        })
        .unwrap_or_else(|| panic!("missing top-level key `{key}`"))
}

fn has_top_level_key(value: &YamlValue, key: &str) -> bool {
    value.as_mapping().is_some_and(|mapping| {
        mapping
            .iter()
            .any(|(entry_key, _)| entry_key.as_str() == Some(key))
    })
}

fn sequence_strings(value: &YamlValue) -> Vec<String> {
    value
        .as_sequence()
        .unwrap_or_else(|| panic!("expected YAML sequence: {value:?}"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("expected string item: {item:?}"))
                .to_string()
        })
        .collect()
}

fn sequence_texts(value: &YamlValue) -> Vec<String> {
    value
        .as_sequence()
        .unwrap_or_else(|| panic!("expected YAML sequence: {value:?}"))
        .iter()
        .map(|item| serde_yaml::to_string(item).unwrap_or_default())
        .collect()
}

fn need_job_names(value: &YamlValue) -> Vec<String> {
    value
        .as_sequence()
        .unwrap_or_else(|| panic!("expected YAML sequence: {value:?}"))
        .iter()
        .map(|item| {
            if let Some(name) = item.as_str() {
                return name.to_string();
            }
            let mapping = item
                .as_mapping()
                .unwrap_or_else(|| panic!("expected need mapping or string: {item:?}"));
            mapping
                .iter()
                .find_map(|(key, value)| {
                    (key.as_str() == Some("job"))
                        .then(|| value.as_str())
                        .flatten()
                })
                .unwrap_or_else(|| panic!("missing job name in need mapping: {item:?}"))
                .to_string()
        })
        .collect()
}

#[test]
fn gitlab_ci_pipeline_mirrors_internal_first_release_flow() {
    let text = read(".gitlab-ci.yml");
    let yaml: YamlValue = serde_yaml::from_str(&text).expect(".gitlab-ci.yml parses as YAML");

    assert_eq!(
        sequence_strings(top_level(&yaml, "stages")),
        vec![
            "quality".to_string(),
            "coverage".to_string(),
            "security".to_string(),
            "audit".to_string(),
            "release".to_string(),
            "deploy".to_string(),
        ]
    );

    assert_eq!(
        top_level(top_level(&yaml, "default"), "image").as_str(),
        Some("rust:1.95.0")
    );

    let workflow_rules = sequence_texts(top_level(top_level(&yaml, "workflow"), "rules"));
    assert!(workflow_rules
        .iter()
        .any(|rule| rule.contains("merge_request_event")));
    assert!(workflow_rules
        .iter()
        .any(|rule| rule.contains("$CI_COMMIT_BRANCH == \"main\"")));
    assert!(workflow_rules
        .iter()
        .any(|rule| rule.contains("$CI_COMMIT_TAG")));

    for job in [
        "quality",
        "coverage",
        "security",
        "audit",
        "release_audit_gate",
        "release_build_linux",
        "release_build_macos",
        "release_publish",
        "post_main_shadow",
    ] {
        assert!(
            top_level(&yaml, job).is_mapping(),
            "missing or malformed job `{job}`"
        );
    }

    for critical_job in [
        "quality",
        "coverage",
        "security",
        "audit",
        "release_audit_gate",
        "release_build_linux",
        "release_build_macos",
        "release_publish",
    ] {
        assert!(
            !has_top_level_key(top_level(&yaml, critical_job), "allow_failure"),
            "critical job `{critical_job}` must not be allowed to fail"
        );
    }

    assert_eq!(
        sequence_strings(top_level(top_level(&yaml, "security"), "dependencies")),
        Vec::<String>::new(),
        "security job should not inherit generated artifacts from earlier stages"
    );

    assert!(text.contains("bash ops/ci/quality-gates.sh"));
    assert!(text.contains("bash ops/ci/coverage-llvm.sh"));
    assert!(text.contains("bash ops/ci/security-tools.sh"));
    assert!(text.contains("cargo run -p jankurai -- security run . --strict --profile ci --script tools/security-lane.sh --out target/jankurai/security/evidence.json"));
    assert!(text.contains("bash ops/ci/audit.sh"));
    assert!(text.contains("bash ops/ci/release-audit-gate.sh"));
    assert!(text.contains("bash ops/ci/release-build.sh"));
    assert!(text.contains("bash ops/ci/release-publish.sh"));
    assert!(text.contains("bash ops/ci/post-main-shadow.sh"));
    assert!(text.contains("JERYU_SCCACHE_ENABLED: \"0\""));
    assert!(text.contains("CARGO_LLVM_COV_VERSION: \"0.6.16\""));
    assert!(text.contains("CARGO_MUTANTS_VERSION: \"27.0.0\""));
    assert!(text.contains("TARGET: x86_64-unknown-linux-gnu"));
    assert!(text.contains("TARGET: aarch64-apple-darwin"));

    assert!(text.contains("target/jankurai/repo-score.json"));
    assert!(text.contains("target/jankurai/repo-score.md"));
    assert!(text.contains("target/jankurai/repair-queue.jsonl"));
    assert!(text.contains("target/jankurai/security/evidence.json"));
    assert!(text.contains("target/jankurai/jeryu-shadow.json"));
    assert!(text.contains("target/jankurai/jeryu-shadow.md"));

    let audit_needs = sequence_strings(top_level(top_level(&yaml, "audit"), "needs"));
    assert_eq!(
        audit_needs,
        vec![
            "quality".to_string(),
            "coverage".to_string(),
            "security".to_string(),
        ]
    );

    let release_needs = need_job_names(top_level(top_level(&yaml, "release_publish"), "needs"));
    assert_eq!(
        release_needs,
        vec![
            "release_audit_gate".to_string(),
            "release_build_linux".to_string(),
            "release_build_macos".to_string(),
        ]
    );

    assert!(!text.contains("tags:"));
    assert!(!text.contains("when: manual"));
    assert!(!text.contains("optional: true"));
    assert_eq!(
        top_level(top_level(&yaml, "post_main_shadow"), "allow_failure").as_bool(),
        Some(true)
    );
}
