use jankurai::audit::{
    self,
    helpers::AuditContext,
    language_rules::{ci, docker, git, gittools, python, release, sql, typescript},
};
use jankurai::model::{FileInfo, Finding};
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn fixture_root() -> PathBuf {
    repo_root().join("crates/jankurai/tests/fixtures/language_bad_behavior")
}

fn read_fixture(rel: &str) -> String {
    fs::read_to_string(fixture_root().join(rel)).unwrap()
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn copy_fixture(repo: &Path, fixture_rel: &str, repo_rel: &str) {
    let text = read_fixture(fixture_rel);
    write(&repo.join(repo_rel), &text);
}

fn findings_for(repo: &Path, rule_id: &str) -> Vec<Finding> {
    audit::run_audit(repo, &[])
        .unwrap()
        .findings
        .into_iter()
        .filter(|finding| finding.rule_id.as_deref() == Some(rule_id))
        .collect()
}

fn assert_finding(findings: &[Finding], path: &str, matched_term: &str, evidence_term: &str) {
    let finding = findings
        .iter()
        .find(|finding| finding.path == path)
        .unwrap_or_else(|| panic!("missing finding for {path}: {findings:?}"));
    assert_eq!(
        finding.matched_term.as_deref(),
        Some(matched_term),
        "unexpected matched_term for {path}: {finding:?}"
    );
    assert!(
        finding
            .evidence
            .iter()
            .any(|value| value.to_ascii_lowercase().contains(evidence_term)),
        "finding evidence missing `{evidence_term}`: {:?}",
        finding.evidence
    );
}

fn assert_has_finding(findings: &[Finding], path: &str, matched_term: &str, evidence_term: &str) {
    assert!(
        findings.iter().any(|finding| {
            finding.path == path
                && finding.matched_term.as_deref() == Some(matched_term)
                && finding
                    .evidence
                    .iter()
                    .any(|value| value.to_ascii_lowercase().contains(evidence_term))
        }),
        "missing finding for {path} / {matched_term}: {findings:?}"
    );
}

fn file_info(rel_path: &str, text: &str) -> FileInfo {
    let name = Path::new(rel_path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let suffix = Path::new(rel_path)
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    FileInfo {
        rel_path: rel_path.into(),
        name,
        suffix,
        size: text.len() as u64,
        line_count: text.lines().count(),
        text: text.into(),
        is_generated: false,
        is_code: true,
    }
}

fn ctx_with_files(files: Vec<FileInfo>) -> AuditContext {
    let root = tempdir().unwrap();
    AuditContext {
        root: root.path().to_path_buf(),
        all_files: files.clone(),
        scope_files: files,
        scope_paths: vec![],
        self_audit: false,
        boundary_reclassifications: vec![],
    }
}

#[test]
fn sql_fixture_corpus_covers_risky_and_safe_cases() {
    let cases: &[(&str, &[&str])] = &[
        ("sql/risky/dynamic_sql.sql", &["EXECUTE", "||", "SELECT *"]),
        (
            "sql/risky/destructive_migration.sql",
            &["DROP TABLE", "CASCADE"],
        ),
        ("sql/risky/full_table_write.sql", &["DELETE FROM"]),
        ("sql/safe/parameterized.sql", &["DELETE FROM", "WHERE"]),
        (
            "sql/safe/proofed_migration.sql",
            &["rollback", "backup", "DROP TABLE"],
        ),
    ];

    for (rel, needles) in cases {
        let text = read_fixture(rel);
        for needle in *needles {
            assert!(
                text.contains(needle),
                "fixture {rel} missing `{needle}`\n{text}"
            );
        }
    }
}

#[test]
fn sql_risky_fixtures_emit_hlt030_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "sql/risky/dynamic_sql.sql",
        "src/dynamic_sql.sql",
    );
    copy_fixture(
        repo.path(),
        "sql/risky/destructive_migration.sql",
        "db/migrations/001_destructive.sql",
    );
    copy_fixture(
        repo.path(),
        "sql/risky/full_table_write.sql",
        "src/full_table_write.sql",
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_eq!(findings.len(), 4, "{findings:?}");
    assert_finding(
        &findings,
        "src/dynamic_sql.sql",
        "execute",
        "detector=sql.dynamic-sql",
    );
    assert_has_finding(
        &findings,
        "db/migrations/001_destructive.sql",
        "drop table",
        "detector=sql.migration.destructive-no-proof",
    );
    assert_has_finding(
        &findings,
        "db/migrations/001_destructive.sql",
        "cascade",
        "detector=sql.migration.cascade-convenience",
    );
    assert_finding(
        &findings,
        "src/full_table_write.sql",
        "update/delete",
        "detector=sql.query.full-table-write",
    );
}

#[test]
fn sql_safe_fixtures_emit_no_hlt030_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "sql/safe/parameterized.sql",
        "src/parameterized.sql",
    );
    copy_fixture(
        repo.path(),
        "sql/safe/proofed_migration.sql",
        "db/migrations/002_proofed.sql",
    );
    write(
        &repo.path().join("db/migrations/002_proofed.meta.toml"),
        r#"
owner = "db-platform"
approval = "fixture-approved"
rollback = "roll-forward via restore"
backup = "fixture restore drill"
lock_timeout = "5s"
statement_timeout = "30s"
verify = "002_proofed.verify.sql"
dependency_inventory = ["old_sessions dependencies reviewed"]
"#,
    );
    write(
        &repo.path().join("db/migrations/002_proofed.verify.sql"),
        "SELECT count(*) >= 0 FROM pg_class;\n",
    );

    assert!(findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR").is_empty());
}

#[test]
fn python_fixture_corpus_covers_risky_and_safe_cases() {
    let cases: &[(&str, &[&str])] = &[
        ("python/risky/dynamic_code.py", &["eval("]),
        ("python/risky/unsafe_deser.py", &["pickle.loads"]),
        ("python/risky/shell_dynamic.py", &["shell=True"]),
        (
            "python/risky/sql_string_built.py",
            &["cursor.execute", "f\"SELECT *"],
        ),
        ("python/risky/tls_debug.py", &["verify=False"]),
        ("python/safe/parameterized.py", &["cursor.execute", "%s"]),
        ("python/safe/safe_deser.py", &["safe_load"]),
        (
            "python/safe/safe_shell.py",
            &["subprocess.run", "[\"git\", \"status\"]"],
        ),
        ("python/safe/secure_tls.py", &["verify=True"]),
    ];

    for (rel, needles) in cases {
        let text = read_fixture(rel);
        for needle in *needles {
            assert!(
                text.contains(needle),
                "fixture {rel} missing `{needle}`\n{text}"
            );
        }
    }
}

#[test]
fn python_risky_fixtures_emit_hlt033_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "python/risky/dynamic_code.py",
        "src/dynamic_code.py",
    );
    copy_fixture(
        repo.path(),
        "python/risky/unsafe_deser.py",
        "src/unsafe_deser.py",
    );
    copy_fixture(
        repo.path(),
        "python/risky/shell_dynamic.py",
        "src/shell_dynamic.py",
    );
    copy_fixture(
        repo.path(),
        "python/risky/sql_string_built.py",
        "src/sql_string_built.py",
    );
    copy_fixture(repo.path(), "python/risky/tls_debug.py", "src/tls_debug.py");

    let findings = findings_for(repo.path(), "HLT-033-PYTHON-BAD-BEHAVIOR");
    assert_eq!(findings.len(), 5, "{findings:?}");
    assert_finding(
        &findings,
        "src/dynamic_code.py",
        "python.exec.dynamic-code",
        "detector=python.exec.dynamic-code",
    );
    assert_finding(
        &findings,
        "src/unsafe_deser.py",
        "python.deser.unsafe-object",
        "detector=python.deser.unsafe-object",
    );
    assert_finding(
        &findings,
        "src/shell_dynamic.py",
        "python.shell.dynamic",
        "detector=python.shell.dynamic",
    );
    assert_finding(
        &findings,
        "src/sql_string_built.py",
        "python.sql.string-built",
        "detector=python.sql.string-built",
    );
    assert_finding(
        &findings,
        "src/tls_debug.py",
        "python.net.tls-debug",
        "detector=python.net.tls-debug",
    );
}

#[test]
fn python_safe_fixtures_emit_no_hlt033_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "python/safe/parameterized.py",
        "src/parameterized.py",
    );
    copy_fixture(
        repo.path(),
        "python/safe/safe_deser.py",
        "src/safe_deser.py",
    );
    copy_fixture(
        repo.path(),
        "python/safe/safe_shell.py",
        "src/safe_shell.py",
    );
    copy_fixture(
        repo.path(),
        "python/safe/secure_tls.py",
        "src/secure_tls.py",
    );

    assert!(findings_for(repo.path(), "HLT-033-PYTHON-BAD-BEHAVIOR").is_empty());
}

#[test]
fn docs_tips_reference_and_generated_paths_stay_out_of_sql_python_scans() {
    let repo = tempdir().unwrap();
    write(
        &repo.path().join("docs/migration.sql"),
        "DROP TABLE docs_users;\n",
    );
    write(&repo.path().join("tips/unsafe.py"), "eval(payload)\n");
    write(
        &repo.path().join("reference/query.sql"),
        "DELETE FROM reference_rows;\n",
    );
    write(
        &repo.path().join("generated/script.py"),
        "subprocess.run(command, shell=True)\n",
    );

    assert!(findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR").is_empty());
    assert!(findings_for(repo.path(), "HLT-033-PYTHON-BAD-BEHAVIOR").is_empty());
    assert!(findings_for(repo.path(), "HLT-031-TYPESCRIPT-BAD-BEHAVIOR").is_empty());
    assert!(findings_for(repo.path(), "HLT-032-DOCKER-BAD-BEHAVIOR").is_empty());
    assert!(findings_for(repo.path(), "HLT-034-CI-BAD-BEHAVIOR").is_empty());
    assert!(findings_for(repo.path(), "HLT-035-GIT-BAD-BEHAVIOR").is_empty());
    assert!(findings_for(repo.path(), "HLT-036-GITTOOLS-BAD-BEHAVIOR").is_empty());
    assert!(findings_for(repo.path(), "HLT-037-RELEASE-BAD-BEHAVIOR").is_empty());
}

#[test]
fn language_bad_behavior_lane_and_test_map_are_routed() {
    let repo = repo_root();
    let test_map: JsonValue =
        serde_json::from_str(&fs::read_to_string(repo.join("agent/test-map.json")).unwrap())
            .unwrap();
    let entries = test_map["tests"]
        .as_object()
        .expect("test map must contain a tests object");
    for path in [
        "crates/jankurai/tests/language_bad_behavior.rs",
        "crates/jankurai/tests/fixtures/language_bad_behavior/",
        "crates/jankurai/tests/fixtures/language_bad_behavior/sql/",
        "crates/jankurai/tests/fixtures/language_bad_behavior/typescript/",
        "crates/jankurai/tests/fixtures/language_bad_behavior/docker/",
        "crates/jankurai/tests/fixtures/language_bad_behavior/python/",
        "crates/jankurai/tests/fixtures/language_bad_behavior/ci/",
        "crates/jankurai/tests/fixtures/language_bad_behavior/git/",
        "crates/jankurai/tests/fixtures/language_bad_behavior/gittools/",
        "crates/jankurai/tests/fixtures/language_bad_behavior/release/",
    ] {
        let entry = entries
            .get(path)
            .unwrap_or_else(|| panic!("missing test-map entry for {path}"));
        assert_eq!(
            entry["command"], "cargo test -p jankurai --test language_bad_behavior",
            "unexpected command for {path}"
        );
    }

    let lanes: toml::Value =
        toml::from_str(&fs::read_to_string(repo.join("agent/proof-lanes.toml")).unwrap()).unwrap();
    let lane_entries = lanes["lane"]
        .as_array()
        .expect("proof lanes must be an array");
    let lane = lane_entries
        .iter()
        .find(|entry| {
            entry.get("command_id").and_then(|v| v.as_str()) == Some("lane.language-bad-behavior")
        })
        .expect("missing language-bad-behavior lane");
    let rules = lane
        .get("rules_covered")
        .and_then(|value| value.as_array())
        .expect("lane rules_covered must be an array");
    for rule_id in [
        "HLT-029-RUST-BAD-BEHAVIOR",
        "HLT-030-SQL-BAD-BEHAVIOR",
        "HLT-031-TYPESCRIPT-BAD-BEHAVIOR",
        "HLT-032-DOCKER-BAD-BEHAVIOR",
        "HLT-033-PYTHON-BAD-BEHAVIOR",
        "HLT-034-CI-BAD-BEHAVIOR",
        "HLT-035-GIT-BAD-BEHAVIOR",
        "HLT-036-GITTOOLS-BAD-BEHAVIOR",
        "HLT-037-RELEASE-BAD-BEHAVIOR",
    ] {
        assert!(
            rules.iter().any(|value| value.as_str() == Some(rule_id)),
            "missing {rule_id} from language-bad-behavior lane"
        );
    }
}

#[test]
fn sql_summary_reports_hard_and_advisory_counts() {
    let ctx = ctx_with_files(vec![file_info(
        "src/query.sql",
        "SELECT * FROM users;\nDELETE FROM logs;\n",
    )]);
    let summary = sql::summary(&ctx);
    assert_eq!(summary.hard_findings, 1, "{summary:?}");
    assert_eq!(summary.advisory_signals, 1, "{summary:?}");
}

#[test]
fn python_summary_reports_hard_and_advisory_counts() {
    let ctx = ctx_with_files(vec![file_info(
        "src/task.py",
        "eval(payload)\nexcept Exception:\n",
    )]);
    let summary = python::summary(&ctx);
    assert_eq!(summary.hard_findings, 1, "{summary:?}");
    assert_eq!(summary.advisory_signals, 1, "{summary:?}");
}

#[test]
fn catalog_entries_reference_stable_hlt_rules() {
    let sql_rules = sql::catalog();
    let python_rules = python::catalog();
    let typescript_rules = typescript::catalog();
    let docker_rules = docker::catalog();
    let ci_rules = ci::catalog();
    let git_rules = git::catalog();
    let gittools_rules = gittools::catalog();
    let release_rules = release::catalog();
    let mut ids = HashSet::new();
    for rule in sql_rules
        .iter()
        .chain(python_rules.iter())
        .chain(typescript_rules.iter())
        .chain(docker_rules.iter())
        .chain(ci_rules.iter())
        .chain(git_rules.iter())
        .chain(gittools_rules.iter())
        .chain(release_rules.iter())
    {
        ids.insert(rule.hlt_rule_id);
    }
    assert!(ids.contains("HLT-030-SQL-BAD-BEHAVIOR"));
    assert!(ids.contains("HLT-033-PYTHON-BAD-BEHAVIOR"));
    assert!(ids.contains("HLT-031-TYPESCRIPT-BAD-BEHAVIOR"));
    assert!(ids.contains("HLT-032-DOCKER-BAD-BEHAVIOR"));
    assert!(ids.contains("HLT-034-CI-BAD-BEHAVIOR"));
    assert!(ids.contains("HLT-035-GIT-BAD-BEHAVIOR"));
    assert!(ids.contains("HLT-036-GITTOOLS-BAD-BEHAVIOR"));
    assert!(ids.contains("HLT-037-RELEASE-BAD-BEHAVIOR"));
    assert!(sql_rules
        .iter()
        .all(|rule| rule.hlt_rule_id == "HLT-030-SQL-BAD-BEHAVIOR"));
    assert!(python_rules
        .iter()
        .all(|rule| rule.hlt_rule_id == "HLT-033-PYTHON-BAD-BEHAVIOR"));
    assert!(typescript_rules
        .iter()
        .all(|rule| rule.hlt_rule_id == "HLT-031-TYPESCRIPT-BAD-BEHAVIOR"));
    assert!(docker_rules
        .iter()
        .all(|rule| rule.hlt_rule_id == "HLT-032-DOCKER-BAD-BEHAVIOR"));
    assert!(ci_rules
        .iter()
        .all(|rule| rule.hlt_rule_id == "HLT-034-CI-BAD-BEHAVIOR"));
    assert!(git_rules
        .iter()
        .all(|rule| rule.hlt_rule_id == "HLT-035-GIT-BAD-BEHAVIOR"));
    assert!(gittools_rules
        .iter()
        .all(|rule| rule.hlt_rule_id == "HLT-036-GITTOOLS-BAD-BEHAVIOR"));
    assert!(release_rules
        .iter()
        .all(|rule| rule.hlt_rule_id == "HLT-037-RELEASE-BAD-BEHAVIOR"));
}

#[test]
fn typescript_fixture_corpus_covers_risky_and_safe_cases() {
    let cases: &[(&str, &[&str])] = &[
        ("typescript/risky/suppress.ts", &["@ts-nocheck"]),
        ("typescript/risky/any_boundary.ts", &["as any"]),
        ("typescript/risky/dangerous_eval.ts", &["eval("]),
        ("typescript/risky/dangerous_dom.ts", &["innerHTML"]),
        ("typescript/risky/raw_command_sql.ts", &["req.body.command"]),
        ("typescript/risky/raw_sql.ts", &["req.body.id"]),
        ("typescript/risky/strict_false.json", &["\"strict\": false"]),
        ("typescript/safe/guarded.ts", &["textContent"]),
        ("typescript/safe/strict_true.json", &["\"strict\": true"]),
    ];

    for (rel, needles) in cases {
        let text = read_fixture(rel);
        for needle in *needles {
            assert!(
                text.contains(needle),
                "fixture {rel} missing `{needle}`\n{text}"
            );
        }
    }
}

#[test]
fn docker_fixture_corpus_covers_risky_and_safe_cases() {
    let cases: &[(&str, &[&str])] = &[
        (
            "docker/risky/Dockerfile",
            &["FROM node:latest", "curl", ".env"],
        ),
        (
            "docker/risky/docker-compose.yml",
            &[
                "privileged: true",
                "network_mode: host",
                "/var/run/docker.sock",
                "seccomp=unconfined",
            ],
        ),
        ("docker/safe/Dockerfile", &["HEALTHCHECK", "USER node"]),
        ("docker/safe/docker-compose.yml", &["127.0.0.1:5432:5432"]),
    ];

    for (rel, needles) in cases {
        let text = read_fixture(rel);
        for needle in *needles {
            assert!(
                text.contains(needle),
                "fixture {rel} missing `{needle}`\n{text}"
            );
        }
    }
}

#[test]
fn ci_fixture_corpus_covers_risky_and_safe_cases() {
    let cases: &[(&str, &[&str])] = &[
        (
            "ci/risky/risky.yml",
            &[
                "pull_request_target",
                "write-all",
                "secrets.DEPLOY_TOKEN",
                "actions/checkout@main",
                "actions/upload-artifact@latest",
            ],
        ),
        (
            "ci/safe/safe.yml",
            &[
                "pull_request",
                "contents: read",
                "actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd",
                "timeout-minutes",
                "concurrency",
            ],
        ),
    ];

    for (rel, needles) in cases {
        let text = read_fixture(rel);
        for needle in *needles {
            assert!(
                text.contains(needle),
                "fixture {rel} missing `{needle}`\n{text}"
            );
        }
    }
}

#[test]
fn git_fixture_corpus_covers_risky_and_safe_cases() {
    let cases: &[(&str, &[&str])] = &[
        (
            "git/risky/scripts/release.sh",
            &[
                "git reset --hard",
                "git clean -fdx",
                "git stash -u",
                "git add .",
                "git commit -am",
                "git push --force",
                "git branch -D",
                "git worktree remove --force",
            ],
        ),
        (
            "git/safe/scripts/status.sh",
            &["git status --porcelain", "git diff --quiet"],
        ),
    ];

    for (rel, needles) in cases {
        let text = read_fixture(rel);
        for needle in *needles {
            assert!(
                text.contains(needle),
                "fixture {rel} missing `{needle}`\n{text}"
            );
        }
    }
}

#[test]
fn gittools_fixture_corpus_covers_risky_and_safe_cases() {
    let cases: &[(&str, &[&str])] = &[
        (
            "gittools/risky/.husky/pre-commit",
            &["git reset --hard", "git add ."],
        ),
        (
            "gittools/risky/package.json",
            &["git commit --no-verify", "lint-staged", "git add ."],
        ),
        (
            "gittools/risky/.pre-commit-config.yaml",
            &["git push --no-verify"],
        ),
        ("gittools/risky/lefthook.yml", &["git clean -fdx"]),
        (
            "gittools/risky/scripts/install-hooks.sh",
            &["core.hooksPath /dev/null", ".git/hooks/pre-commit"],
        ),
        ("gittools/safe/.husky/pre-commit", &["npm run lint:staged"]),
        (
            "gittools/safe/package.json",
            &["lint-staged", "eslint --fix"],
        ),
        (
            "gittools/safe/.pre-commit-config.yaml",
            &["rev: v4.6.0", "check-yaml"],
        ),
        (
            "gittools/safe/.github/workflows/quality.yml",
            &["pre-commit run --all-files"],
        ),
    ];

    for (rel, needles) in cases {
        let text = read_fixture(rel);
        for needle in *needles {
            assert!(
                text.contains(needle),
                "fixture {rel} missing `{needle}`\n{text}"
            );
        }
    }
}

#[test]
fn release_fixture_corpus_covers_risky_and_safe_cases() {
    let cases: &[(&str, &[&str])] = &[
        (
            "release/risky/scripts/release.sh",
            &[
                "SKIP_TESTS=1",
                "git tag -f",
                "gh release upload",
                "--clobber",
                ".env",
                ":latest",
                "cargo publish --no-verify",
            ],
        ),
        (
            "release/safe/scripts/release.sh",
            &[
                "git tag -s",
                "git tag -v",
                "sha256sum",
                "sbom.spdx.json",
                "cosign attest",
                "gh release create",
                "--verify-tag",
            ],
        ),
    ];

    for (rel, needles) in cases {
        let text = read_fixture(rel);
        for needle in *needles {
            assert!(
                text.contains(needle),
                "fixture {rel} missing `{needle}`\n{text}"
            );
        }
    }
}

#[test]
fn typescript_risky_fixtures_emit_hlt031_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "typescript/risky/suppress.ts",
        "src/suppress.ts",
    );
    copy_fixture(
        repo.path(),
        "typescript/risky/any_boundary.ts",
        "src/any_boundary.ts",
    );
    copy_fixture(
        repo.path(),
        "typescript/risky/dangerous_eval.ts",
        "src/dangerous_eval.ts",
    );
    copy_fixture(
        repo.path(),
        "typescript/risky/dangerous_dom.ts",
        "src/dangerous_dom.ts",
    );
    copy_fixture(
        repo.path(),
        "typescript/risky/raw_command_sql.ts",
        "src/raw_command_sql.ts",
    );
    copy_fixture(repo.path(), "typescript/risky/raw_sql.ts", "src/raw_sql.ts");
    copy_fixture(
        repo.path(),
        "typescript/risky/strict_false.json",
        "tsconfig.json",
    );

    let findings = findings_for(repo.path(), "HLT-031-TYPESCRIPT-BAD-BEHAVIOR");
    assert_eq!(findings.len(), 9, "{findings:?}");
    assert_has_finding(
        &findings,
        "src/suppress.ts",
        "typescript.suppress.ts-nocheck",
        "detector=typescript.suppress.ts-nocheck",
    );
    assert_has_finding(
        &findings,
        "src/any_boundary.ts",
        "typescript.types.any-boundary",
        "detector=typescript.types.any-boundary",
    );
    assert_has_finding(
        &findings,
        "src/dangerous_eval.ts",
        "typescript.runtime.dangerous-eval-dom",
        "detector=typescript.runtime.dangerous-eval-dom",
    );
    assert_has_finding(
        &findings,
        "src/dangerous_dom.ts",
        "typescript.runtime.dangerous-eval-dom",
        "detector=typescript.runtime.dangerous-eval-dom",
    );
    assert_has_finding(
        &findings,
        "src/raw_command_sql.ts",
        "typescript.security.raw-command-sql",
        "detector=typescript.security.raw-command-sql",
    );
    assert_has_finding(
        &findings,
        "src/raw_sql.ts",
        "typescript.security.raw-command-sql",
        "detector=typescript.security.raw-command-sql",
    );
    assert!(
        findings
            .iter()
            .filter(|finding| finding.path == "tsconfig.json")
            .count()
            == 3,
        "{findings:?}"
    );
}

#[test]
fn typescript_safe_fixtures_emit_no_hlt031_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(repo.path(), "typescript/safe/guarded.ts", "src/guarded.ts");
    copy_fixture(
        repo.path(),
        "typescript/safe/strict_true.json",
        "tsconfig.json",
    );

    assert!(findings_for(repo.path(), "HLT-031-TYPESCRIPT-BAD-BEHAVIOR").is_empty());
}

#[test]
fn docker_risky_fixtures_emit_hlt032_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(repo.path(), "docker/risky/Dockerfile", "Dockerfile");
    copy_fixture(
        repo.path(),
        "docker/risky/docker-compose.yml",
        "docker-compose.yml",
    );

    let findings = findings_for(repo.path(), "HLT-032-DOCKER-BAD-BEHAVIOR");
    assert_eq!(findings.len(), 10, "{findings:?}");
    assert_has_finding(
        &findings,
        "Dockerfile",
        "docker.image.mutable-tag",
        "detector=docker.image.mutable-tag",
    );
    assert_has_finding(
        &findings,
        "Dockerfile",
        "docker.install.unverified-remote",
        "detector=docker.install.unverified-remote",
    );
    assert_has_finding(
        &findings,
        "Dockerfile",
        "docker.secret.in-layer",
        "detector=docker.secret.in-layer",
    );
    assert_has_finding(
        &findings,
        "docker-compose.yml",
        "docker.compose.privileged",
        "detector=docker.compose.privileged",
    );
    assert_has_finding(
        &findings,
        "docker-compose.yml",
        "docker.compose.host-namespace",
        "detector=docker.compose.host-namespace",
    );
    assert_has_finding(
        &findings,
        "docker-compose.yml",
        "docker.compose.dangerous-capability",
        "detector=docker.compose.dangerous-capability",
    );
    assert_has_finding(
        &findings,
        "docker-compose.yml",
        "docker.compose.socket-mount",
        "detector=docker.compose.socket-mount",
    );
    assert_has_finding(
        &findings,
        "docker-compose.yml",
        "docker.confinement.disabled",
        "detector=docker.confinement.disabled",
    );
    assert_has_finding(
        &findings,
        "docker-compose.yml",
        "docker.port.public-db-admin",
        "detector=docker.port.public-db-admin",
    );
}

#[test]
fn docker_safe_fixtures_emit_no_hlt032_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(repo.path(), "docker/safe/Dockerfile", "Dockerfile");
    copy_fixture(
        repo.path(),
        "docker/safe/docker-compose.yml",
        "docker-compose.yml",
    );

    assert!(findings_for(repo.path(), "HLT-032-DOCKER-BAD-BEHAVIOR").is_empty());
}

#[test]
fn ci_risky_fixtures_emit_hlt034_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "ci/risky/risky.yml",
        ".github/workflows/risky.yml",
    );

    let findings = findings_for(repo.path(), "HLT-034-CI-BAD-BEHAVIOR");
    assert_eq!(findings.len(), 13, "{findings:?}");
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.timeout.missing",
        "detector=ci.timeout.missing",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.concurrency.missing",
        "detector=ci.concurrency.missing",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.github.pull-request-target-checkout-head",
        "detector=ci.github.pull-request-target-checkout-head",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.permissions.write-all",
        "detector=ci.permissions.write-all",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.secret.echo-or-debug",
        "detector=ci.secret.echo-or-debug",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.secret.echo-or-debug",
        "detector=ci.secret.echo-or-debug",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.security-scan.nonblocking",
        "detector=ci.security-scan.nonblocking",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.action.mutable-ref",
        "detector=ci.action.mutable-ref",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.action.not-full-sha",
        "detector=ci.action.not-full-sha",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.action.not-full-sha",
        "detector=ci.action.not-full-sha",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.untrusted-runner.privileged-docker",
        "detector=ci.untrusted-runner.privileged-docker",
    );
    assert_has_finding(
        &findings,
        ".github/workflows/risky.yml",
        "ci.artifact.cache.secret-path",
        "detector=ci.artifact.cache.secret-path",
    );
}

#[test]
fn ci_safe_fixtures_emit_no_hlt034_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "ci/safe/safe.yml",
        ".github/workflows/safe.yml",
    );

    assert!(findings_for(repo.path(), "HLT-034-CI-BAD-BEHAVIOR").is_empty());
}

#[test]
fn git_risky_fixtures_emit_hlt035_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "git/risky/scripts/release.sh",
        "scripts/release.sh",
    );

    let findings = findings_for(repo.path(), "HLT-035-GIT-BAD-BEHAVIOR");
    assert_eq!(findings.len(), 9, "{findings:?}");
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "git.destructive.reset-hard",
        "detector=git.destructive.reset-hard",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "git.destructive.clean",
        "detector=git.destructive.clean",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "git.stash.hidden-state",
        "detector=git.stash.hidden-state",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "git.stage.unbounded",
        "detector=git.stage.unbounded",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "git.remote.force-mutation",
        "detector=git.remote.force-mutation",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "git.refs.destructive",
        "detector=git.refs.destructive",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "git.worktree.force-cleanup",
        "detector=git.worktree.force-cleanup",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "git.remote.credential-url",
        "detector=git.remote.credential-url",
    );
}

#[test]
fn git_safe_fixtures_emit_no_hlt035_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "git/safe/scripts/status.sh",
        "scripts/status.sh",
    );

    assert!(findings_for(repo.path(), "HLT-035-GIT-BAD-BEHAVIOR").is_empty());
}

#[test]
fn gittools_risky_fixtures_emit_hlt036_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "gittools/risky/.husky/pre-commit",
        ".husky/pre-commit",
    );
    copy_fixture(repo.path(), "gittools/risky/package.json", "package.json");
    copy_fixture(
        repo.path(),
        "gittools/risky/.pre-commit-config.yaml",
        ".pre-commit-config.yaml",
    );
    copy_fixture(repo.path(), "gittools/risky/lefthook.yml", "lefthook.yml");
    copy_fixture(
        repo.path(),
        "gittools/risky/scripts/install-hooks.sh",
        "scripts/install-hooks.sh",
    );

    let report = audit::run_audit(repo.path(), &[]).unwrap();
    assert!(
        report
            .caps_applied
            .iter()
            .any(|cap| cap == "gittools-bad-behavior"),
        "{:?}",
        report.caps_applied
    );
    let findings: Vec<_> = report
        .findings
        .into_iter()
        .filter(|finding| finding.rule_id.as_deref() == Some("HLT-036-GITTOOLS-BAD-BEHAVIOR"))
        .collect();
    assert_eq!(findings.len(), 9, "{findings:?}");
    assert_has_finding(
        &findings,
        ".husky/pre-commit",
        "gittools.hook.destructive-git-command",
        "detector=gittools.hook.destructive-git-command",
    );
    assert_has_finding(
        &findings,
        ".husky/pre-commit",
        "gittools.hook.unbounded-stage",
        "detector=gittools.hook.unbounded-stage",
    );
    assert_has_finding(
        &findings,
        "package.json",
        "gittools.bypass.no-verify-automation",
        "proof_window=none",
    );
    assert_has_finding(
        &findings,
        "package.json",
        "gittools.lint-staged.manual-restage",
        "snippet=",
    );
    assert_has_finding(
        &findings,
        ".pre-commit-config.yaml",
        "gittools.bypass.no-verify-automation",
        "line=",
    );
    assert_has_finding(
        &findings,
        "lefthook.yml",
        "gittools.hook.destructive-git-command",
        "detector=gittools.hook.destructive-git-command",
    );
    assert_has_finding(
        &findings,
        "scripts/install-hooks.sh",
        "gittools.hooks-path.disabled",
        "detector=gittools.hooks-path.disabled",
    );
    assert_has_finding(
        &findings,
        "scripts/install-hooks.sh",
        "gittools.raw-hooks.unversioned-install",
        "detector=gittools.raw-hooks.unversioned-install",
    );
}

#[test]
fn gittools_safe_fixtures_emit_no_hlt036_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "gittools/safe/.husky/pre-commit",
        ".husky/pre-commit",
    );
    copy_fixture(repo.path(), "gittools/safe/package.json", "package.json");
    copy_fixture(
        repo.path(),
        "gittools/safe/.pre-commit-config.yaml",
        ".pre-commit-config.yaml",
    );
    copy_fixture(
        repo.path(),
        "gittools/safe/.github/workflows/quality.yml",
        ".github/workflows/quality.yml",
    );

    assert!(findings_for(repo.path(), "HLT-036-GITTOOLS-BAD-BEHAVIOR").is_empty());
}

#[test]
fn release_risky_fixtures_emit_hlt037_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "release/risky/scripts/release.sh",
        "scripts/release.sh",
    );

    let report = audit::run_audit(repo.path(), &[]).unwrap();
    assert!(
        report
            .caps_applied
            .iter()
            .any(|cap| cap == "release-bad-behavior"),
        "{:?}",
        report.caps_applied
    );
    let findings: Vec<_> = report
        .findings
        .into_iter()
        .filter(|finding| finding.rule_id.as_deref() == Some("HLT-037-RELEASE-BAD-BEHAVIOR"))
        .collect();
    assert!(
        findings.len() >= 8,
        "expected broad release findings: {findings:?}"
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "release.verification.skipped",
        "detector=release.verification.skipped",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "release.git.mutable-tag",
        "detector=release.git.mutable-tag",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "release.asset.mutable-upload",
        "detector=release.asset.mutable-upload",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "release.artifact.mutable-latest",
        "detector=release.artifact.mutable-latest",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "release.secret.packaged",
        "detector=release.secret.packaged",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "release.gh.unverified-tag",
        "detector=release.gh.unverified-tag",
    );
    assert_has_finding(
        &findings,
        "scripts/release.sh",
        "release.integrity.missing",
        "detector=release.integrity.missing",
    );
}

#[test]
fn release_safe_fixtures_emit_no_hlt037_findings() {
    let repo = tempdir().unwrap();
    copy_fixture(
        repo.path(),
        "release/safe/scripts/release.sh",
        "scripts/release.sh",
    );

    assert!(findings_for(repo.path(), "HLT-037-RELEASE-BAD-BEHAVIOR").is_empty());
}

#[test]
fn release_summary_reports_hard_and_advisory_counts() {
    let ctx = ctx_with_files(vec![file_info(
        "scripts/release.sh",
        "gh release create v1.2.3 dist/app\n",
    )]);
    let summary = release::summary(&ctx);
    assert!(summary.hard_findings >= 1, "{summary:?}");
    assert_eq!(summary.advisory_signals, 1, "{summary:?}");
}

#[test]
fn gittools_summary_reports_hard_and_advisory_counts() {
    let ctx = ctx_with_files(vec![file_info(
        ".husky/pre-commit",
        "git add .\ncargo test\n",
    )]);
    let summary = gittools::summary(&ctx);
    assert_eq!(summary.hard_findings, 1, "{summary:?}");
    assert_eq!(summary.advisory_signals, 2, "{summary:?}");
}

#[test]
fn gittools_surfaces_do_not_double_score_as_generic_git() {
    let repo = tempdir().unwrap();
    write(
        &repo.path().join(".husky/pre-commit"),
        "git reset --hard HEAD\n",
    );

    assert!(findings_for(repo.path(), "HLT-035-GIT-BAD-BEHAVIOR").is_empty());
    assert!(!findings_for(repo.path(), "HLT-036-GITTOOLS-BAD-BEHAVIOR").is_empty());
}
