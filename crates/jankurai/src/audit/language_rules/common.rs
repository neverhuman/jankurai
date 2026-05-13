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

fn earliest_python_triple_quote(line: &str) -> Option<(&'static str, usize)> {
    let double = line.find("\"\"\"");
    let single = line.find("'''");
    match (double, single) {
        (Some(d), Some(s)) if d <= s => Some(("\"\"\"", d)),
        (Some(_d), Some(s)) => Some(("'''", s)),
        (Some(d), None) => Some(("\"\"\"", d)),
        (None, Some(s)) => Some(("'''", s)),
        (None, None) => None,
    }
}

fn strip_python_string_literals(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_single {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        if in_double {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_double = false;
            }
            continue;
        }

        if ch == '\'' {
            if matches!(chars.peek(), Some(&'\'')) {
                let mut preview = chars.clone();
                preview.next();
                if matches!(preview.next(), Some('\'')) {
                    break;
                }
            }
            in_single = true;
            continue;
        }
        if ch == '"' {
            if matches!(chars.peek(), Some(&'"')) {
                let mut preview = chars.clone();
                preview.next();
                if matches!(preview.next(), Some('"')) {
                    break;
                }
            }
            in_double = true;
            continue;
        }

        out.push(ch);
    }

    out
}

/// Return Python source lines with comments and docstrings stripped so line-based
/// detectors only examine executable code.
pub fn python_code_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_docstring: Option<&'static str> = None;

    for (idx, raw_line) in text.lines().enumerate() {
        let mut line = raw_line.trim();
        if line.is_empty() {
            out.push((idx + 1, String::new()));
            continue;
        }

        if let Some(delim) = in_docstring {
            if let Some(end) = line.find(delim) {
                line = &line[end + delim.len()..];
                in_docstring = None;
            } else {
                out.push((idx + 1, String::new()));
                continue;
            }
        }

        let mut code = line.trim().to_string();
        if code.is_empty() {
            out.push((idx + 1, String::new()));
            continue;
        }

        if let Some((delim, start)) = earliest_python_triple_quote(code.as_str()) {
            let prefix = code[..start].trim_end();
            let suffix = &code[start + delim.len()..];
            let closing = suffix.find(delim);
            let mut combined = String::new();
            if !prefix.is_empty() {
                combined.push_str(prefix);
            }
            if let Some(end) = closing {
                let tail = suffix[end + delim.len()..].trim();
                if !tail.is_empty() {
                    if !combined.is_empty() {
                        combined.push(' ');
                    }
                    combined.push_str(tail);
                }
            } else {
                in_docstring = Some(delim);
            }
            code = combined.trim().to_string();
        }

        if code.is_empty() {
            out.push((idx + 1, String::new()));
            continue;
        }

        let code = code
            .split_once('#')
            .map(|(left, _)| left)
            .unwrap_or(code.as_str())
            .trim();
        if code.is_empty() {
            out.push((idx + 1, String::new()));
            continue;
        }

        out.push((idx + 1, code.trim().to_string()));
    }

    out
}

/// True when the line contains a direct builtin call like `eval(`, `exec(`, or
/// `compile(`, but not a qualified method call such as `model.eval()` or
/// `re.compile(...)`.
pub fn contains_unqualified_python_builtin_call(line: &str, builtin: &str) -> bool {
    let stripped = strip_python_string_literals(line);
    let trimmed = stripped.trim_start();
    if trimmed.is_empty()
        || trimmed.starts_with("def ")
        || trimmed.starts_with("async def ")
        || trimmed.starts_with("class ")
    {
        return false;
    }

    let needle = builtin.to_ascii_lowercase();
    let hay = trimmed.to_ascii_lowercase();
    let mut offset = 0usize;
    while let Some(pos) = hay[offset..].find(&needle) {
        let idx = offset + pos;
        let before = hay[..idx].chars().rev().find(|c| !c.is_whitespace());
        if matches!(before, Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '.') {
            offset = idx + needle.len();
            continue;
        }
        let mut after = hay[idx + needle.len()..]
            .chars()
            .skip_while(|c| c.is_whitespace());
        if matches!(after.next(), Some('(')) {
            return true;
        }
        offset = idx + needle.len();
    }

    false
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

// SQL-specific constructor used by both sql.rs and sql_migration.rs, which share the same
// proof-window/snippet evidence shape but differ only in their HLT_RULE_ID constant.
#[allow(clippy::too_many_arguments)]
pub(super) fn sql_finding(
    rule_id: &'static str,
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
    let snippet = line.trim().chars().take(160).collect::<String>();
    LanguageFinding::new(
        rule_id,
        matched_term,
        file.rel_path.clone(),
        Some(line_no),
        snippet.clone(),
        problem,
        reason,
        agent_fix,
        vec![
            format!("detector={detector_id}"),
            format!("proof-window={proof_window}"),
            format!("snippet={snippet}"),
        ],
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_code(text: &str) -> Vec<String> {
        python_code_lines(text)
            .into_iter()
            .filter(|(_, line)| !line.is_empty())
            .map(|(_, line)| line)
            .collect()
    }

    #[test]
    fn python_code_lines_strips_hash_comments() {
        let src = "x = 1  # eval(\nresult = x + 2  # exec(\n";
        assert_eq!(collect_code(src), vec!["x = 1", "result = x + 2"]);
    }

    #[test]
    fn python_code_lines_drops_pure_comment_and_blank_lines() {
        let src = "# only a comment\n\n    \nvalue = 3\n";
        let kept: Vec<(usize, String)> = python_code_lines(src)
            .into_iter()
            .filter(|(_, line)| !line.is_empty())
            .collect();
        assert_eq!(kept, vec![(4_usize, "value = 3".to_string())]);
    }

    #[test]
    fn python_code_lines_skips_triple_quoted_docstring() {
        let src = "def f():\n    \"\"\"docstring eval( inside\n    multiple lines\n    still doc\"\"\"\n    return 1\n";
        assert_eq!(collect_code(src), vec!["def f():", "return 1"]);
    }

    #[test]
    fn python_code_lines_handles_inline_triple_quote() {
        let src = "msg = \"\"\"hello eval(\"\"\"\nrun = True\n";
        let kept = collect_code(src);
        assert!(kept.iter().any(|l| l == "msg ="));
        assert!(kept.contains(&"run = True".to_string()));
    }

    #[test]
    fn python_code_lines_handles_single_triple_quoted_docstring() {
        let src = "def g():\n    '''single triple eval('''\n    return 2\n";
        assert_eq!(collect_code(src), vec!["def g():", "return 2"]);
    }

    #[test]
    fn unqualified_builtin_accepts_direct_call() {
        assert!(contains_unqualified_python_builtin_call(
            "eval(payload)",
            "eval"
        ));
        assert!(contains_unqualified_python_builtin_call(
            "    exec(code)",
            "exec"
        ));
        assert!(contains_unqualified_python_builtin_call(
            "compile(src, '<x>', 'exec')",
            "compile"
        ));
    }

    #[test]
    fn unqualified_builtin_rejects_method_call() {
        assert!(!contains_unqualified_python_builtin_call(
            "model.eval()",
            "eval"
        ));
        assert!(!contains_unqualified_python_builtin_call(
            "re.compile(pattern)",
            "compile"
        ));
        assert!(!contains_unqualified_python_builtin_call(
            "self.exec(query)",
            "exec"
        ));
    }

    #[test]
    fn unqualified_builtin_rejects_calls_inside_strings() {
        assert!(!contains_unqualified_python_builtin_call(
            "msg = \"eval(payload)\"",
            "eval"
        ));
        assert!(!contains_unqualified_python_builtin_call(
            "msg = 'exec(payload)'",
            "exec"
        ));
    }

    #[test]
    fn unqualified_builtin_rejects_definitions() {
        assert!(!contains_unqualified_python_builtin_call(
            "def eval(self, ctx):",
            "eval"
        ));
        assert!(!contains_unqualified_python_builtin_call(
            "async def exec(s):",
            "exec"
        ));
        assert!(!contains_unqualified_python_builtin_call(
            "class Compile(Base):",
            "compile"
        ));
    }

    #[test]
    fn unqualified_builtin_requires_call_parens() {
        assert!(!contains_unqualified_python_builtin_call(
            "if eval in names:",
            "eval"
        ));
        assert!(!contains_unqualified_python_builtin_call(
            "x = eval_handler()",
            "eval"
        ));
        assert!(!contains_unqualified_python_builtin_call(
            "compile_step = 1",
            "compile"
        ));
    }

    #[test]
    fn unqualified_builtin_handles_escaped_quotes() {
        assert!(!contains_unqualified_python_builtin_call(
            "msg = \"escaped \\\" then eval(x)\"",
            "eval"
        ));
        assert!(contains_unqualified_python_builtin_call(
            "label = 'safe'; eval(payload)",
            "eval"
        ));
    }
}
