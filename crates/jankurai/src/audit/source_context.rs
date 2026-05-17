use crate::model::FileInfo;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct ScanLine {
    pub line_no: usize,
    pub raw: String,
    pub active_code: String,
    pub comment_only: bool,
    pub test_scaffold: bool,
}

enum LineStyle {
    CLike,
    Python,
    Hash,
    Sql,
    Lua,
}

fn line_style(file: &FileInfo) -> LineStyle {
    match file.suffix.as_str() {
        ".py" => LineStyle::Python,
        ".sql" => LineStyle::Sql,
        ".sh" | ".bash" | ".zsh" | ".fish" | ".ps1" | ".yml" | ".yaml" | ".toml" | ".rb" => {
            LineStyle::Hash
        }
        ".lua" => LineStyle::Lua,
        _ => LineStyle::CLike,
    }
}

pub fn source_lines(file: &FileInfo) -> Vec<ScanLine> {
    let style = line_style(file);
    let mut out = Vec::new();
    let mut in_block_comment = false;
    let mut in_python_docstring: Option<&'static str> = None;

    for (idx, raw) in file.text.lines().enumerate() {
        let active_code =
            strip_comments(raw, &style, &mut in_block_comment, &mut in_python_docstring);
        out.push(ScanLine {
            line_no: idx + 1,
            raw: raw.to_string(),
            comment_only: !raw.trim().is_empty() && active_code.trim().is_empty(),
            active_code,
            test_scaffold: false,
        });
    }

    if file.suffix == ".rs" {
        mark_rust_test_scaffolding(&mut out);
    }

    out
}

fn strip_comments(
    raw: &str,
    style: &LineStyle,
    in_block_comment: &mut bool,
    in_python_docstring: &mut Option<&'static str>,
) -> String {
    let trimmed = raw.trim_start();
    if matches!(style, LineStyle::Python) {
        if let Some(delim) = *in_python_docstring {
            if let Some(end) = raw.find(delim) {
                let tail = &raw[end + delim.len()..];
                *in_python_docstring = None;
                return tail.trim_start().to_string();
            }
            return String::new();
        }
        if let Some(delim) = python_docstring_start(trimmed) {
            *in_python_docstring = Some(delim);
            if trimmed.ends_with(delim) && trimmed.len() > delim.len() {
                *in_python_docstring = None;
            }
            return String::new();
        }
    }

    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if *in_block_comment {
            if ch == '*' && matches!(chars.peek(), Some('/')) {
                chars.next();
                *in_block_comment = false;
            }
            continue;
        }

        if in_single {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        if in_double {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }
        if in_backtick {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '`' {
                in_backtick = false;
            }
            continue;
        }

        match style {
            LineStyle::CLike => {
                if ch == '/' && matches!(chars.peek(), Some('/')) {
                    break;
                }
                if ch == '/' && matches!(chars.peek(), Some('*')) {
                    chars.next();
                    *in_block_comment = true;
                    continue;
                }
            }
            LineStyle::Hash | LineStyle::Python => {
                if ch == '#' {
                    break;
                }
            }
            LineStyle::Sql | LineStyle::Lua => {
                if ch == '-' && matches!(chars.peek(), Some('-')) {
                    break;
                }
            }
        }

        match ch {
            '\'' => {
                in_single = true;
                out.push(ch);
            }
            '"' => {
                in_double = true;
                out.push(ch);
            }
            '`' => {
                if matches!(style, LineStyle::CLike) {
                    in_backtick = true;
                }
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }

    out
}

fn python_docstring_start(trimmed: &str) -> Option<&'static str> {
    let stripped = trimmed
        .strip_prefix("r\"\"\"")
        .or_else(|| trimmed.strip_prefix("u\"\"\""))
        .or_else(|| trimmed.strip_prefix("f\"\"\""))
        .or_else(|| trimmed.strip_prefix("R\"\"\""))
        .or_else(|| trimmed.strip_prefix("U\"\"\""))
        .or_else(|| trimmed.strip_prefix("F\"\"\""))
        .or_else(|| trimmed.strip_prefix("r'''"))
        .or_else(|| trimmed.strip_prefix("u'''"))
        .or_else(|| trimmed.strip_prefix("f'''"))
        .or_else(|| trimmed.strip_prefix("R'''"))
        .or_else(|| trimmed.strip_prefix("U'''"))
        .or_else(|| trimmed.strip_prefix("F'''"))
        .or_else(|| trimmed.strip_prefix("\"\"\""))
        .or_else(|| trimmed.strip_prefix("'''"));
    stripped.and_then(|tail| {
        if tail.is_empty()
            || tail
                .chars()
                .next()
                .map(|c| c.is_whitespace())
                .unwrap_or(false)
        {
            if trimmed.contains("\"\"\"") {
                Some("\"\"\"")
            } else {
                Some("'''")
            }
        } else {
            None
        }
    })
}

fn brace_balance(line: &str) -> i32 {
    let mut balance = 0;
    for ch in line.chars() {
        match ch {
            '{' => balance += 1,
            '}' => balance -= 1,
            _ => {}
        }
    }
    balance
}

fn mark_rust_test_scaffolding(lines: &mut [ScanLine]) {
    let mut in_cfg_tests = false;
    let mut cfg_depth = 0i32;
    let mut pending_cfg_tests = false;
    let mut pending_test_attr = false;
    let mut in_test_fn = false;
    let mut fn_depth = 0i32;

    for line in lines.iter_mut() {
        let code = line.active_code.trim();
        let lower = code.to_ascii_lowercase();

        if in_cfg_tests {
            line.test_scaffold = true;
            cfg_depth += brace_balance(code);
            if cfg_depth <= 0 {
                in_cfg_tests = false;
                cfg_depth = 0;
            }
            continue;
        }
        if in_test_fn {
            line.test_scaffold = true;
            fn_depth += brace_balance(code);
            if fn_depth <= 0 {
                in_test_fn = false;
                fn_depth = 0;
            }
            continue;
        }

        if lower.contains("#[cfg(test)]") {
            line.test_scaffold = true;
            if lower.contains("mod tests") {
                in_cfg_tests = true;
                cfg_depth = brace_balance(code);
                if cfg_depth <= 0 {
                    in_cfg_tests = false;
                }
            } else {
                pending_cfg_tests = true;
            }
            continue;
        }

        if pending_cfg_tests {
            line.test_scaffold = true;
            if lower.contains("mod tests") && code.contains('{') {
                in_cfg_tests = true;
                cfg_depth = brace_balance(code);
                if cfg_depth <= 0 {
                    in_cfg_tests = false;
                }
                pending_cfg_tests = false;
            }
            continue;
        }

        if pending_test_attr {
            line.test_scaffold = true;
            if lower.contains("fn ") && code.contains('{') {
                in_test_fn = true;
                fn_depth = brace_balance(code);
                if fn_depth <= 0 {
                    in_test_fn = false;
                }
                pending_test_attr = false;
            }
            continue;
        }

        if lower.contains("#[test]")
            || lower.contains("#[tokio::test]")
            || lower.contains("#[rstest]")
            || lower.contains("#[async_std::test]")
        {
            line.test_scaffold = true;
            pending_test_attr = true;
            continue;
        }
    }
}

pub fn term_only_appears_in_local_binding(active_code: &str, term: &str) -> bool {
    let lower = active_code.to_ascii_lowercase();
    let term = term.to_ascii_lowercase();
    if lower.is_empty() || term.is_empty() {
        return false;
    }
    if lower.contains('"') || lower.contains('\'') || lower.contains('`') {
        return false;
    }
    if lower.contains("pub ")
        || lower.starts_with("pub ")
        || lower.contains("export ")
        || lower.starts_with("export ")
        || lower.contains("public ")
    {
        return false;
    }
    let binding_prefixes = [
        "let ",
        "const ",
        "var ",
        "fn ",
        "def ",
        "function ",
        "static ",
        "mutable ",
    ];
    if binding_prefixes.iter().any(|prefix| {
        lower
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
            .starts_with(prefix)
            && lower.contains(&format!("{prefix}{term}"))
    }) {
        return true;
    }
    if (lower.contains(&format!(": {term}")) || lower.contains(&format!(":{term}")))
        && (lower.contains('{')
            || lower.contains('(')
            || lower.contains(',')
            || lower.contains("=>"))
    {
        return true;
    }
    false
}

pub fn looks_like_scan_example(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let has_example_command = lower.contains("grep")
        || lower.contains("rg ")
        || lower.contains("ripgrep")
        || lower.contains("regex")
        || lower.contains("search for")
        || lower.contains("find");
    let has_pattern_only_tokens = ["sk-|sk_|hf_|AIza|gsk_", "ghp_", "xox", "eyj"]
        .iter()
        .any(|needle| lower.contains(needle));
    has_example_command && has_pattern_only_tokens
}

static SECRET_LITERAL_REGEXES: Lazy<Vec<(&'static str, Regex)>> = Lazy::new(|| {
    vec![
        (
            "aws-access-key",
            Regex::new(r"(?i)\bAKIA[0-9A-Z]{16}\b").expect("valid regex"),
        ),
        (
            "github-token",
            Regex::new(r"(?i)\bgh[pousr]_[A-Za-z0-9_]{20,}\b").expect("valid regex"),
        ),
        (
            "openai-key",
            Regex::new(r"(?i)\bsk-(?:proj|test)?-[A-Za-z0-9_-]{16,}\b").expect("valid regex"),
        ),
        (
            "slack-token",
            Regex::new(r"(?i)\bxox[baprs]-[A-Za-z0-9-]{10,}\b").expect("valid regex"),
        ),
        (
            "jwt",
            Regex::new(r"(?i)\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b")
                .expect("valid regex"),
        ),
        (
            "pem-private-key",
            Regex::new(r"(?i)-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----").expect("valid regex"),
        ),
        (
            "google-api-key",
            Regex::new(r"(?i)\bAIza[0-9A-Za-z\-_]{20,}\b").expect("valid regex"),
        ),
        (
            "groq-key",
            Regex::new(r"(?i)\bgsk_[A-Za-z0-9]{20,}\b").expect("valid regex"),
        ),
        (
            "huggingface-key",
            Regex::new(r"(?i)\bhf_[A-Za-z0-9]{20,}\b").expect("valid regex"),
        ),
    ]
});

pub fn extract_high_confidence_secret_literal(text: &str) -> Option<String> {
    if looks_like_scan_example(text) {
        return None;
    }
    SECRET_LITERAL_REGEXES.iter().find_map(|(label, regex)| {
        regex
            .find(text)
            .map(|mat| format!("{label}:{}", mat.as_str()))
    })
}
