use crate::audit::helpers::AuditContext;
use crate::audit::language_rules::common::{
    finding, is_docs_reference_tips_or_generated, is_test_fixture_or_example, nearby_allow,
    sort_and_cap_findings, strip_comments_for_line_language,
};
use crate::audit::language_rules::{LanguageFinding, ProofWindow};
use crate::model::FileInfo;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeSet;

const HLT_RULE_ID: &str = "HLT-040-REPO-ROT-BAD-BEHAVIOR";

static FAKE_VERSION_SUFFIX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:^|[_\-.])(old|backup|bak|copy|final|v[2-9])(?:[_\-.]|$)")
        .expect("repo-rot fake version regex is valid")
});

static HARD_DISABLED_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^\s*(?:if\s+false\b|if\s*\(\s*(?:false|0)\s*\)|while\s+false\b|#\s*if\s+0\b|#\s*\[cfg\(\s*false\s*\)\])")
        .expect("hard-disabled code regex is valid")
});

#[derive(Debug, Clone, Copy, Default)]
pub struct RepoRotSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn summary(ctx: &AuditContext) -> RepoRotSummary {
    RepoRotSummary {
        hard_findings: hard_findings(ctx).len(),
        advisory_signals: advisory_hits(ctx).len(),
    }
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = hard_findings(ctx);
    out.extend(advisory_hits(ctx));
    sort_and_cap_findings(out, 60)
}

pub fn advisory_signals(ctx: &AuditContext) -> Vec<LanguageFinding> {
    sort_and_cap_findings(advisory_hits(ctx), 50)
}

fn hard_findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in ctx.all_files.iter().filter(|file| !excluded(file)) {
        if active_path(file) && !valid_version_or_migration_path(&file.rel_path) {
            out.extend(path_rot_hits(file));
        }
    }
    out
}

fn advisory_hits(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in ctx.all_files.iter().filter(|file| !excluded(file)) {
        if active_path(file) && !valid_version_or_migration_path(&file.rel_path) {
            out.extend(archive_snapshot_hits(file));
            if is_code_like(file) {
                out.extend(commented_code_block_hits(file));
                out.extend(hard_disabled_code_hits(file));
            }
        }
    }
    out
}

fn excluded(file: &FileInfo) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    is_docs_reference_tips_or_generated(&file.rel_path)
        || is_test_fixture_or_example(&file.rel_path)
        || lower.contains("/fixtures/")
        || matches!(
            lower.as_str(),
            "changelog.md" | "release_notes.md" | "release-notes.md" | "version" | "cargo.lock"
        )
}

fn active_path(file: &FileInfo) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    lower.starts_with("src/")
        || lower.starts_with("apps/")
        || lower.starts_with("crates/")
        || lower.starts_with("packages/")
        || lower.starts_with("db/")
        || lower.starts_with("ops/")
        || lower.starts_with("tools/")
        || lower.starts_with("scripts/")
        || lower.starts_with(".github/workflows/")
        || lower.starts_with("contracts/")
}

fn valid_version_or_migration_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    if lower.starts_with("db/migrations/") || lower.contains("/migrations/") {
        return true;
    }
    if lower.starts_with("contracts/") && has_version_segment(&lower) {
        return true;
    }
    if (lower.starts_with("api/") || lower.contains("/api/")) && has_version_segment(&lower) {
        return true;
    }
    false
}

fn has_version_segment(path: &str) -> bool {
    path.split('/').any(|segment| {
        segment.len() >= 2
            && segment.starts_with('v')
            && segment[1..].chars().all(|ch| ch.is_ascii_digit())
    })
}

fn path_rot_hits(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    if nearby_allow(&file.text, 1, "repo-rot.path.fake-versioned-source") {
        return out;
    }
    let lower = file.rel_path.to_ascii_lowercase();
    let segments = lower.split('/').collect::<Vec<_>>();
    let exact_rot_segment = segments.iter().any(|segment| {
        matches!(
            *segment,
            "archive"
                | "archives"
                | "archived"
                | "attic"
                | "graveyard"
                | "old"
                | "older"
                | "old2"
                | "backup"
                | "backups"
                | "bak"
                | "bkp"
                | "copy"
                | "copy_of_copy"
                | "unused"
                | "dead"
                | "deprecated"
                | "obsolete"
                | "disabled"
                | "do-not-use"
                | "do_not_use"
                | "scratch"
                | "tmp"
                | "temp"
                | "prototype"
                | "spike"
        )
    });
    let file_stem = lower
        .rsplit('/')
        .next()
        .and_then(|name| name.split('.').next())
        .unwrap_or(lower.as_str());
    let fake_versioned_file = FAKE_VERSION_SUFFIX_RE.is_match(file_stem)
        || file_stem.contains("copy-of")
        || file_stem.contains("final-final");
    if exact_rot_segment || fake_versioned_file {
        out.push(finding(
            HLT_RULE_ID,
            "repo-rot.path.fake-versioned-source",
            file,
            1,
            "active source path looks like an old, backup, copied, or parked implementation",
            "ambiguous old-looking active source makes agents and reviewers guess whether code is live",
            "delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan",
            ProofWindow::None,
        ));
    }
    out
}

fn archive_snapshot_hits(file: &FileInfo) -> Vec<LanguageFinding> {
    let lower = file.rel_path.to_ascii_lowercase();
    let archive_ext = [
        ".zip",
        ".tar",
        ".tar.gz",
        ".tgz",
        ".rar",
        ".7z",
        ".bak",
        ".orig",
        ".rej",
        ".tmp",
        ".sql.dump",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix));
    if !archive_ext || nearby_allow(&file.text, 1, "repo-rot.archive.source-snapshot") {
        return vec![];
    }
    let suspicious_name = ["source", "src", "project", "backup", "dump", "prod", "old"]
        .iter()
        .any(|needle| lower.contains(needle));
    if suspicious_name || file.size > 250_000 {
        vec![finding(
            HLT_RULE_ID,
            "repo-rot.archive.source-snapshot",
            file,
            1,
            "active tree contains an archive or backup artifact that looks like a source snapshot",
            "checked-in snapshots bypass normal source control review and can preserve stale code or secrets",
            "remove the snapshot from active source or move it to a documented artifact/archive system with ownership and retention policy",
            ProofWindow::None,
        )]
    } else {
        vec![]
    }
}

fn commented_code_block_hits(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    let mut start_line = 0usize;
    let mut block = Vec::new();
    for (idx, line) in file.text.lines().enumerate() {
        let line_no = idx + 1;
        if let Some(comment) = comment_content(line) {
            if looks_like_dead_commented_code(comment) {
                if block.is_empty() {
                    start_line = line_no;
                }
                block.push(comment.to_string());
                continue;
            }
        }
        flush_comment_block(file, start_line, &block, &mut out);
        block.clear();
    }
    flush_comment_block(file, start_line, &block, &mut out);
    out
}

fn flush_comment_block(
    file: &FileInfo,
    start_line: usize,
    block: &[String],
    out: &mut Vec<LanguageFinding>,
) {
    if block.len() < 5 || nearby_allow(&file.text, start_line, "repo-rot.comment.dead-code-block") {
        return;
    }
    let joined = block.join("\n").to_ascii_lowercase();
    let stale_marker = [
        "old",
        "backup",
        "temporary",
        "remove later",
        "do not delete",
        "deprecated",
    ]
    .iter()
    .any(|needle| joined.contains(needle));
    if stale_marker {
        out.push(finding(
            HLT_RULE_ID,
            "repo-rot.comment.dead-code-block",
            file,
            start_line,
            "active source contains a large commented-out stale code block",
            "commented code keeps dead behavior in the agent context without executable proof",
            "delete the block or move the rationale to a dated owner/issue record outside active source",
            ProofWindow::None,
        ));
    }
}

fn hard_disabled_code_hits(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for (idx, raw_line) in file.text.lines().enumerate() {
        let line_no = idx + 1;
        if nearby_allow(&file.text, line_no, "repo-rot.unreachable.hard-disabled") {
            continue;
        }
        let stripped = strip_comments_for_line_language(raw_line, "source");
        if stripped.trim_start().starts_with('"') || stripped.trim_start().starts_with('\'') {
            continue;
        }
        if HARD_DISABLED_RE.is_match(&stripped) {
            out.push(finding(
                HLT_RULE_ID,
                "repo-rot.unreachable.hard-disabled",
                file,
                line_no,
                "active source contains a hard-disabled branch",
                "unreachable code paths keep old behavior around without proof or deletion plan",
                "remove the branch or replace it with a tracked feature flag that has owner, expiry, and cleanup proof",
                ProofWindow::None,
            ));
        }
    }
    out
}

fn is_code_like(file: &FileInfo) -> bool {
    file.is_code
        || matches!(
            file.suffix.as_str(),
            ".rs"
                | ".ts"
                | ".tsx"
                | ".js"
                | ".jsx"
                | ".py"
                | ".go"
                | ".java"
                | ".kt"
                | ".swift"
                | ".c"
                | ".cc"
                | ".cpp"
                | ".h"
                | ".hpp"
                | ".sql"
                | ".sh"
                | ".yaml"
                | ".yml"
        )
}

fn comment_content(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix("//")
        .or_else(|| trimmed.strip_prefix('#'))
        .or_else(|| trimmed.strip_prefix("--"))
        .or_else(|| trimmed.strip_prefix('*'))
        .map(str::trim)
}

fn looks_like_dead_commented_code(comment: &str) -> bool {
    let lower = comment.to_ascii_lowercase();
    let code_like = [
        "function ",
        "class ",
        "const ",
        "let ",
        "var ",
        "if ",
        "return ",
        "select ",
        "<div",
        "useeffect",
        "fn ",
        "pub ",
        "async ",
        "await ",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    let stale = [
        "old",
        "backup",
        "temporary",
        "remove later",
        "do not delete",
        "deprecated",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    code_like || stale
}

#[allow(dead_code)]
fn unique_findings(mut findings: Vec<LanguageFinding>) -> Vec<LanguageFinding> {
    let mut seen = BTreeSet::new();
    findings.retain(|finding| {
        seen.insert((
            finding.rule_id,
            finding.path.clone(),
            finding.line.unwrap_or_default(),
            finding.matched_term,
        ))
    });
    findings
}
