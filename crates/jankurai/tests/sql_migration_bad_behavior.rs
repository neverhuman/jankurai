use jankurai::audit::run_audit;
use jankurai::model::Finding;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn write_minimal_standard_repo(dir: &Path) {
    write(
        &dir.join("AGENTS.md"),
        "Read `agent/JANKURAI_STANDARD.md` first.\n",
    );
    write(
        &dir.join("README.md"),
        "# Repo\n\nlayout map validate workspace\n",
    );
    write(&dir.join("Justfile"), "check:\n    cargo test\n");
    write(
        &dir.join("agent/JANKURAI_STANDARD.md"),
        "Standard version: `0.8.0`\n",
    );
    write(
        &dir.join("docs/agent-native-standard.md"),
        "Standard version: `0.8.0`\n",
    );
}

fn findings_for(repo: &Path, rule_id: &str) -> Vec<Finding> {
    run_audit(repo, &[])
        .unwrap()
        .findings
        .into_iter()
        .filter(|finding| finding.rule_id.as_deref() == Some(rule_id))
        .collect()
}

fn assert_detector(findings: &[Finding], detector: &str) {
    assert!(
        findings
            .iter()
            .any(|finding| finding.evidence.iter().any(|e| e.contains(detector))),
        "missing detector {detector}: {findings:?}"
    );
}

fn assert_no_detector(findings: &[Finding], detector: &str) {
    assert!(
        !findings
            .iter()
            .any(|finding| finding.evidence.iter().any(|e| e.contains(detector))),
        "unexpected detector {detector}: {findings:?}"
    );
}

fn write_complete_destructive_metadata(repo: &Path, stem: &str) {
    write(
        &repo.join(format!("db/migrations/{stem}.meta.toml")),
        r#"
owner = "db-platform"
approval = "CAB-2026-05-09"
rollback = "roll-forward via restore of archived table"
backup = "PITR restore drill 2026-05-09"
lock_timeout = "5s"
statement_timeout = "30s"
verify = "same-stem verify artifact"
affected_objects = ["public.sessions"]
"#,
    );
    write(
        &repo.join(format!("db/migrations/{stem}.verify.sql")),
        "SELECT count(*) >= 0 FROM pg_class;\n",
    );
}

#[test]
fn concurrent_index_inside_explicit_transaction_fires() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_concurrent.sql"),
        "BEGIN;\nCREATE INDEX CONCURRENTLY idx_orders_user ON orders(user_id);\nCOMMIT;\n",
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_detector(&findings, "sql.migration.concurrent-in-txn");
}

#[test]
fn concurrent_index_outside_transaction_is_safe() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_concurrent.sql"),
        "CREATE INDEX CONCURRENTLY idx_orders_user ON orders(user_id);\n",
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_no_detector(&findings, "sql.migration.concurrent-in-txn");
}

#[test]
fn begin_inside_function_body_does_not_open_transaction() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_function.sql"),
        r#"
DO $$
BEGIN
  CREATE INDEX CONCURRENTLY idx_orders_user ON orders(user_id);
END $$;
"#,
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_no_detector(&findings, "sql.migration.concurrent-in-txn");
}

#[test]
fn hlt021_rejects_comment_only_migration_safe_marker() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_drop.sql"),
        "-- jankurai:migration-safe rollback backup approved\nDROP TABLE sessions;\n",
    );

    let findings = findings_for(repo.path(), "HLT-021-DESTRUCTIVE-MIGRATION");
    assert!(
        !findings.is_empty(),
        "comment-only migration-safe marker must not suppress HLT-021"
    );
}

#[test]
fn hlt021_accepts_structured_metadata_plus_verify_artifact() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_drop.sql"),
        "DROP TABLE sessions;\n",
    );
    write_complete_destructive_metadata(repo.path(), "001_drop");

    let findings = findings_for(repo.path(), "HLT-021-DESTRUCTIVE-MIGRATION");
    assert!(
        findings.is_empty(),
        "structured metadata plus verify artifact should suppress HLT-021: {findings:?}"
    );
}

#[test]
fn cascade_requires_structured_dependency_inventory() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_cascade.sql"),
        "DROP TABLE sessions CASCADE;\n",
    );
    write_complete_destructive_metadata(repo.path(), "001_cascade");

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_detector(&findings, "sql.migration.cascade-convenience");
}

#[test]
fn cascade_with_structured_dependency_inventory_is_safe() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_cascade.sql"),
        "DROP TABLE sessions CASCADE;\n",
    );
    write_complete_destructive_metadata(repo.path(), "001_cascade");
    write(
        &repo.path().join("db/migrations/migration.toml"),
        r#"
owner = "db-platform"
approval = "CAB-2026-05-09"
dependency_inventory = ["view old_sessions_v", "policy sessions_rls"]
approved = true
"#,
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_no_detector(&findings, "sql.migration.cascade-convenience");
}

#[test]
fn risky_postgres_ddl_missing_timeouts_fires_specific_detectors() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_alter.sql"),
        "ALTER TABLE accounts DROP COLUMN legacy_flag;\n",
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_detector(&findings, "sql.migration.missing-lock-timeout");
    assert_detector(&findings, "sql.migration.missing-statement-timeout");
}

#[test]
fn risky_postgres_ddl_missing_one_timeout_fires_only_that_detector() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_alter.sql"),
        "SET lock_timeout = '5s';\nALTER TABLE accounts DROP COLUMN legacy_flag;\n",
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_no_detector(&findings, "sql.migration.missing-lock-timeout");
    assert_detector(&findings, "sql.migration.missing-statement-timeout");
}

#[test]
fn multiline_update_with_where_is_not_full_table_write() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_backfill.sql"),
        "UPDATE accounts\nSET normalized_email = lower(email)\nWHERE normalized_email IS NULL;\n",
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_no_detector(&findings, "sql.migration.full-table-write");
}

#[test]
fn unbounded_update_and_delete_in_migration_fire() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_backfill.sql"),
        "UPDATE accounts SET normalized_email = lower(email);\nDELETE FROM audit_log;\n",
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    let hits = findings
        .iter()
        .filter(|finding| {
            finding
                .evidence
                .iter()
                .any(|e| e.contains("sql.migration.full-table-write"))
        })
        .count();
    assert_eq!(hits, 2, "expected UPDATE and DELETE findings: {findings:?}");
}

#[test]
fn blocking_maintenance_without_window_fires() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_maintenance.sql"),
        "VACUUM FULL accounts;\nREINDEX TABLE accounts;\n",
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_detector(&findings, "sql.migration.blocking-maintenance-op");
}

#[test]
fn sqlite_unsafe_pragmas_and_rebuild_without_checks_fire() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(
        &repo.path().join("db/migrations/001_sqlite.sql"),
        r#"
PRAGMA foreign_keys = OFF;
CREATE TABLE new_users(id INTEGER PRIMARY KEY);
INSERT INTO new_users SELECT id FROM users;
DROP TABLE users;
ALTER TABLE new_users RENAME TO users;
"#,
    );

    let findings = findings_for(repo.path(), "HLT-030-SQL-BAD-BEHAVIOR");
    assert_detector(&findings, "sql.migration.sqlite-unsafe-pragma");
    assert_detector(&findings, "sql.migration.sqlite-rebuild-no-check");
}

#[test]
fn repo_without_sql_or_database_setup_has_no_migration_findings_or_cap() {
    let repo = tempdir().unwrap();
    write_minimal_standard_repo(repo.path());
    write(&repo.path().join("src/main.rs"), "fn main() {}\n");
    write(
        &repo.path().join("app.ts"),
        "export const value: number = 1;\n",
    );

    let report = run_audit(repo.path(), &[]).unwrap();
    assert!(
        !report
            .findings
            .iter()
            .any(
                |finding| finding.rule_id.as_deref() == Some("HLT-021-DESTRUCTIVE-MIGRATION")
                    || finding
                        .evidence
                        .iter()
                        .any(|e| e.contains("sql.migration."))
            ),
        "unexpected migration findings: {:?}",
        report.findings
    );
    assert!(
        !report
            .caps_applied
            .iter()
            .any(|cap| cap == "destructive-migration-risk"),
        "unexpected migration cap: {:?}",
        report.caps_applied
    );
}
