use super::catalog::{LanguageFinding, ProofWindow};
use crate::model::FileInfo;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeSet;

static ALLOW_EXPIRES_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"expires=\d{4}-\d{2}-\d{2}").expect("allow expiry regex is valid"));

pub fn is_docs_reference_tips_or_generated(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with("docs/")
        || lower.starts_with("paper/")
        || lower.starts_with("reference/")
        || lower.starts_with("tips/")
        || lower.starts_with("generated/")
        || lower.contains("/generated/")
        || lower.starts_with("target/")
}

pub fn is_test_fixture_or_example(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with("tests/")
        || lower.starts_with("test/")
        || lower.contains("/tests/")
        || lower.contains("/test/")
        || lower.starts_with("fixtures/")
        || lower.contains("/fixtures/")
        || lower.starts_with("examples/")
        || lower.contains("/examples/")
        || lower.starts_with("__tests__/")
        || lower.contains("/__tests__/")
        || lower.starts_with("__fixtures__/")
        || lower.contains("/__fixtures__/")
        || lower.ends_with(".spec.ts")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".spec.rs")
        || lower.ends_with(".test.rs")
}

pub fn is_dev_only_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    is_test_fixture_or_example(path)
        || lower.starts_with("sandbox/")
        || lower.contains("/sandbox/")
        || lower.starts_with("dev/")
        || lower.contains("/dev/")
        || lower.starts_with("devtools/")
        || lower.contains("/devtools/")
        || lower.starts_with("playground/")
        || lower.contains("/playground/")
        || lower.starts_with("mocks/")
        || lower.contains("/mocks/")
}

pub fn is_executable_policy_surface(file: &FileInfo) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    lower.starts_with(".github/workflows/")
        || lower.starts_with(".github/actions/")
        || lower.starts_with(".github/hooks/")
        || lower.starts_with(".git/hooks/")
        || lower.starts_with(".agents/")
        || lower.starts_with(".cursor/")
        || lower.starts_with(".claude/")
        || lower.starts_with("hooks/")
        || lower.starts_with("scripts/")
        || lower.starts_with("tools/")
        || lower.starts_with("ci/")
        || lower.ends_with("/justfile")
        || lower.ends_with("justfile")
        || lower.ends_with("/makefile")
        || lower.ends_with("makefile")
        || lower.ends_with("/dockerfile")
        || lower.starts_with("dockerfile")
        || lower.ends_with(".dockerfile")
        || lower.ends_with("docker-compose.yml")
        || lower.ends_with("docker-compose.yaml")
        || lower.ends_with("compose.yml")
        || lower.ends_with("compose.yaml")
        || lower.ends_with(".gitmodules")
        || lower.ends_with("package.json")
        || lower.ends_with(".sh")
        || lower.ends_with(".ps1")
        || lower.ends_with(".bat")
}

fn nearby_lines(text: &str, line: usize, radius: usize) -> Vec<&str> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return vec![];
    }
    let idx = line.saturating_sub(1).min(lines.len() - 1);
    let start = idx.saturating_sub(radius);
    let end = (idx + radius + 1).min(lines.len());
    lines[start..end].to_vec()
}

pub fn nearby_allow(text: &str, line: usize, detector_id: &str) -> bool {
    let detector = detector_id.to_ascii_lowercase();
    nearby_lines(text, line, 2).iter().any(|candidate| {
        let lower = candidate.trim().to_ascii_lowercase();
        lower.contains("jankurai:allow")
            && lower.contains(&detector)
            && lower.contains("reason=")
            && ALLOW_EXPIRES_RE.is_match(&lower)
    })
}

pub fn nearby_proof(text: &str, line: usize, keywords: &[&str]) -> bool {
    if keywords.is_empty() {
        return false;
    }
    let lowered: Vec<String> = keywords.iter().map(|k| k.to_ascii_lowercase()).collect();
    nearby_lines(text, line, 3).iter().any(|candidate| {
        let lower = candidate.trim().to_ascii_lowercase();
        lowered.iter().any(|keyword| lower.contains(keyword))
    })
}

pub fn strip_comments_for_line_language(line: &str, kind: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let lower = kind.to_ascii_lowercase();
    let stripped = match lower.as_str() {
        "sql" => trimmed
            .split_once("--")
            .map(|(left, _)| left)
            .unwrap_or(trimmed),
        "yaml" | "yml" | "toml" | "shell" | "sh" => trimmed
            .split_once('#')
            .map(|(left, _)| left)
            .unwrap_or(trimmed),
        "ts" | "tsx" | "js" | "jsx" | "rs" | "py" | "ci" | "git" | "source" => trimmed
            .split_once("//")
            .map(|(left, _)| left)
            .or_else(|| trimmed.split_once('#').map(|(left, _)| left))
            .unwrap_or(trimmed),
        _ => trimmed,
    };
    stripped.trim().to_string()
}

pub fn contains_secret_name(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "private_key",
        "private key",
        "aws_",
        "database_url",
        "db_url",
        "connection_string",
        "client_secret",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

// Shared language-rule constructor keeps detector call sites explicit and fixture-readable.
#[allow(clippy::too_many_arguments)]
pub fn finding(
    rule_id: &'static str,
    detector_id: &'static str,
    file: &FileInfo,
    line: usize,
    problem: impl Into<String>,
    reason: impl Into<String>,
    fix: impl Into<String>,
    proof_window: ProofWindow,
) -> LanguageFinding {
    let snippet = file
        .text
        .lines()
        .nth(line.saturating_sub(1))
        .map(|l| l.trim().chars().take(160).collect::<String>())
        .unwrap_or_default();
    let mut evidence = vec![
        format!("detector={detector_id}"),
        format!("path={}", file.rel_path),
        format!("line={line}"),
        format!("proof_window={proof_window:?}"),
    ];
    if !snippet.is_empty() {
        evidence.push(format!("snippet={snippet}"));
    }
    LanguageFinding::new(
        rule_id,
        detector_id,
        file.rel_path.clone(),
        Some(line),
        snippet,
        problem,
        reason,
        fix,
        evidence,
    )
}

pub fn sort_and_cap_findings(
    mut findings: Vec<LanguageFinding>,
    max: usize,
) -> Vec<LanguageFinding> {
    findings.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.unwrap_or(0).cmp(&b.line.unwrap_or(0)))
            .then(a.rule_id.cmp(b.rule_id))
            .then(a.matched_term.cmp(b.matched_term))
    });
    let mut seen = BTreeSet::new();
    findings
        .into_iter()
        .filter(|finding| {
            let key = (
                finding.rule_id.to_string(),
                finding.path.clone(),
                finding.line.unwrap_or(0),
                finding.matched_term.to_string(),
            );
            seen.insert(key)
        })
        .take(max)
        .collect()
}
