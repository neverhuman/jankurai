use jankurai::audit::run_audit;
use jankurai::commands::context_data::RepoCatalog;
use jankurai::report::sarif;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/jankurai")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn write_minimal_standard_repo(dir: &Path) {
    fs::create_dir_all(dir.join("agent")).unwrap();
    fs::write(
        dir.join("AGENTS.md"),
        "Read `agent/JANKURAI_STANDARD.md` first.\n",
    )
    .unwrap();
    fs::write(
        dir.join("README.md"),
        "# Repo\n\nlayout map validate workspace\n",
    )
    .unwrap();
    fs::write(dir.join("Justfile"), "check:\n    cargo test\n").unwrap();
    fs::write(
        dir.join("agent/JANKURAI_STANDARD.md"),
        "Standard version: `0.5.0`\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("docs")).unwrap();
    fs::write(
        dir.join("docs/agent-native-standard.md"),
        "Standard version: `0.5.0`\n",
    )
    .unwrap();
}

#[test]
fn destructive_migration_sql_applies_destructive_migration_risk_cap() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__drop.sql"),
        "DROP TABLE users;\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(
        report
            .caps_applied
            .iter()
            .any(|c| c == "destructive-migration-risk"),
        "expected destructive-migration-risk cap, caps={:?}",
        report.caps_applied
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION")),
        "expected HLT-021 finding alongside cap"
    );
}

#[test]
fn truncate_table_triggers_hlt021() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__trunc.sql"),
        "TRUNCATE TABLE staging_import;\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.findings.iter().any(|f| {
        f.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION")
            && f.evidence.iter().any(|e| e.contains("truncate"))
    }));
}

#[test]
fn jankurai_migration_safe_marker_alone_does_not_suppress_destructive_finding() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__drop.sql"),
        "-- jankurai:migration-safe (human-approved exceptional drop)\nDROP TABLE legacy_import;\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report
        .findings
        .iter()
        .any(|f| { f.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION") }));
}

#[test]
fn perfect_web_api_db_fixture_has_no_destructive_migration_finding() {
    let root = workspace_root();
    let example = root.join("examples/perfect-web-api-db");
    assert!(
        example.join("db/migrations/001_init.sql").is_file(),
        "expected example fixture at {}",
        example.display()
    );
    let report = run_audit(&example, &[]).unwrap();
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION")),
        "golden example should not flag HLT-021: {:?}",
        report.findings
    );
}

#[test]
fn db_migrations_paths_resolve_to_db_migration_analyze_lane() {
    let root = workspace_root();
    let catalog = RepoCatalog::load(&root).expect("repo catalog");
    let path = "db/migrations/V1__x.sql";
    let (route, spec) = catalog
        .test_route_for_path(path)
        .unwrap_or_else(|| panic!("no test-map route for {path}"));
    assert_eq!(route.prefix, "db/migrations");
    assert_eq!(route.match_kind, "directory");
    let lane = catalog
        .proof_lane_for_command(spec.command.trim())
        .unwrap_or_else(|| panic!("no proof lane for command {}", spec.command));
    assert_eq!(lane, "db-migration-analyze");
}

#[test]
fn destructive_migration_sql_triggers_hlt021() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__drop.sql"),
        "DROP TABLE users;\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION")),
        "expected HLT-021 finding, got: {:?}",
        report
            .findings
            .iter()
            .map(|f| f.rule_id.as_deref())
            .collect::<Vec<_>>()
    );
}

#[test]
fn destructive_migration_suppressed_when_safety_evidence_present() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__drop.sql"),
        "DROP TABLE users;\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__drop.meta.toml"),
        r#"
owner = "db-platform"
approval = "CAB-2026-05-09"
rollback = "roll-forward via restore of archived users"
backup = "PITR restore drill 2026-05-09"
lock_timeout = "5s"
statement_timeout = "30s"
verify = "V1__drop.verify.sql"
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__drop.verify.sql"),
        "SELECT count(*) >= 0 FROM pg_class;\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(!report
        .findings
        .iter()
        .any(|f| { f.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION") }));
}

#[test]
fn alter_table_add_column_is_not_flagged() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__add.sql"),
        "ALTER TABLE users ADD COLUMN nick text;\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(!report
        .findings
        .iter()
        .any(|f| { f.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION") }));
}

#[test]
fn multiline_delete_with_where_on_next_line_is_not_flagged() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__bounded_delete.sql"),
        "DELETE FROM stale_rows\nWHERE imported_at < now() - interval '90 days';\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION")),
        "DELETE ... WHERE on next line should not be destructive: {:?}",
        report.findings
    );
}

#[test]
fn delete_without_where_triggers_hlt021() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__purge.sql"),
        "DELETE FROM stale_rows;\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.findings.iter().any(|f| {
        f.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION")
            && f.evidence
                .iter()
                .any(|e| e.contains("delete without where"))
    }));
}

#[test]
fn hlt021_sarif_rule_help_uri_is_https_and_result_has_region() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__drop.sql"),
        "DROP TABLE users;\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    let sarif_text = sarif::render_sarif(&report);
    let v: serde_json::Value = serde_json::from_str(&sarif_text).unwrap();
    let rules = v["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("rules");
    let rule = rules
        .iter()
        .find(|r| r["id"] == "HLT-021-DESTRUCTIVE-MIGRATION")
        .expect("HLT-021 rule descriptor");
    let help = rule["helpUri"].as_str().expect("helpUri");
    assert!(
        help.starts_with("https://github.com/jeppsontaylor/jankurai/blob/main/"),
        "expected absolute helpUri, got {help}"
    );
    assert!(
        help.contains("BAD_MIGRATION.md"),
        "HLT-021 docs live under BAD_MIGRATION.md: {help}"
    );

    let results = v["runs"][0]["results"].as_array().expect("results");
    let hit = results
        .iter()
        .find(|r| r["ruleId"] == "HLT-021-DESTRUCTIVE-MIGRATION")
        .expect("HLT-021 result");
    let region = &hit["locations"][0]["physicalLocation"]["region"];
    assert_eq!(region["startLine"], region["endLine"]);
    assert!(
        region["snippet"]["text"]
            .as_str()
            .is_some_and(|t| !t.is_empty()),
        "expected snippet text"
    );
}

#[test]
fn concurrent_index_in_txn_triggers_hlt030_via_full_audit() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/V1__concurrent.sql"),
        "BEGIN;\nCREATE INDEX CONCURRENTLY idx_orders_user ON orders(user_id);\nCOMMIT;\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id.as_deref() == Some("HLT-030-SQL-BAD-BEHAVIOR")
                && f.evidence.iter().any(|e| e.contains("concurrent-in-txn"))),
        "expected HLT-030 concurrent-in-txn finding, got: {:?}",
        report.findings
    );
}

#[test]
fn no_migration_surface_produces_zero_migration_findings() {
    let dir = tempdir().unwrap();
    write_minimal_standard_repo(dir.path());
    // Only Rust code, no SQL or database setup at all
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/main.rs"),
        "fn main() { println!(\"hello\"); }\n",
    )
    .unwrap();
    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION")),
        "repos without SQL should have no HLT-021 findings"
    );
    // No migration-specific HLT-030 findings either
    assert!(
        !report.findings.iter().any(|f| f
            .rule_id
            .as_deref()
            .is_some_and(|r| r == "HLT-030-SQL-BAD-BEHAVIOR")
            && f.evidence.iter().any(|e| e.contains("sql.migration."))),
        "repos without SQL should have no migration-specific HLT-030 findings"
    );
}

#[test]
fn hlt021_docs_url_points_to_bad_migration_md() {
    let rule = jankurai::audit::rules::lookup("HLT-021-DESTRUCTIVE-MIGRATION")
        .expect("HLT-021 should exist");
    assert_eq!(
        rule.docs_url, "docs/BAD_MIGRATION.md",
        "HLT-021 should point to docs/BAD_MIGRATION.md"
    );
}
