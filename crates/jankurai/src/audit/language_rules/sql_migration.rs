use super::catalog::{
    ConfidencePolicy, Language, LanguageFinding, LanguageRule, Matcher, ProofWindow,
};
use crate::audit::helpers::AuditContext;
use crate::model::FileInfo;
use std::path::{Path, PathBuf};

const HLT_RULE_ID: &str = "HLT-030-SQL-BAD-BEHAVIOR";
pub const DETECTOR_DESTRUCTIVE_MIGRATION: &str = "sql.migration.destructive-no-proof";
pub const DETECTOR_CONCURRENT_IN_TXN: &str = "sql.migration.concurrent-in-txn";
pub const DETECTOR_CASCADE_CONVENIENCE: &str = "sql.migration.cascade-convenience";
pub const DETECTOR_MISSING_LOCK_TIMEOUT: &str = "sql.migration.missing-lock-timeout";
pub const DETECTOR_MISSING_STATEMENT_TIMEOUT: &str = "sql.migration.missing-statement-timeout";
pub const DETECTOR_FULL_TABLE_WRITE: &str = "sql.migration.full-table-write";
pub const DETECTOR_BLOCKING_MAINTENANCE_OP: &str = "sql.migration.blocking-maintenance-op";
pub const DETECTOR_SQLITE_UNSAFE_PRAGMA: &str = "sql.migration.sqlite-unsafe-pragma";
pub const DETECTOR_SQLITE_REBUILD_NO_CHECK: &str = "sql.migration.sqlite-rebuild-no-check";
pub const DETECTOR_NON_CONCURRENT_INDEX: &str = "sql.migration.non-concurrent-index";
pub const DETECTOR_NOT_VALID_UNVALIDATED: &str = "sql.migration.not-valid-unvalidated";

pub const RULES: &[LanguageRule] = &[
    LanguageRule {
        id: DETECTOR_DESTRUCTIVE_MIGRATION,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "db",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "drop table",
            "truncate",
            "drop column",
            "drop constraint",
            "drop schema",
            "drop database",
            "disable trigger",
        ]),
        proof_window: ProofWindow::None,
        problem: "destructive migration appears without structured safety evidence",
        fix: "add structured migration metadata plus verify evidence, or split the change into a reviewed staged migration",
    },
    LanguageRule {
        id: DETECTOR_CONCURRENT_IN_TXN,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "db",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["create index concurrently"]),
        proof_window: ProofWindow::None,
        problem: "CREATE INDEX CONCURRENTLY appears inside an explicit transaction block",
        fix: "run CONCURRENTLY outside BEGIN/COMMIT or configure the migration runner as non-transactional",
    },
    LanguageRule {
        id: DETECTOR_CASCADE_CONVENIENCE,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "db",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["cascade"]),
        proof_window: ProofWindow::None,
        problem: "DROP/TRUNCATE CASCADE lacks structured dependency inventory",
        fix: "inventory dependent objects in migration metadata or remove CASCADE and handle dependencies explicitly",
    },
    LanguageRule {
        id: DETECTOR_MISSING_LOCK_TIMEOUT,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "db",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["alter table"]),
        proof_window: ProofWindow::None,
        problem: "risky PostgreSQL DDL lacks lock_timeout evidence",
        fix: "set lock_timeout in SQL or declare it in structured migration metadata",
    },
    LanguageRule {
        id: DETECTOR_MISSING_STATEMENT_TIMEOUT,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "db",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["alter table"]),
        proof_window: ProofWindow::None,
        problem: "risky PostgreSQL DDL lacks statement_timeout evidence",
        fix: "set statement_timeout in SQL or declare it in structured migration metadata",
    },
    LanguageRule {
        id: DETECTOR_FULL_TABLE_WRITE,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "db",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["delete from", "update "]),
        proof_window: ProofWindow::None,
        problem: "migration UPDATE or DELETE lacks a WHERE clause",
        fix: "add a WHERE clause, batch the backfill, or attach structured proof for the full-table rewrite",
    },
    LanguageRule {
        id: DETECTOR_BLOCKING_MAINTENANCE_OP,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "db",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["vacuum full", "cluster", "reindex"]),
        proof_window: ProofWindow::None,
        problem: "blocking maintenance operation lacks maintenance-window metadata",
        fix: "move the operation into a declared maintenance window or use a nonblocking alternative",
    },
    LanguageRule {
        id: DETECTOR_SQLITE_UNSAFE_PRAGMA,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "db",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "pragma writable_schema",
            "pragma journal_mode",
            "pragma foreign_keys",
        ]),
        proof_window: ProofWindow::None,
        problem: "SQLite migration disables integrity safeguards without re-check proof",
        fix: "avoid unsafe PRAGMAs, or re-enable constraints and run foreign_key_check plus integrity checks",
    },
    LanguageRule {
        id: DETECTOR_SQLITE_REBUILD_NO_CHECK,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "db",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["alter table", "rename to", "foreign_key_check"]),
        proof_window: ProofWindow::None,
        problem: "SQLite table rebuild lacks foreign-key and integrity checks",
        fix: "run PRAGMA foreign_key_check and PRAGMA quick_check or integrity_check after rebuilding the table",
    },
    LanguageRule {
        id: DETECTOR_NON_CONCURRENT_INDEX,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "advisory",
        lane: "db",
        confidence: ConfidencePolicy::Medium,
        matcher: Matcher::ContainsAny(&["create index"]),
        proof_window: ProofWindow::None,
        problem: "migration creates a non-concurrent index",
        fix: "prefer CREATE INDEX CONCURRENTLY for live PostgreSQL tables, or document the maintenance window",
    },
    LanguageRule {
        id: DETECTOR_NOT_VALID_UNVALIDATED,
        language: Language::Sql,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "advisory",
        lane: "db",
        confidence: ConfidencePolicy::Medium,
        matcher: Matcher::ContainsAny(&["not valid"]),
        proof_window: ProofWindow::None,
        problem: "NOT VALID constraint is not validated in the same migration",
        fix: "add or schedule a VALIDATE CONSTRAINT migration with verification evidence",
    },
];

#[derive(Debug, Clone)]
struct Statement {
    line_no: usize,
    text: String,
}

#[derive(Debug, Clone)]
struct ExecLine {
    line_no: usize,
    text: String,
}

pub fn findings(ctx: &AuditContext, file: &FileInfo) -> Vec<LanguageFinding> {
    if !is_migration_file_path(&file.rel_path) {
        return Vec::new();
    }

    let exec_lines = executable_lines(&file.text);
    let statements = statements(&exec_lines);
    let mut out = Vec::new();

    out.extend(transaction_findings(file, &exec_lines));

    let full_exec = statements
        .iter()
        .map(|s| normalized_sql(&s.text))
        .collect::<Vec<_>>()
        .join("\n");
    let full_lower = full_exec.to_ascii_lowercase();
    let has_lock_timeout =
        full_lower.contains("lock_timeout") || metadata_has(ctx, file, &["lock_timeout"]);
    let has_statement_timeout =
        full_lower.contains("statement_timeout") || metadata_has(ctx, file, &["statement_timeout"]);

    for stmt in &statements {
        let normalized = normalized_sql(&stmt.text);
        let lower = normalized.to_ascii_lowercase();
        if lower.is_empty() {
            continue;
        }

        if is_destructive_statement(&lower) && !destructive_safety_evidence_present(ctx, file) {
            out.push(finding(
                DETECTOR_DESTRUCTIVE_MIGRATION,
                destructive_matched_term(&lower),
                file,
                stmt.line_no,
                &normalized,
                "destructive migration appears without structured safety evidence",
                "comment-only markers are not proof for destructive schema or data removal",
                "add same-stem or same-directory migration metadata plus verify/check evidence",
                "structured-metadata",
            ));
        }

        if is_cascade_statement(&lower) && !dependency_inventory_present(ctx, file) {
            out.push(finding(
                DETECTOR_CASCADE_CONVENIENCE,
                "cascade",
                file,
                stmt.line_no,
                &normalized,
                "DROP/TRUNCATE CASCADE lacks structured dependency inventory",
                "CASCADE can remove dependent views, policies, triggers, constraints, and functions",
                "declare the dependency inventory in migration metadata or remove CASCADE",
                "dependency-inventory",
            ));
        }

        if is_risky_postgres_ddl(&lower) {
            if !has_lock_timeout {
                out.push(finding(
                    DETECTOR_MISSING_LOCK_TIMEOUT,
                    "lock_timeout",
                    file,
                    stmt.line_no,
                    &normalized,
                    "risky PostgreSQL DDL lacks lock_timeout evidence",
                    "a migration waiting on a lock can queue behind traffic and block unrelated queries",
                    "add SET lock_timeout or structured migration metadata with lock_timeout",
                    "timeout-guard",
                ));
            }
            if !has_statement_timeout {
                out.push(finding(
                    DETECTOR_MISSING_STATEMENT_TIMEOUT,
                    "statement_timeout",
                    file,
                    stmt.line_no,
                    &normalized,
                    "risky PostgreSQL DDL lacks statement_timeout evidence",
                    "a long-running DDL statement can hold locks indefinitely without a statement timeout",
                    "add SET statement_timeout or structured migration metadata with statement_timeout",
                    "timeout-guard",
                ));
            }
        }

        if is_full_table_write_statement(&lower) && !destructive_safety_evidence_present(ctx, file)
        {
            out.push(finding(
                DETECTOR_FULL_TABLE_WRITE,
                "update/delete",
                file,
                stmt.line_no,
                &normalized,
                "migration UPDATE or DELETE lacks a WHERE clause",
                "the statement can rewrite or delete every row in the target table",
                "add a WHERE clause, batch the backfill, or attach structured proof for a full-table rewrite",
                "where-clause",
            ));
            continue;
        }

        if is_blocking_maintenance_statement(&lower) && !maintenance_window_present(ctx, file) {
            out.push(finding(
                DETECTOR_BLOCKING_MAINTENANCE_OP,
                "maintenance operation",
                file,
                stmt.line_no,
                &normalized,
                "blocking maintenance operation lacks maintenance-window metadata",
                "VACUUM FULL, CLUSTER, and blocking REINDEX can hold heavy locks on live tables",
                "declare a maintenance window in migration metadata or use a nonblocking alternative",
                "maintenance-window",
            ));
            continue;
        }

        if let Some(term) = unsafe_sqlite_pragma_term(&lower, &full_lower) {
            out.push(finding(
                DETECTOR_SQLITE_UNSAFE_PRAGMA,
                term,
                file,
                stmt.line_no,
                &normalized,
                "SQLite migration disables integrity safeguards without re-check proof",
                "unsafe PRAGMAs can bypass schema or foreign-key integrity during migration",
                "avoid unsafe PRAGMAs, or re-enable constraints and run foreign_key_check plus integrity checks",
                "sqlite-integrity-check",
            ));
        }
    }

    if is_sqlite_rebuild_without_checks(&full_lower) {
        let line_no = statements.first().map(|s| s.line_no).unwrap_or(1);
        out.push(finding(
            DETECTOR_SQLITE_REBUILD_NO_CHECK,
            "sqlite rebuild",
            file,
            line_no,
            "SQLite table rebuild",
            "SQLite table rebuild lacks foreign-key and integrity checks",
            "table-copy rebuilds can orphan rows or corrupt constraints unless SQLite checks run after rename",
            "run PRAGMA foreign_key_check and PRAGMA quick_check or integrity_check after the rebuild",
            "sqlite-integrity-check",
        ));
    }

    out
}

pub fn advisory_findings(_ctx: &AuditContext, file: &FileInfo) -> Vec<LanguageFinding> {
    if !is_migration_file_path(&file.rel_path) {
        return Vec::new();
    }
    let statements = statements(&executable_lines(&file.text));
    let mut out = Vec::new();
    let full_lower = statements
        .iter()
        .map(|s| normalized_sql(&s.text))
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    for stmt in statements {
        let normalized = normalized_sql(&stmt.text);
        let lower = normalized.to_ascii_lowercase();
        if is_non_concurrent_index_statement(&lower) {
            out.push(finding(
                DETECTOR_NON_CONCURRENT_INDEX,
                "create index",
                file,
                stmt.line_no,
                &normalized,
                "migration creates a non-concurrent index",
                "non-concurrent PostgreSQL index builds can block writes on live tables",
                "prefer CREATE INDEX CONCURRENTLY for live PostgreSQL tables, or document the maintenance window",
                "soft-recommendation",
            ));
        }
        if lower.contains(" not valid") && !full_lower.contains("validate constraint") {
            out.push(finding(
                DETECTOR_NOT_VALID_UNVALIDATED,
                "not valid",
                file,
                stmt.line_no,
                &normalized,
                "NOT VALID constraint is not validated in the same migration",
                "NOT VALID skips existing rows until a later VALIDATE CONSTRAINT step runs",
                "add or schedule a VALIDATE CONSTRAINT migration with verification evidence",
                "soft-recommendation",
            ));
        }
    }
    out
}

pub fn is_migration_file_path(rel_path: &str) -> bool {
    let p = rel_path.to_ascii_lowercase();
    p.starts_with("db/")
        || p.contains("/db/migrations/")
        || p.contains("/db/constraints/")
        || p.starts_with("migrations/")
        || p.starts_with("apps/api/migrations/")
        || matches_monorepo_migration_segment(&p)
}

pub fn destructive_safety_evidence_present(ctx: &AuditContext, file: &FileInfo) -> bool {
    let metadata = migration_metadata(ctx, file);
    if metadata.is_empty() {
        return false;
    }
    metadata.iter().any(|meta| {
        let lower = meta.text.to_ascii_lowercase();
        has_owner_or_approval(&lower)
            && has_rollback_or_roll_forward(&lower)
            && has_backup_restore_or_irreversible_approval(&lower)
            && has_lock_timeout_posture(&lower)
            && has_verify_or_check_evidence(ctx, file, &lower)
    })
}

pub fn dependency_inventory_present(ctx: &AuditContext, file: &FileInfo) -> bool {
    migration_metadata(ctx, file).iter().any(|meta| {
        let lower = meta.text.to_ascii_lowercase();
        (lower.contains("dependency_inventory")
            || lower.contains("dependencies")
            || lower.contains("dependent_objects")
            || lower.contains("cascade_scope"))
            && (lower.contains("owner") || lower.contains("approved") || lower.contains("approval"))
    })
}

fn metadata_has(ctx: &AuditContext, file: &FileInfo, needles: &[&str]) -> bool {
    migration_metadata(ctx, file).iter().any(|meta| {
        let lower = meta.text.to_ascii_lowercase();
        needles.iter().all(|needle| lower.contains(needle))
    })
}

fn maintenance_window_present(ctx: &AuditContext, file: &FileInfo) -> bool {
    migration_metadata(ctx, file).iter().any(|meta| {
        let lower = meta.text.to_ascii_lowercase();
        lower.contains("maintenance_window") || lower.contains("maintenance window")
    })
}

fn migration_metadata<'a>(ctx: &'a AuditContext, file: &FileInfo) -> Vec<&'a FileInfo> {
    let candidates = metadata_candidate_paths(&file.rel_path);
    ctx.all_files
        .iter()
        .filter(|candidate| candidates.iter().any(|path| path == &candidate.rel_path))
        .collect()
}

fn metadata_candidate_paths(rel_path: &str) -> Vec<String> {
    let path = Path::new(rel_path);
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let mut out = Vec::new();
    if !stem.is_empty() {
        out.push(parent.join(format!("{stem}.meta.toml")));
        out.push(parent.join(format!("{stem}.jankurai.toml")));
    }
    if !file_name.is_empty() {
        out.push(parent.join(format!("{file_name}.meta.toml")));
    }
    out.push(parent.join("meta.toml"));
    out.push(parent.join("migration.toml"));
    out.push(parent.join(".meta.toml"));
    out.push(parent.join(".jankurai.toml"));
    out.into_iter().map(pathbuf_to_rel).collect()
}

fn verify_candidate_paths(rel_path: &str) -> Vec<String> {
    let path = Path::new(rel_path);
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let mut out = Vec::new();
    if !stem.is_empty() {
        out.push(parent.join(format!("{stem}.verify.sql")));
        out.push(parent.join(format!("{stem}.check.sql")));
        out.push(parent.join(format!("{stem}_verify.sql")));
    }
    out.into_iter().map(pathbuf_to_rel).collect()
}

fn pathbuf_to_rel(path: PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn has_owner_or_approval(lower: &str) -> bool {
    lower.contains("owner")
        && (lower.contains("approval") || lower.contains("approved") || lower.contains("approver"))
}

fn has_rollback_or_roll_forward(lower: &str) -> bool {
    lower.contains("rollback")
        || lower.contains("roll_forward")
        || lower.contains("roll-forward")
        || lower.contains("roll forward")
        || lower.contains("fix_forward")
        || lower.contains("fix-forward")
}

fn has_backup_restore_or_irreversible_approval(lower: &str) -> bool {
    lower.contains("backup")
        || lower.contains("restore")
        || (lower.contains("irreversible")
            && (lower.contains("approval") || lower.contains("approved")))
}

fn has_lock_timeout_posture(lower: &str) -> bool {
    (lower.contains("lock_timeout") && lower.contains("statement_timeout"))
        || (lower.contains("lock") && lower.contains("timeout"))
}

fn has_verify_or_check_evidence(ctx: &AuditContext, file: &FileInfo, metadata_lower: &str) -> bool {
    if metadata_lower.contains("verify")
        || metadata_lower.contains("verification")
        || metadata_lower.contains("check_artifact")
        || metadata_lower.contains("post_check")
        || metadata_lower.contains("pre_check")
    {
        return true;
    }
    let candidates = verify_candidate_paths(&file.rel_path);
    ctx.all_files
        .iter()
        .any(|candidate| candidates.iter().any(|path| path == &candidate.rel_path))
}

fn executable_lines(text: &str) -> Vec<ExecLine> {
    let mut lines = Vec::new();
    let mut in_block_comment = false;
    let mut in_dollar_quote = false;
    for (idx, raw) in text.lines().enumerate() {
        let mut line = raw;
        let dollar_count = line.matches("$$").count();
        if in_dollar_quote {
            if dollar_count % 2 == 1 {
                in_dollar_quote = false;
            }
            continue;
        }
        if dollar_count % 2 == 1 {
            line = line.split("$$").next().unwrap_or("");
            in_dollar_quote = true;
        }
        let mut cleaned = String::new();
        let mut rest = line;
        loop {
            if in_block_comment {
                if let Some(end) = rest.find("*/") {
                    rest = &rest[end + 2..];
                    in_block_comment = false;
                } else {
                    break;
                }
            }
            if let Some(start) = rest.find("/*") {
                cleaned.push_str(&rest[..start]);
                rest = &rest[start + 2..];
                in_block_comment = true;
                continue;
            }
            cleaned.push_str(rest);
            break;
        }
        let executable = cleaned
            .split_once("--")
            .map(|(before, _)| before)
            .unwrap_or(cleaned.as_str())
            .trim();
        if !executable.is_empty() {
            lines.push(ExecLine {
                line_no: idx + 1,
                text: executable.to_string(),
            });
        }
    }
    lines
}

fn statements(lines: &[ExecLine]) -> Vec<Statement> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut line_no = 1;
    for line in lines {
        if current.is_empty() {
            line_no = line.line_no;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&line.text);
        if line.text.contains(';') {
            out.push(Statement {
                line_no,
                text: current.clone(),
            });
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        out.push(Statement {
            line_no,
            text: current,
        });
    }
    out
}

fn transaction_findings(file: &FileInfo, lines: &[ExecLine]) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    let mut in_txn = false;
    for line in lines {
        let normalized = normalized_sql(&line.text);
        let lower = normalized.to_ascii_lowercase();
        if is_transaction_begin(&lower) {
            in_txn = true;
            continue;
        }
        if is_transaction_end(&lower) {
            in_txn = false;
            continue;
        }
        if in_txn && lower.contains("create index concurrently") {
            out.push(finding(
                DETECTOR_CONCURRENT_IN_TXN,
                "create index concurrently",
                file,
                line.line_no,
                &normalized,
                "CREATE INDEX CONCURRENTLY appears inside an explicit transaction block",
                "PostgreSQL forbids CONCURRENTLY inside BEGIN/COMMIT transaction blocks",
                "run CONCURRENTLY outside BEGIN/COMMIT or configure the migration runner as non-transactional",
                "transaction-boundary",
            ));
        }
    }
    out
}

fn normalized_sql(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(';')
        .trim()
        .to_string()
}

fn is_transaction_begin(lower: &str) -> bool {
    matches!(
        lower.trim_end_matches(';').trim(),
        "begin" | "begin transaction" | "start transaction"
    )
}

fn is_transaction_end(lower: &str) -> bool {
    matches!(lower.trim_end_matches(';').trim(), "commit" | "rollback")
}

fn is_destructive_statement(lower: &str) -> bool {
    lower.contains("drop table")
        || lower.contains("drop database")
        || lower.contains("drop schema")
        || lower.contains("truncate table")
        || lower.contains("truncate ")
        || lower.contains("drop column")
        || lower.contains("drop index")
        || lower.contains("drop constraint")
        || lower.contains("disable trigger")
}

fn destructive_matched_term(lower: &str) -> &'static str {
    if lower.contains("truncate") {
        "truncate"
    } else if lower.contains("drop column") {
        "drop column"
    } else if lower.contains("drop index") {
        "drop index"
    } else if lower.contains("drop constraint") {
        "drop constraint"
    } else if lower.contains("drop schema") {
        "drop schema"
    } else if lower.contains("drop database") {
        "drop database"
    } else {
        "drop table"
    }
}

fn is_cascade_statement(lower: &str) -> bool {
    lower.contains(" cascade")
        && (lower.starts_with("drop ")
            || lower.starts_with("truncate ")
            || lower.contains(" drop ")
            || lower.contains(" truncate "))
}

fn is_risky_postgres_ddl(lower: &str) -> bool {
    lower.contains("alter table")
        && (lower.contains(" drop ")
            || lower.contains(" add constraint")
            || lower.contains(" add column")
            || lower.contains(" alter column")
            || lower.contains(" set not null"))
}

fn is_full_table_write_statement(lower: &str) -> bool {
    let padded = format!(" {lower} ");
    (lower.starts_with("update ") && padded.contains(" set ") && !padded.contains(" where "))
        || (lower.starts_with("delete from ") && !padded.contains(" where "))
}

fn is_blocking_maintenance_statement(lower: &str) -> bool {
    lower.starts_with("vacuum full")
        || lower.starts_with("cluster ")
        || (lower.starts_with("reindex ") && !lower.contains(" concurrently"))
}

fn unsafe_sqlite_pragma_term(lower: &str, full_lower: &str) -> Option<&'static str> {
    let compact = lower.replace(' ', "");
    if compact.contains("pragmawritable_schema=on") {
        return Some("pragma writable_schema");
    }
    if compact.contains("pragmajournal_mode=off") {
        return Some("pragma journal_mode");
    }
    if compact.contains("pragmaforeign_keys=off")
        && !(full_lower.contains("pragma foreign_keys = on")
            && full_lower.contains("pragma foreign_key_check"))
    {
        return Some("pragma foreign_keys");
    }
    None
}

fn is_sqlite_rebuild_without_checks(full_lower: &str) -> bool {
    let looks_like_rebuild = (full_lower.contains("create table new_")
        || full_lower.contains("create table temp_")
        || full_lower.contains("_new"))
        && full_lower.contains("insert into")
        && full_lower.contains("select")
        && full_lower.contains("drop table")
        && full_lower.contains("rename to");
    looks_like_rebuild
        && !(full_lower.contains("foreign_key_check")
            && (full_lower.contains("quick_check") || full_lower.contains("integrity_check")))
}

fn is_non_concurrent_index_statement(lower: &str) -> bool {
    lower.starts_with("create index") && !lower.contains(" concurrently")
}

fn matches_monorepo_migration_segment(rel_path: &str) -> bool {
    if let Some(stripped) = rel_path
        .strip_prefix("packages/")
        .or_else(|| rel_path.strip_prefix("apps/"))
    {
        let mut parts = stripped.splitn(3, '/');
        let _name = parts.next();
        if let Some(segment) = parts.next() {
            if segment == "migration" || segment == "migrations" {
                return true;
            }
        }
    }
    rel_path.contains("/db/migrations/")
}

#[allow(clippy::too_many_arguments)]
fn finding(
    detector_id: &'static str,
    matched_term: &'static str,
    file: &FileInfo,
    line_no: usize,
    line: &str,
    problem: &str,
    reason: &str,
    agent_fix: &str,
    proof_window: &'static str,
) -> LanguageFinding {
    super::common::sql_finding(
        HLT_RULE_ID,
        detector_id,
        matched_term,
        file,
        line_no,
        line,
        problem,
        reason,
        agent_fix,
        proof_window,
    )
}
