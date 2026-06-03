use super::helpers::*;
use super::language_rules;
use super::language_rules::common::{contains_unqualified_python_builtin_call, python_code_lines};
use super::prose;
use super::source_context;
use crate::model::FileInfo;
use crate::validation;
use once_cell::sync::Lazy;
use regex::Regex;

/// HLT-001 placeholder/TODO patterns. Multi-word phrases stay as substrings (already
/// word-bounded). Short bare words like `placeholder` were too broad and matched
/// legitimate identifiers (`argumentSlots`, `placeholder` field name, etc.); we now
/// only flag the actual hostile shapes (`// placeholder`, `placeholder!()`, …).
pub const TODO_PATTERNS: &[&str] = &[
    "TODO",
    "FIXME",
    "HACK",
    "XXX",
    "stub",
    "// placeholder",
    "# placeholder",
    "placeholder!(",
    "<placeholder>",
    "not implemented",
    "todo!(",
    "unimplemented!(",
    "panic!(\"todo",
    "panic!(\"not implemented",
];

/// HLT-001 fallback/retry patterns. Bare `retry` was a substring of legitimate
/// fields like `retry_after_seconds`, so it has been replaced with hostile-only
/// phrases (`silent retry`, `unbounded retry`, `retry forever`).
pub const FALLBACK_PATTERNS: &[&str] = &[
    "best effort",
    "silent retry",
    "unbounded retry",
    "retry forever",
    "except Exception",
    "except:",
    "return null",
    "return undefined",
];

pub const PROMPT_PATTERNS: &[&str] = &[
    "ignore previous instructions",
    "ignore prior instructions",
    "reveal the secret",
    "reveal the token",
    "bypass policy",
    "trust user input",
    "execute untrusted",
    "run whatever the issue",
    "trust the issue",
    "trust issue text",
    "ignore rules",
    "ignore constraints",
];

pub const AGENCY_PATTERNS: &[&str] = &[
    "danger-full-access",
    "approval_policy: never",
    "sandbox_mode: danger-full-access",
    "allow all tools",
    "unrestricted terminal",
    "unrestricted browser",
    "unrestricted filesystem",
];

pub const FALSE_GREEN_PATTERNS: &[&str] = &[
    "test.skip(",
    "it.skip(",
    "describe.skip(",
    "pytest.mark.skip",
    "pytest.mark.skipif(",
    "pytest.skip(",
    "unittest.skip(",
    "self.skipTest(",
    ".only(",
    "xtest(",
    "xit(",
    "expect(true).toBe(true)",
    "assert true",
    "toMatchSnapshot(",
    "toMatchInlineSnapshot(",
];

pub const STREAMING_CLIENT_PATTERNS: &[&str] = &[
    "rdkafka",
    "kafka",
    "kafka-node",
    "kafkajs",
    "tansu",
    "apache_iggy",
    "iggy",
    "fluvio",
    "nats",
    "redis::streams",
    "xadd",
    "xreadgroup",
];

pub const AUTHZ_ISOLATION_PATTERNS: &[&str] = &[
    "owner_id",
    "tenant_id",
    "organization_id",
    "org_id",
    "rls",
    "row level security",
    "admin",
];

pub const INPUT_BOUNDARY_PATTERNS: &[&str] = &[
    "eval(",
    "exec(",
    "child_process",
    "Command::new",
    "shell=True",
    "innerHTML",
    "dangerouslySetInnerHTML",
    "SELECT * FROM",
    "fetch(",
];

pub fn language_bad_behavior_hits(
    ctx: &AuditContext,
) -> Vec<super::language_rules::LanguageFinding> {
    language_rules::findings(ctx)
}

pub fn is_test_or_example_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with("tests/")
        || lower.contains("/tests/")
        || lower.starts_with("examples/")
        || lower.contains("/examples/")
        || lower.contains("/example/")
        || lower.contains("/spec/")
        // End-to-end test trees (Playwright/Cypress convention): e2e specs +
        // their fixtures/page-objects are TEST code, not product code.
        || lower.starts_with("e2e/")
        || lower.contains("/e2e/")
        || lower.ends_with("_test.rs")
        || lower.ends_with(".test.rs")
        || lower.ends_with(".spec.rs")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".spec.ts")
        || lower.ends_with("_tests.rs")
        || lower.ends_with("/tests.rs")
        || lower == "tests.rs"
}

pub fn is_generated_or_reference_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    if lower.starts_with("docs/")
        || lower.starts_with("paper/")
        || lower.starts_with("reference/")
        || lower.starts_with("tips/")
        || lower.starts_with("generated/")
        || lower.contains("/generated/")
        || lower.starts_with("target/")
    {
        return true;
    }
    // Suffix-based skips for files that are auto-generated regardless of location.
    // `*.gen.{ts,tsx,js,mjs}` come from codegen tools, and `sst-env.d.ts` is produced
    // by SST and lives next to handwritten code.
    if lower.ends_with(".gen.ts")
        || lower.ends_with(".gen.tsx")
        || lower.ends_with(".gen.js")
        || lower.ends_with(".gen.mjs")
    {
        return true;
    }
    let basename = lower.rsplit('/').next().unwrap_or(lower.as_str());
    basename == "sst-env.d.ts"
}

pub fn line_has_nearby_safety_comment(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return false;
    }
    let idx = line.saturating_sub(1);
    let start = idx.saturating_sub(3);
    let end = (idx + 2).min(lines.len());
    lines[start..end].iter().any(|candidate| {
        let trimmed = candidate.trim().to_ascii_lowercase();
        trimmed.contains("safety:")
            || trimmed.starts_with("// safety")
            || trimmed.starts_with("/// safety")
            || trimmed.starts_with("/* safety")
            || trimmed.starts_with("// safety:")
    })
}

pub fn public_unsafe_has_safety_docs(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return false;
    }
    let idx = line.saturating_sub(1);
    let start = idx.saturating_sub(12);
    let mut saw_doc = false;
    for candidate in lines[start..idx].iter().rev() {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            if saw_doc {
                break;
            }
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("pub ") || lower.starts_with("fn ") || lower.starts_with("impl ") {
            break;
        }
        if lower.starts_with("///")
            || lower.starts_with("//!")
            || lower.starts_with("/**")
            || lower.starts_with("/*")
            || lower.starts_with("*")
        {
            saw_doc = true;
            if lower.contains("# safety") {
                return true;
            }
        } else if saw_doc {
            break;
        }
    }
    false
}

pub fn function_context_contains_async(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return false;
    }
    let idx = line.saturating_sub(1);
    let start = idx.saturating_sub(32);
    lines[start..idx].iter().rev().any(|candidate| {
        let trimmed = candidate.trim().to_ascii_lowercase();
        trimmed.contains("async fn")
            || (trimmed.contains("async") && trimmed.contains("fn"))
            || trimmed.contains("async move")
            || trimmed.contains("tokio::main")
            || trimmed.contains("tokio::test")
    })
}

pub fn is_fixed_safe_command_invocation(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let Some(start) = lower.find("command::new(") else {
        return false;
    };
    let tail = &lower[start + "command::new(".len()..];
    let quote = tail.chars().find(|c| *c == '"' || *c == '\'');
    let Some(quote) = quote else {
        return false;
    };
    let quote_idx = tail.find(quote).unwrap_or(0);
    let tail = &tail[quote_idx + 1..];
    let end = tail.find(quote).unwrap_or(0);
    if end == 0 {
        return false;
    }
    let executable = &tail[..end];
    let fixed = [
        "git",
        "cargo",
        "rustc",
        "node",
        "python",
        "python3",
        "npm",
        "pnpm",
        "yarn",
        "make",
        "just",
        "bash",
        "sh",
        "zsh",
        "fish",
        "cmd",
        "cmd.exe",
        "powershell",
        "pwsh",
    ];
    let looks_fixed = fixed.contains(&executable);
    let looks_shell = matches!(
        executable,
        "bash" | "sh" | "zsh" | "fish" | "cmd" | "cmd.exe" | "powershell" | "pwsh"
    );
    let has_shell_eval = lower.contains(".arg(\"-c\")")
        || lower.contains(".args([\"-c\"")
        || lower.contains(".args(&[\"-c\"")
        || lower.contains("shell=true")
        || lower.contains("shell = true");
    looks_fixed && !looks_shell && !has_shell_eval
}

fn is_import_only_line(lower: &str) -> bool {
    let trimmed = lower.trim();
    (trimmed.starts_with("import ") || trimmed.starts_with("const ") || trimmed.starts_with("let "))
        && (trimmed.contains(" from ") || trimmed.contains("require("))
        && !trimmed.contains(".exec(")
        && !trimmed.contains(".spawn(")
        && !trimmed.contains(".execfile(")
}

pub const AGENT_TOOL_SUPPLY_PATTERNS: &[&str] = &[
    "mcp",
    "modelcontextprotocol",
    "tool server",
    "extension",
    "hooks",
    "auto-run",
    "agent rules",
];

pub const RELEASE_READINESS_PATTERNS: &[&str] = &[
    "launch",
    "production",
    "rollback",
    "backup",
    "restore",
    "rate limit",
    "monitoring",
];

pub const COST_BUDGET_PATTERNS: &[&str] = &[
    "budget",
    "spend cap",
    "cost",
    "quota",
    "token limit",
    "kill switch",
];

pub const HUMAN_REVIEW_EVIDENCE_PATTERNS: &[&str] = &[
    "accept all",
    "lgtm",
    "looks good",
    "fabricated",
    "raw ci logs",
    "review evidence",
];

pub const FUTURE_HOSTILE_TERMS: &[&str] = &[
    "cleanup later",
    "remove later",
    "best effort",
    "dead code",
    "deprecated",
    "depricated",
    "temporary",
    "workaround",
    "backcompat",
    "placeholder",
    "fallback",
    "obsolete",
    "legacy",
    "unused",
    "stale",
    "fixme",
    "dummy",
    "compat",
    "shim",
    "stub",
    "hack",
    "todo",
    "temp",
    "old",
];

pub const FUTURE_HOSTILE_ALLOWLIST_PREFIXES: &[&str] = &["docs/", "reference/", "vendor/"];
pub const FUTURE_HOSTILE_PRODUCT_COPY_PARTS: &[&str] = &[
    "copy-deck",
    "copydeck",
    "i18n",
    "l10n",
    "locale",
    "locales",
    "marketing-copy",
    "messages",
    "product-copy",
    "productcopy",
    "translations",
];

#[derive(Clone, Debug)]
pub struct FindingHit {
    pub path: String,
    pub line: Option<usize>,
    pub text: String,
    pub matched_term: Option<String>,
    pub agent_fix: String,
    pub problem: String,
}

impl FindingHit {
    pub fn new(path: &str, line: usize, text: &str) -> Self {
        Self {
            path: path.into(),
            line: Some(line),
            text: text.into(),
            matched_term: None,
            agent_fix: String::new(),
            problem: text.into(),
        }
    }
}

/// Returns true when a substring pattern is short and bare enough to need a
/// word-boundary check (e.g. `retry` would otherwise match `retry_after_seconds`).
/// Multi-word phrases (`silent retry`, `not implemented`) and shape patterns
/// (`todo!(`, `// placeholder`) are treated as already bounded.
fn pattern_needs_word_boundary(pattern: &str) -> bool {
    if pattern.contains(' ')
        || pattern.contains('(')
        || pattern.contains('/')
        || pattern.contains('#')
        || pattern.contains('<')
        || pattern.contains('"')
    {
        return false;
    }
    // Pure alphanumeric short words: TODO, FIXME, HACK, XXX, stub, fallback.
    pattern
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn pattern_matches_with_boundary(line: &str, pattern: &str) -> Option<usize> {
    if !pattern_needs_word_boundary(pattern) {
        return line.find(pattern);
    }
    let bytes = line.as_bytes();
    let pat_bytes = pattern.as_bytes();
    let mut start = 0;
    while let Some(rel) = line[start..].find(pattern) {
        let abs = start + rel;
        let before = if abs == 0 { None } else { Some(bytes[abs - 1]) };
        let after_idx = abs + pat_bytes.len();
        let after = if after_idx >= bytes.len() {
            None
        } else {
            Some(bytes[after_idx])
        };
        let is_word_byte = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        if before.map(is_word_byte).unwrap_or(false) || after.map(is_word_byte).unwrap_or(false) {
            start = abs + 1;
            continue;
        }
        return Some(abs);
    }
    None
}

/// Like [`pattern_hits`] but suppresses lines with a nearby `jankurai:allow` comment
/// for `detector_id` (when supplied). Pass `None` to keep legacy unfiltered behavior.
pub fn pattern_hits_filtered(
    files: &[FileInfo],
    patterns: &[&str],
    detector_id: Option<&str>,
) -> Vec<FindingHit> {
    if files.is_empty() || patterns.is_empty() {
        return vec![];
    }
    let mut out = vec![];
    for file in files {
        for (idx, line) in file.text.lines().enumerate() {
            let mut matched: Option<&str> = None;
            for pattern in patterns {
                if pattern_matches_with_boundary(line, pattern).is_some() {
                    matched = Some(*pattern);
                    break;
                }
            }
            let Some(matched_pattern) = matched else {
                continue;
            };
            let line_no = idx + 1;
            if let Some(rule) = detector_id {
                if super::language_rules::common::nearby_allow(&file.text, line_no, rule) {
                    continue;
                }
            }
            out.push(FindingHit {
                path: file.rel_path.clone(),
                line: Some(line_no),
                text: line.trim().chars().take(160).collect(),
                matched_term: Some(matched_pattern.to_string()),
                agent_fix: String::new(),
                problem: line.trim().chars().take(160).collect(),
            });
            if out.len() >= 20 {
                return out;
            }
        }
    }
    out
}

pub fn pattern_hits(files: &[FileInfo], patterns: &[&str]) -> Vec<FindingHit> {
    pattern_hits_filtered(files, patterns, None)
}

pub fn todo_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let mut hits = vec![];
    for file in product_code_files(ctx) {
        for line in source_context::source_lines(&file) {
            if line.comment_only || line.test_scaffold {
                continue;
            }
            let active = line.active_code.trim();
            if active.is_empty() {
                continue;
            }
            if let Some(pattern) = TODO_PATTERNS
                .iter()
                .find(|pattern| pattern_matches_with_boundary(active, pattern).is_some())
            {
                if pattern_needs_word_boundary(pattern)
                    && source_context::term_only_appears_in_local_binding(active, pattern)
                {
                    continue;
                }
                hits.push(FindingHit {
                    path: file.rel_path.clone(),
                    line: Some(line.line_no),
                    text: active.chars().take(160).collect(),
                    matched_term: Some((*pattern).to_string()),
                    agent_fix: String::new(),
                    problem: active.chars().take(160).collect(),
                });
                if hits.len() >= 20 {
                    return hits;
                }
            }
        }
    }
    hits
}

pub fn fallback_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let mut hits = vec![];
    for file in product_code_files(ctx) {
        for line in source_context::source_lines(&file) {
            if line.comment_only || line.test_scaffold {
                continue;
            }
            let active = line.active_code.trim();
            if active.is_empty() || line_looks_like_framework_fallback_service(active) {
                continue;
            }
            if line_has_error_hiding_fallback(active) {
                hits.push(FindingHit {
                    path: file.rel_path.clone(),
                    line: Some(line.line_no),
                    text: active.chars().take(160).collect(),
                    matched_term: Some("fallback soup".into()),
                    agent_fix: String::new(),
                    problem: active.chars().take(160).collect(),
                });
                if hits.len() >= 20 {
                    break;
                }
            }
        }
    }
    if hits.len() <= 1 {
        vec![]
    } else {
        hits
    }
}

fn line_looks_like_framework_fallback_service(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("fallback_service(") || lower.contains(".fallback_service(")
}

fn line_has_error_hiding_fallback(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let line_trimmed = lower.trim();

    if lower.contains("silent retry")
        || lower.contains("unbounded retry")
        || lower.contains("retry forever")
        || lower.contains("best effort")
    {
        return true;
    }
    // NOTE: a bare `return null` / `return undefined` is NOT treated as
    // error-hiding. Those literals only occur in TS/JS (Rust uses `None`), where
    // they are idiomatic value returns — a React component rendering nothing
    // (`if (!open) return null`) or a `useEffect` with no cleanup
    // (`return undefined`). Genuine error swallowing is still caught by the
    // explicit signals below (catch all / swallow|ignore|silence error / except:).

    if lower.contains("except:")
        || lower.contains("except exception")
        || lower.contains("catch all")
        || lower.contains("swallow error")
        || lower.contains("ignore error")
        || lower.contains("silence error")
    {
        return true;
    }

    // `ok_or_else(|| Err)` converts `Option` -> `Result`, PRODUCING a typed
    // error — the opposite of hiding one. Exclude it before the `or_else(`
    // substring check below would otherwise match it.
    if lower.contains("ok_or_else") || lower.contains("ok_or(") {
        return false;
    }

    if lower.contains(".ok().unwrap_or")
        || lower.contains("unwrap_or_default(")
        || lower.contains("unwrap_or_else(")
        || lower.contains("or_else(")
    {
        if is_deterministic_path_normalization(&lower) {
            return false;
        }
        if lower.contains("unwrap_or(candidate)") {
            return false;
        }
        // A closure that `panic!`s on the error SURFACES it loudly — that is the
        // opposite of error-hiding, so it is not fallback-soup.
        if lower.contains("panic!") {
            return false;
        }
        // Reading an OPTIONAL environment override and falling back to a default
        // (`env::var("X").unwrap_or_else(|_| default)`) is idiomatic config: the
        // Err means "unset", which is the expected path, not a swallowed failure.
        if lower.contains("env::var") {
            return false;
        }
        // Only a genuinely FALLIBLE source (Result/parse/IO/decode/query/etc.)
        // can hide an error here. A bare `unwrap_or_default()` /
        // `unwrap_or_else(|| ...)` on an infallible `Option` — e.g.
        // `map.get(k).cloned().unwrap_or_default()` (missing optional field ->
        // type default) — is idiomatic Rust, not error swallowing. The method
        // name merely containing "default"/"none" is not a signal, so require a
        // real fallible-source marker before flagging.
        return has_fallible_source_marker(&lower);
    }

    FALLBACK_PATTERNS
        .iter()
        .any(|pattern| pattern_matches_with_boundary(line_trimmed, pattern).is_some())
        && (has_fallible_source_marker(&lower)
            || lower.contains("fallback:")
            || lower.contains("error")
            || lower.contains("retry"))
}

fn has_fallible_source_marker(lower: &str) -> bool {
    [
        "read_to_string",
        "read_dir",
        "read(",
        "open(",
        "parse(",
        "from_str",
        "from_slice",
        "deserialize",
        "json",
        "env::var",
        "try_from",
        "load(",
        "lookup(",
        "fetch(",
        // Call form only: a bare `request` matches common variable/field names
        // like `request.default_branch`, which are not fallible sources.
        "request(",
        ".request(",
        "connect(",
        "recv(",
        "send(",
        "query(",
        "run(",
        "execute(",
        "decode(",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_deterministic_path_normalization(lower: &str) -> bool {
    [
        "file_name",
        "basename",
        "stem",
        "extension",
        "parent",
        "strip_prefix",
        "strip_suffix",
        "split_once",
        "rsplit_once",
        "components",
        "canonicalize",
        "normalize",
        "to_str",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

/// Matches an `api_key:` style assignment. Capture group 1 holds the right-hand-side
/// value so [`secret_assignment_value_is_secret_like`] can decide whether the value is
/// a real literal credential or a bare identifier path (e.g. `model.api_key`,
/// `config.token`) that should not flag.
static SECRET_ASSIGNMENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)\b(?:api[_-]?key|access[_-]?token|client[_-]?secret|private[_-]?key|password|secret)\b\s*[:=]\s*(.+)$"#,
    )
    .expect("secret regex is valid")
});

/// Returns true when an assignment RHS looks like a real literal credential.
///
/// Rejects bare identifier paths like `model.api_key` or `config.token`.
pub fn secret_assignment_value_is_secret_like(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    if source_context::extract_high_confidence_secret_literal(trimmed).is_some() {
        return true;
    }
    let first = trimmed.as_bytes()[0];
    if first == b'"' || first == b'\'' || first == b'`' {
        let close = first;
        let body = &trimmed[1..];
        let end = body
            .as_bytes()
            .iter()
            .position(|&b| b == close)
            .unwrap_or(body.len());
        let content = body[..end].trim();
        return content.len() >= 10;
    }
    false
}

pub fn secret_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let mut hits = vec![];
    for file in &ctx.all_files {
        if file.is_generated
            || is_tracked_auditor_score_artifact(&file.rel_path)
            || file.rel_path.starts_with("crates/jankurai/")
            || file.rel_path.starts_with("docs/")
            || file.rel_path.starts_with("paper/")
            || file.rel_path.starts_with("reference/")
            || file.rel_path.starts_with("tips/")
        {
            continue;
        }
        for line in source_context::source_lines(file) {
            let raw = line.raw.trim();
            let active = line.active_code.trim();
            let literal_match = source_context::extract_high_confidence_secret_literal(raw);
            let assignment_match = SECRET_ASSIGNMENT.captures(active).and_then(|caps| {
                caps.get(1).and_then(|rhs| {
                    if secret_assignment_value_is_secret_like(rhs.as_str()) {
                        Some(())
                    } else {
                        None
                    }
                })
            });
            if literal_match.is_some() || assignment_match.is_some() {
                if super::language_rules::common::nearby_allow(
                    &file.text,
                    line.line_no,
                    "HLT-010-SECRET-SPRAWL",
                ) {
                    continue;
                }
                let problem = literal_match.unwrap_or_else(|| active.chars().take(160).collect());
                hits.push(FindingHit {
                    path: file.rel_path.clone(),
                    line: Some(line.line_no),
                    text: problem.clone(),
                    matched_term: Some("secret-like material".into()),
                    agent_fix: "remove the credential, rotate the secret, and replace it with a config reference or test fixture that cannot be used as a live credential".into(),
                    problem,
                });
                if hits.len() >= 20 {
                    return hits;
                }
            }
        }
    }
    hits
}

fn is_tracked_auditor_score_artifact(path: &str) -> bool {
    path == ".jankurai/repo-score.json"
        || path == ".jankurai/repo-score.md"
        || path == "agent/repo-score.json"
        || path == "agent/repo-score.md"
        || (path.starts_with("agent/baselines/") && path.ends_with(".repo-score.json"))
}

pub fn prompt_injection_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    pattern_hits_filtered(
        &ctx.all_files
            .iter()
            .filter(|f| !f.is_generated && prose::is_trusted_policy_path(&f.rel_path))
            .cloned()
            .collect::<Vec<_>>(),
        PROMPT_PATTERNS,
        Some("HLT-011-PROMPT-INJECTION"),
    )
}

pub fn agency_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    pattern_hits_filtered(
        &ctx.all_files
            .iter()
            .filter(|f| !f.is_generated && prose::is_trusted_policy_path(&f.rel_path))
            .cloned()
            .collect::<Vec<_>>(),
        AGENCY_PATTERNS,
        Some("HLT-012-OVERBROAD-AGENCY"),
    )
}

pub fn false_green_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    pattern_hits_filtered(
        &ctx.all_files
            .iter()
            .filter(|f| {
                !f.is_generated
                    && prose::allows_word_scan(f)
                    && (f.rel_path.contains("/test")
                        || f.rel_path.contains("/spec")
                        || f.name.ends_with(".test.ts")
                        || f.name.ends_with(".spec.ts")
                        || f.name.ends_with("_test.rs")
                        || f.name.ends_with("_test.go"))
            })
            .cloned()
            .collect::<Vec<_>>(),
        FALSE_GREEN_PATTERNS,
        Some("HLT-008-FALSE-GREEN-RISK"),
    )
}

pub fn authz_isolation_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let product = product_code_files(ctx);
    let authz_surface = product.iter().find_map(|file| {
        file.text.lines().enumerate().find_map(|(idx, line)| {
            let lower = line.to_ascii_lowercase();
            AUTHZ_ISOLATION_PATTERNS
                .iter()
                .find(|pattern| lower.contains(**pattern))
                .map(|pattern| {
                    (
                        file,
                        idx + 1,
                        line.trim().to_string(),
                        (*pattern).to_string(),
                    )
                })
        })
    });
    let Some((file, line, text, matched_term)) = authz_surface else {
        return vec![];
    };
    let has_negative_tests = ctx.all_files.iter().any(|file| {
        let lower = file.text.to_ascii_lowercase();
        is_test_file(file)
            && (lower.contains("wrong user")
                || lower.contains("other user")
                || lower.contains("non-owner")
                || lower.contains("non owner")
                || lower.contains("forbidden")
                || lower.contains("tenant isolation")
                || lower.contains("owner/non-owner")
                || lower.contains("rls"))
    });
    if has_negative_tests {
        vec![]
    } else {
        vec![FindingHit {
            path: file.rel_path.clone(),
            line: Some(line),
            text,
            matched_term: Some(matched_term),
            agent_fix: "add owner/non-owner authorization tests or RLS evidence for the touched data boundary".into(),
            problem: "authorization or data-isolation surface lacks direct negative proof".into(),
        }]
    }
}

pub fn input_boundary_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let mut hits = Vec::new();
    for file in product_code_files(ctx) {
        let lines = if file.suffix == ".py" {
            python_code_lines(&file.text)
        } else {
            file.text
                .lines()
                .enumerate()
                .map(|(idx, line)| (idx + 1, line.to_string()))
                .collect::<Vec<_>>()
        };
        for (idx, line) in lines {
            if super::language_rules::common::nearby_allow(
                &file.text,
                idx,
                "HLT-023-INPUT-BOUNDARY-GAP",
            ) {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            let matched = if contains_unqualified_python_builtin_call(&line, "eval") {
                Some("eval(")
            } else if contains_unqualified_python_builtin_call(&line, "exec")
                || (lower.contains("child_process") && !is_import_only_line(&lower))
                || lower.contains("shell=true")
            {
                Some("shell execution")
            } else if lower.contains("command::new") {
                if file.suffix == ".rs" || is_fixed_safe_command_invocation(&line) {
                    None
                } else if lower.contains(".arg(\"-c\")")
                    || lower.contains(".args([\"-c\"")
                    || lower.contains(".args(&[\"-c\"")
                    || lower.contains("command::new(\"bash\"")
                    || lower.contains("command::new(\"sh\"")
                    || lower.contains("command::new(\"zsh\"")
                    || lower.contains("command::new(\"fish\"")
                    || lower.contains("command::new(\"cmd\"")
                    || lower.contains("command::new(\"powershell\"")
                {
                    Some("shell execution")
                } else {
                    None
                }
            } else if lower.contains("dangerouslysetinnerhtml") || lower.contains("innerhtml") {
                Some("unsafe html")
            } else if (lower.contains("select ") || lower.contains("select * from"))
                && (lower.contains("format!(")
                    || lower.contains("+")
                    || lower.contains("${")
                    || lower.contains("concat"))
            {
                Some("string sql")
            } else if lower.contains("fetch(")
                && (lower.contains("req.query")
                    || lower.contains("request.query")
                    || lower.contains("user_url")
                    || lower.contains("userurl")
                    || lower.contains("url ="))
            {
                Some("ssrf fetch")
            } else if (lower.contains("upload") || lower.contains("filename"))
                && (lower.contains("../")
                    || lower.contains("path.join")
                    || lower.contains("originalname"))
            {
                Some("upload traversal")
            } else {
                None
            };
            if let Some(term) = matched {
                if lower.contains("allowlist")
                    || lower.contains("parameterized")
                    || lower.contains("prepared")
                    || lower.contains("sanitize")
                    || lower.contains("safehtml")
                    || lower.contains("safe_url")
                {
                    continue;
                }
                hits.push(FindingHit {
                    path: file.rel_path.clone(),
                    line: Some(idx + 1),
                    text: line.trim().chars().take(160).collect(),
                    matched_term: Some(term.into()),
                    agent_fix: "replace unsafe sinks with typed schemas, parameterized APIs, allowlists, or sandboxed execution plus negative tests".into(),
                    problem: line.trim().chars().take(160).collect(),
                });
            }
        }
    }
    hits
}

pub fn agent_tool_supply_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let files = ctx
        .all_files
        .iter()
        .filter(|file| {
            !file.is_generated
                && prose::is_trusted_policy_path(&file.rel_path)
                && !is_tracked_auditor_score_artifact(&file.rel_path)
        })
        .cloned()
        .collect::<Vec<_>>();
    let risky = [
        "allow = \"all\"",
        "allow = [\"all\"]",
        "permissions = \"all\"",
        "write-all",
        "auto_run = true",
        "auto-run = true",
        "unrestricted",
        "unpinned",
        "latest",
        "curl | sh",
        "network = true",
        "filesystem = true",
        "danger-full-access",
    ];
    let mut hits = Vec::new();
    for file in files {
        let file_has_surface = AGENT_TOOL_SUPPLY_PATTERNS
            .iter()
            .any(|pattern| file.text.to_ascii_lowercase().contains(pattern));
        if !file_has_surface {
            continue;
        }
        for (idx, line) in file.text.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            if lower.contains("hlt-") {
                continue;
            }
            let has_risk = risky.iter().any(|pattern| lower.contains(pattern));
            if has_risk {
                hits.push(FindingHit {
                    path: file.rel_path.clone(),
                    line: Some(idx + 1),
                    text: line.trim().chars().take(160).collect(),
                    matched_term: Some("agent tool supply".into()),
                    agent_fix: "pin and review agent tools, MCP servers, hooks, and rule files; keep untrusted tool output separate from trusted policy".into(),
                    problem: line.trim().chars().take(160).collect(),
                });
            }
        }
    }
    hits
}

pub fn release_readiness_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    if !has_release_surface(ctx) {
        return vec![];
    }

    let missing_structure = missing_release_structure(ctx);
    if !missing_structure.is_empty() {
        return vec![FindingHit {
            path: "docs/release.md".into(),
            line: None,
            text: format!(
                "release structure missing: {}",
                missing_structure.join(", ")
            ),
            matched_term: Some("release structure".into()),
            agent_fix: "add a release control surface with version source, changelog, release process docs, CI or script evidence, integrity/provenance evidence, and rollback guidance".into(),
            problem: "release management structure is incomplete".into(),
        }];
    }

    if !has_release_lane(ctx) {
        return vec![FindingHit {
            path: "docs/testing.md".into(),
            line: None,
            text: "release language found without full launch-gate evidence".into(),
            matched_term: Some("release readiness".into()),
            agent_fix: "add launch-gate evidence for security, backups, monitoring, rollback, and abuse controls".into(),
            problem: "release readiness is claimed without complete proof artifacts".into(),
        }];
    }

    vec![]
}

fn has_release_surface(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|file| {
        if file.is_generated
            || file.rel_path.starts_with("reference/")
            || file.rel_path.starts_with("tips/")
        {
            return false;
        }
        let lower_path = file.rel_path.to_ascii_lowercase();
        matches!(
            lower_path.as_str(),
            "cargo.toml" | "package.json" | "pyproject.toml" | "go.mod"
        ) || lower_path.starts_with(".github/workflows/")
            || lower_path.contains("release")
            || lower_path.contains("publish")
            || (prose::allows_word_scan(file) && {
                let lower = file.text.to_ascii_lowercase();
                lower.contains("gh release")
                    || lower.contains("npm publish")
                    || lower.contains("cargo publish")
                    || lower.contains("docker push")
                    || RELEASE_READINESS_PATTERNS
                        .iter()
                        .any(|pattern| lower.contains(pattern))
            })
    })
}

fn missing_release_structure(ctx: &AuditContext) -> Vec<String> {
    let mut missing = Vec::new();
    if !has_release_version_source(ctx) {
        missing.push("version source".into());
    }
    if !has_release_changelog(ctx) {
        missing.push("changelog".into());
    }
    if !has_release_process_doc(ctx) {
        missing.push("release process doc".into());
    }
    if !has_release_automation_or_policy(ctx) {
        missing.push("release automation or command policy".into());
    }
    if !has_release_integrity_policy(ctx) {
        missing.push("checksum/provenance/SBOM evidence policy".into());
    }
    if !has_release_rollback_policy(ctx) {
        missing.push("rollback guidance".into());
    }
    missing
}

fn has_release_version_source(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|file| {
        let path = file.rel_path.to_ascii_lowercase();
        let lower = file.text.to_ascii_lowercase();
        path == "version"
            || path == "agent/standard-version.toml"
            || path == "cargo.toml" && lower.contains("version")
            || path == "package.json" && lower.contains("\"version\"")
            || path == "pyproject.toml" && lower.contains("version")
    })
}

fn has_release_changelog(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|file| {
        let path = file.rel_path.to_ascii_lowercase();
        path == "changelog.md"
            || path == "changes.md"
            || path == "news.md"
            || path == "history.md"
            || path.ends_with("/changelog.md")
            || path.ends_with("/release-notes.md")
    })
}

fn has_release_process_doc(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|file| {
        let path = file.rel_path.to_ascii_lowercase();
        path == "release.md"
            || path == "docs/release.md"
            || path == "docs/release-plan.md"
            || path == "docs/bad_release.md"
            || path.ends_with("/release.md")
            || path.ends_with("/release-plan.md")
    })
}

fn has_release_automation_or_policy(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|file| {
        let path = file.rel_path.to_ascii_lowercase();
        let lower = file.text.to_ascii_lowercase();
        path.starts_with(".github/workflows/")
            && (prose::allows_word_scan(file) && {
                lower.contains("release")
                    || lower.contains("publish")
                    || lower.contains("jankurai publish")
                    || lower.contains("cargo publish")
                    || lower.contains("npm publish")
            })
            || path.starts_with("scripts/")
                && (path.contains("release") || path.contains("publish"))
            || path == "justfile" && (lower.contains("\nrelease:") || lower.contains("\npublish:"))
            || (prose::allows_word_scan(file) && {
                lower.contains("release gate") || lower.contains("launch gate")
            })
    })
}

fn has_release_integrity_policy(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|file| {
        prose::allows_word_scan(file) && {
            let lower = file.text.to_ascii_lowercase();
            lower.contains("sha256")
                || lower.contains("checksum")
                || lower.contains("sbom")
                || lower.contains("provenance")
                || lower.contains("attestation")
                || lower.contains("slsa")
                || lower.contains("cosign")
        }
    })
}

fn has_release_rollback_policy(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|file| {
        prose::allows_word_scan(file) && file.text.to_ascii_lowercase().contains("rollback")
    })
}

fn has_release_lane(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|file| {
        prose::allows_word_scan(file) && {
            let lower = file.text.to_ascii_lowercase();
            (lower.contains("launch gate") || lower.contains("release gate"))
                && lower.contains("backup")
                && lower.contains("rollback")
                && lower.contains("monitoring")
                && (lower.contains("rate limit") || lower.contains("abuse"))
        }
    })
}

/// Returns true when `agent/audit-policy.toml` declares one or more
/// `[[cost_surface]]` blocks. Repos that explicitly enumerate their cost
/// surfaces are the source of truth for HLT-026; we trust that list and only
/// look for budget proof when it is non-empty.
fn declared_cost_surfaces(ctx: &AuditContext) -> Option<usize> {
    let path = ctx.root.join("agent/audit-policy.toml");
    if !path.exists() {
        return None;
    }
    let text = std::fs::read_to_string(&path).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    let surfaces = value.get("cost_surface")?.as_array()?;
    Some(surfaces.len())
}

pub fn cost_budget_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    // Prefer explicit `[[cost_surface]]` declarations when the policy file
    // enumerates them; fall back to keyword-presence scan otherwise.
    let has_cost_surface = match declared_cost_surfaces(ctx) {
        Some(declared) => declared > 0,
        None => ctx.all_files.iter().any(|file| {
            !file.is_generated && prose::allows_word_scan(file) && {
                let lower = file.text.to_ascii_lowercase();
                lower.contains("openai")
                    || lower.contains("anthropic")
                    || lower.contains("stripe")
                    || lower.contains("paid api")
                    || lower.contains("api bill")
                    || lower.contains("token")
                    || COST_BUDGET_PATTERNS
                        .iter()
                        .any(|pattern| lower.contains(pattern))
            }
        }),
    };
    let has_budget_policy = ctx.all_files.iter().any(|file| {
        prose::allows_word_scan(file) && {
            let lower = file.text.to_ascii_lowercase();
            lower.contains("budget")
                && lower.contains("quota")
                && (lower.contains("spend cap") || lower.contains("kill switch"))
                && lower.contains("stop condition")
        }
    });
    if has_cost_surface && !has_budget_policy {
        vec![FindingHit {
            path: "docs/testing.md".into(),
            line: None,
            text: "cost surface found without budget/stop-condition policy".into(),
            matched_term: Some("budget".into()),
            agent_fix: "add explicit budgets, quotas, stop conditions, and kill-switch evidence for paid or unbounded operations".into(),
            problem: "cost or spend-risk surface lacks budget proof".into(),
        }]
    } else {
        vec![]
    }
}

pub fn human_review_evidence_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let mut hits = Vec::new();
    let risky_claims = [
        "accept all",
        "lgtm",
        "looks good",
        "fabricated",
        "trust me",
        "tests passed (not run)",
        "success without logs",
    ];
    for file in ctx.all_files.iter().filter(|file| {
        !file.is_generated
            && prose::allows_word_scan(file)
            && !file.rel_path.starts_with("reference/")
            && !file.rel_path.starts_with("tips/")
            && !file.rel_path.starts_with("paper/")
            && file.rel_path != "agent/vibe-coverage.toml"
            && !file.rel_path.starts_with("crates/jankurai/")
    }) {
        for (idx, line) in file.text.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            if risky_claims.iter().any(|pattern| lower.contains(pattern)) {
                if super::language_rules::common::nearby_allow(
                    &file.text,
                    idx + 1,
                    "HLT-027-HUMAN-REVIEW-EVIDENCE-GAP",
                ) {
                    continue;
                }
                hits.push(FindingHit {
                    path: file.rel_path.clone(),
                    line: Some(idx + 1),
                    text: line.trim().chars().take(160).collect(),
                    matched_term: Some("review evidence".into()),
                    agent_fix: "attach raw CI logs, review receipts, and replayable commands instead of accepting claims or summaries".into(),
                    problem: line.trim().chars().take(160).collect(),
                });
            }
        }
    }
    hits
}

fn is_test_file(file: &FileInfo) -> bool {
    file.rel_path.contains("/test")
        || file.rel_path.contains("/spec")
        || file.name.ends_with(".test.ts")
        || file.name.ends_with(".spec.ts")
        || file.name.ends_with("_test.rs")
        || file.rel_path.starts_with("tests/")
}

/// Executable SQL fragment on a line (strips trailing `-- ...` inline comments).
fn sql_executable_line(line: &str) -> &str {
    line.split_once("--").map(|(a, _)| a).unwrap_or(line).trim()
}

/// True when `delete without where` matched on `delete_line_idx` but a `WHERE` clause starts on a
/// later line (common style: `DELETE FROM t` then `WHERE …`).
fn delete_has_where_on_following_lines(text: &str, delete_line_idx: usize) -> bool {
    const MAX_LOOKAHEAD: usize = 24;
    let lines: Vec<&str> = text.lines().collect();
    let start = delete_line_idx.saturating_add(1);
    let end = (start + MAX_LOOKAHEAD).min(lines.len());
    for line in lines.iter().take(end).skip(start) {
        let exec = sql_executable_line(line);
        if exec.is_empty() {
            continue;
        }
        let lower = exec.trim().to_ascii_lowercase();
        if lower == "where"
            || lower.starts_with("where ")
            || lower.starts_with("where\t")
            || lower.starts_with("where(")
        {
            return true;
        }
    }
    false
}

fn destructive_migration_class(fragment: &str) -> Option<&'static str> {
    let lower = fragment.to_ascii_lowercase();
    if lower.contains("drop table")
        || lower.contains("drop database")
        || lower.contains("drop schema")
    {
        return Some("drop ddl");
    }
    if lower.contains("truncate table") {
        return Some("truncate");
    }
    if lower.contains("drop column")
        || lower.contains("drop index")
        || lower.contains("drop constraint")
    {
        return Some("drop object");
    }
    if lower.contains("delete from") && !lower.contains(" where ") {
        return Some("delete without where");
    }
    if lower.contains("alter table") && lower.contains(" drop ") {
        return Some("alter table drop");
    }
    None
}

fn is_migration_sql_file(file: &FileInfo, ctx: &AuditContext) -> bool {
    if file.suffix != ".sql" || file.is_generated {
        return false;
    }
    let p = file.rel_path.as_str();
    if p.starts_with("db/")
        || p.contains("/db/migrations/")
        || p.contains("/db/constraints/")
        || p.starts_with("migrations/")
        || p.starts_with("crates/adapters/")
        || p.starts_with("apps/api/migrations/")
        || matches_monorepo_migration_segment(p)
    {
        return true;
    }
    if let Some(m) = boundary_manifest(ctx) {
        if let Some(db) = m.db {
            for prefix in db
                .migration_paths
                .iter()
                .chain(db.root_paths.iter())
                .chain(db.constraint_paths.iter())
            {
                let pre = prefix.trim_end_matches('/');
                if p == pre || p.starts_with(&format!("{pre}/")) {
                    return true;
                }
            }
        }
    }
    false
}

/// Recognizes monorepo migration paths that are not covered by the simple
/// prefix list. Matches `packages/<name>/migration[s]/`, `apps/<name>/migration[s]/`,
/// and any `**/db/migrations/...` path.
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

pub fn destructive_sql_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    const FIX: &str = "add same-stem or same-directory migration metadata with owner/approval, rollback or roll-forward, backup/restore or irreversible approval, lock/timeout posture, and verify/check evidence; comments such as `jankurai:migration-safe` are not sufficient";
    let mut out = vec![];
    for file in &ctx.all_files {
        if !is_migration_sql_file(file, ctx) {
            continue;
        }
        if language_rules::sql_migration::destructive_safety_evidence_present(ctx, file) {
            continue;
        }
        for (idx, line) in file.text.lines().enumerate() {
            let frag = sql_executable_line(line);
            if frag.is_empty() {
                continue;
            }
            let Some(class) = destructive_migration_class(frag) else {
                continue;
            };
            if class == "delete without where"
                && delete_has_where_on_following_lines(&file.text, idx)
            {
                continue;
            }
            let line_no = idx + 1;
            let t = line.trim();
            let text = t.chars().take(160).collect::<String>();
            out.push(FindingHit {
                path: file.rel_path.clone(),
                line: Some(line_no),
                text: text.clone(),
                matched_term: Some(class.to_string()),
                agent_fix: FIX.into(),
                problem: format!("{class}: {text}"),
            });
            if out.len() >= 20 {
                return out;
            }
        }
    }
    out
}

pub fn streaming_runtime_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let mut hits = vec![];
    for file in ctx
        .all_files
        .iter()
        .filter(|f| f.is_code && !f.is_generated)
    {
        if !is_streaming_checked_path(&file.rel_path) || streaming_adapter_path(ctx, &file.rel_path)
        {
            continue;
        }
        let lower = file.text.to_ascii_lowercase();
        let Some(marker) = STREAMING_CLIENT_PATTERNS
            .iter()
            .find(|marker| lower.contains(**marker))
        else {
            continue;
        };
        if marker.contains("kafka") && kafka_exception_with_migration_path(ctx) {
            continue;
        }
        hits.push(FindingHit {
            path: file.rel_path.clone(),
            line: first_line_containing(&file.text, marker),
            text: (*marker).into(),
            matched_term: Some((*marker).into()),
            agent_fix: "move streaming clients behind the queue adapter boundary or document a brownfield exception with a migration path".into(),
            problem: format!(
                "streaming client marker `{marker}` appears outside `crates/adapters/queues`"
            ),
        });
        if hits.len() >= 20 {
            break;
        }
    }
    hits
}

fn is_streaming_checked_path(path: &str) -> bool {
    !(path.starts_with("docs/")
        || path.starts_with("paper/")
        || path.starts_with("reference/")
        || path.starts_with("tips/")
        || path.starts_with("agent/")
        || path.starts_with(".github/")
        || path.starts_with("packages/ux-qa/")
        || path.starts_with("crates/jankurai/"))
}

fn streaming_adapter_path(ctx: &AuditContext, path: &str) -> bool {
    if path.starts_with("crates/adapters/queues/")
        || path.starts_with("crates/adapters/src/queues/")
        || path.starts_with("adapters/queues/")
        || path.starts_with("apps/api/src/adapters/queues/")
    {
        return true;
    }
    boundary_manifest(ctx)
        .and_then(|manifest| manifest.queues)
        .map(|queues| {
            queues
                .adapter_paths
                .iter()
                .any(|adapter_path| path_matches_prefix(path, adapter_path))
        })
        .unwrap_or(false)
}

fn kafka_exception_with_migration_path(ctx: &AuditContext) -> bool {
    boundary_manifest(ctx)
        .map(|manifest| {
            manifest.streaming_exception.iter().any(|exception| {
                exception.runtime.eq_ignore_ascii_case("kafka")
                    && exception
                        .classification
                        .as_deref()
                        .or(exception.reason.as_deref())
                        .map(|value| value.eq_ignore_ascii_case("brownfield"))
                        .unwrap_or(false)
                    && !exception.owner.trim().is_empty()
                    && !exception.migration_path.trim().is_empty()
            })
        })
        .unwrap_or(false)
}

fn first_line_containing(text: &str, needle: &str) -> Option<usize> {
    let needle = needle.to_ascii_lowercase();
    text.lines()
        .position(|line| line.to_ascii_lowercase().contains(&needle))
        .map(|index| index + 1)
}

const GENERATED_ZONES_MANIFEST: &str = "agent/generated-zones.toml";

fn generated_text_header_has_marker(content: &str) -> bool {
    let header_lower = content
        .lines()
        .take(8)
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    header_lower.contains("generated")
        || header_lower.contains("do not edit")
        || header_lower.contains("auto-generated")
}

fn repo_score_json_has_generated_identity(content: &str) -> bool {
    let value = match serde_json::from_str::<serde_json::Value>(content) {
        Ok(value) => value,
        Err(_) => return false,
    };
    value
        .get("schema_url")
        .and_then(|v| v.as_str())
        .map(|schema| schema.ends_with("repo-score.schema.json"))
        .unwrap_or(false)
        && value.get("generated_at").and_then(|v| v.as_str()).is_some()
        && value
            .get("standard_version")
            .and_then(|v| v.as_str())
            .is_some()
        && value
            .get("auditor_version")
            .and_then(|v| v.as_str())
            .is_some()
        && value
            .get("schema_version")
            .and_then(|v| v.as_str())
            .is_some()
}

fn package_lock_json_has_native_shape(content: &str) -> bool {
    let value = match serde_json::from_str::<serde_json::Value>(content) {
        Ok(value) => value,
        Err(_) => return false,
    };
    value
        .get("lockfileVersion")
        .and_then(|v| v.as_u64())
        .is_some()
        && value.get("packages").and_then(|v| v.as_object()).is_some()
        && value
            .get("packages")
            .and_then(|v| v.get(""))
            .and_then(|v| v.as_object())
            .is_some()
}

fn structured_generated_identity(path: &str, content: &str) -> bool {
    match path {
        ".jankurai/repo-score.json" | "agent/repo-score.json" => {
            repo_score_json_has_generated_identity(content)
        }
        "package-lock.json" => package_lock_json_has_native_shape(content),
        _ => false,
    }
}

fn generated_zone_has_identity(path: &str, content: &str) -> bool {
    generated_text_header_has_marker(content) || structured_generated_identity(path, content)
}

fn generated_zone_manifest_present(ctx: &AuditContext) -> bool {
    ctx.all_files
        .iter()
        .any(|f| f.rel_path == GENERATED_ZONES_MANIFEST)
}

/// Returns findings when `agent/generated-zones.toml` exists, parses as zone rows, and any
/// `[[zone]]` omits non-empty `path`, `source`, or `command` (Phase 07 / generated-zone reproducibility).
pub fn generated_zone_manifest_metadata_issues(ctx: &AuditContext) -> Vec<FindingHit> {
    let full = ctx.root.join(GENERATED_ZONES_MANIFEST);
    if !full.exists() {
        return vec![];
    }
    let text = match std::fs::read_to_string(&full) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let file: crate::commands::context_data::GeneratedZonesFile = match toml::from_str(&text) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let mut problems: Vec<String> = vec![];
    for (index, zone) in file.zone.iter().enumerate() {
        let path = zone.path.trim();
        let source = zone.source.trim();
        let command = zone.command.trim();
        if path.is_empty() {
            problems.push(format!("zone[{index}]: missing or empty `path`"));
            continue;
        }
        if source.is_empty() {
            problems.push(format!(
                "zone `{}`: missing or empty `source`",
                zone.path.trim()
            ));
        }
        if command.is_empty() {
            problems.push(format!(
                "zone `{}`: missing or empty `command`",
                zone.path.trim()
            ));
        }
    }
    if problems.is_empty() {
        return vec![];
    }
    let detail = problems.join("; ");
    vec![FindingHit::new(
        GENERATED_ZONES_MANIFEST,
        1,
        &format!("generated zone manifest has incomplete reproducibility metadata: {detail}"),
    )]
}

pub fn generated_zone_issues(ctx: &AuditContext) -> Vec<FindingHit> {
    let generated = ctx
        .all_files
        .iter()
        .filter(|f| f.is_generated && f.is_code)
        .cloned()
        .collect::<Vec<_>>();
    let mut issues = vec![];
    for path in crate::audit::helpers::generated_zone_protected_paths(ctx) {
        issues.push(FindingHit {
            path: GENERATED_ZONES_MANIFEST.into(),
            line: Some(1),
            text: format!(
                "generated zone declaration `{path}` targets protected source or control-plane code"
            ),
            matched_term: Some("protected-generated-zone".into()),
            agent_fix: "remove the protected path from `agent/generated-zones.toml` or move the generated output to a derived artifact root that does not shadow tracked source or control-plane files".into(),
            problem: format!(
                "generated zone declaration `{path}` targets protected source or control-plane code"
            ),
        });
        issues.push(FindingHit {
            path: path.clone(),
            line: Some(1),
            text: format!(
                "generated zone declaration `{path}` targets protected source or control-plane code"
            ),
            matched_term: Some("protected-generated-zone".into()),
            agent_fix: "remove the protected path from `agent/generated-zones.toml` or move the generated output to a derived artifact root that does not shadow tracked source or control-plane files".into(),
            problem: format!(
                "generated zone declaration `{path}` targets protected source or control-plane code"
            ),
        });
    }
    if !generated.is_empty() {
        if !generated_zone_manifest_present(ctx) {
            issues.push(FindingHit::new(
                &generated[0].rel_path,
                1,
                "generated code exists without `agent/generated-zones.toml` ownership rules",
            ));
        }
        for file in generated {
            if !generated_zone_has_identity(&file.rel_path, &file.text) {
                issues.push(FindingHit::new(
                    &file.rel_path,
                    1,
                    "generated file lacks a clear generated/do-not-edit marker",
                ));
            }
            if !pattern_hits(std::slice::from_ref(&file), TODO_PATTERNS).is_empty() {
                issues.push(FindingHit::new(
                    &file.rel_path,
                    1,
                    "generated file contains TODO/stub markers",
                ));
            }
        }
    }
    issues
}

pub fn wrong_layer_db_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let mut hits = vec![];
    for file in product_files(ctx) {
        if !file.is_code && file.suffix != ".sql" {
            continue;
        }
        if file.rel_path.starts_with("db/")
            || file.rel_path.starts_with("migrations/")
            || file.rel_path.starts_with("crates/adapters/")
        {
            continue;
        }
        if ["apps/web/", "crates/domain/", "frontend/", "ui/", "src/"]
            .iter()
            .any(|p| file.rel_path.starts_with(p))
            && [
                "select ", "insert ", "update ", "delete ", "sqlx", "diesel", "psycopg", "sqlite3",
            ]
            .iter()
            .any(|m| file.text.to_ascii_lowercase().contains(m))
        {
            hits.push(FindingHit::new(
                &file.rel_path,
                1,
                "DB marker in non-adapter layer",
            ));
        }
    }
    hits
}

/// Detects repo-local report post-processing that strips or rewrites the
/// canonical `caps_applied`, `findings`, or `issues` fields instead of
/// preserving the source report shape.
pub fn report_post_processing_issues(ctx: &AuditContext) -> Vec<FindingHit> {
    if !ctx.self_audit {
        return vec![];
    }
    let mut hits = vec![];
    if let Some(file) = ctx
        .all_files
        .iter()
        .find(|f| f.rel_path == "crates/jankurai/src/commands/badge.rs")
    {
        if file.text.contains("caps_applied")
            && file.text.contains("findings")
            && file.text.contains("unwrap_or(0)")
        {
            hits.push(FindingHit {
                path: file.rel_path.clone(),
                line: first_line_containing(&file.text, "caps_applied").or(Some(1)),
                text: "badge post-processing defaults missing `caps_applied`/`findings` to zero instead of preserving the report fields".into(),
                matched_term: Some("caps_applied/findings fallback".into()),
                agent_fix: "require the canonical report fields or fail closed when they are absent; do not silently zero out report evidence".into(),
                problem: "badge post-processing rewrites or strips `caps_applied` and `findings`".into(),
            });
        }
    }
    if let Some(file) = ctx
        .all_files
        .iter()
        .find(|f| f.rel_path == "crates/jankurai/src/commands/paper.rs")
    {
        if file.text.contains("row.get(\"issues\")")
            && (file.text.contains("finding_count(row)")
                || file.text.contains("row[\"finding_count\"]"))
        {
            hits.push(FindingHit {
                path: file.rel_path.clone(),
                line: first_line_containing(&file.text, "finding_count(row)").or(Some(1)),
                text: "paper post-processing rewrites missing `issues` from `finding_count` instead of preserving the source row shape".into(),
                matched_term: Some("issues fallback".into()),
                agent_fix: "require the canonical `issues` field in the source rows or emit a hard error when it is missing".into(),
                problem: "paper post-processing rewrites or strips `issues`".into(),
            });
        }
    }
    hits
}

pub fn duplicate_blocks(ctx: &AuditContext) -> Vec<FindingHit> {
    let mut seen: std::collections::HashMap<String, Vec<(String, usize)>> =
        std::collections::HashMap::new();
    let mut dups = vec![];
    for file in product_code_files(ctx) {
        let lines: Vec<_> = file
            .text
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                if l.is_empty()
                    || l.starts_with("//")
                    || l.starts_with("#")
                    || l.starts_with("use ")
                    || l.starts_with("import ")
                {
                    return None;
                }
                let norm = l
                    .replace(['"', '\'', '`'], "\"S\"")
                    .chars()
                    .map(|c| if c.is_ascii_digit() { 'N' } else { c })
                    .collect::<String>();
                if norm.len() < 12 {
                    None
                } else {
                    Some(norm)
                }
            })
            .collect();
        for (start_idx, win) in lines.windows(8).enumerate() {
            let start_line = start_idx + 1;
            let body = win.join("\n");
            let prevs = seen.entry(body.clone()).or_default();
            if prevs.iter().any(|(path, prev_start)| {
                path == &file.rel_path && start_line.abs_diff(*prev_start) < 8
            }) {
                continue;
            }
            if let Some((prev_path, prev_start)) = prevs.first() {
                dups.push(FindingHit {
                    path: file.rel_path.clone(),
                    line: Some(start_line),
                    text: format!(
                        "duplicate block also appears at {}:{}",
                        prev_path, prev_start
                    ),
                    matched_term: Some("duplicate block".into()),
                    agent_fix: "extract the shared block into a helper or keep the block unique per file so overlapping windows do not trip the detector".into(),
                    problem: format!(
                        "duplicate block also appears at {}:{}",
                        prev_path, prev_start
                    ),
                });
            }
            prevs.push((file.rel_path.clone(), start_line));
        }
    }
    dups
}

pub fn future_hostile_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let allow_terms = crate::audit::fs_policy::dead_language_allow_terms(&ctx.root);
    let mut out = vec![];
    for file in product_code_files(ctx) {
        if is_future_hostile_allowlisted(&file) {
            continue;
        }
        for line in source_context::source_lines(&file) {
            if line.comment_only || line.test_scaffold {
                continue;
            }
            let active = line.active_code.trim();
            if active.is_empty() {
                continue;
            }
            if let Some((term, _regex)) = future_hostile_term_regexes()
                .iter()
                .find(|(_, regex)| regex.is_match(active))
            {
                if source_context::term_only_appears_in_local_binding(active, term) {
                    continue;
                }
                // Opt-in domain / platform-API allowlist
                // (`agent/audit-policy.toml` -> `[dead_language] allow_terms`):
                // suppress ONLY the exact words a repository has declared
                // load-bearing — e.g. the HTML `placeholder` attribute, the
                // React `fallback` prop, or GitHub's `stale` CheckConclusion
                // value. An empty/absent list keeps the default behaviour, so
                // repositories that do not opt in are unaffected.
                if allow_terms.iter().any(|allowed| allowed == term) {
                    continue;
                }
                out.push(FindingHit {
                    path: file.rel_path.clone(),
                    line: Some(line.line_no),
                    text: active.to_string(),
                    matched_term: Some(term.clone()),
                    agent_fix: "remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context".into(),
                    problem: format!("future-hostile/dead-language term `{}` appears", term),
                });
            }
        }
    }
    out
}

fn future_hostile_term_regexes() -> &'static [(String, Regex)] {
    static REGEXES: Lazy<Vec<(String, Regex)>> = Lazy::new(|| {
        FUTURE_HOSTILE_TERMS
            .iter()
            .filter_map(|term| {
                let pattern = format!(r"(?i)\b{}\b", regex::escape(term).replace("\\ ", r"\s+"));
                Regex::new(&pattern)
                    .ok()
                    .map(|regex| ((*term).to_string(), regex))
            })
            .collect()
    });
    REGEXES.as_slice()
}

fn is_future_hostile_allowlisted(file: &FileInfo) -> bool {
    file.is_generated
        || FUTURE_HOSTILE_ALLOWLIST_PREFIXES
            .iter()
            .any(|p| file.rel_path.starts_with(p))
        || FUTURE_HOSTILE_PRODUCT_COPY_PARTS
            .iter()
            .any(|p| file.rel_path.to_ascii_lowercase().contains(p))
}

pub fn manifest_parse_findings(ctx: &AuditContext) -> Vec<FindingHit> {
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct OwnerMapFile {
        #[allow(dead_code)]
        owners: HashMap<String, String>,
    }

    #[derive(Deserialize)]
    struct TestMapFile {
        #[allow(dead_code)]
        tests: HashMap<String, TestMapEntry>,
    }

    #[derive(Deserialize)]
    struct TestMapEntry {
        #[allow(dead_code)]
        command: String,
        #[allow(dead_code)]
        purpose: Option<String>,
    }

    type JsonManifestParser = fn(&str) -> std::result::Result<(), String>;

    let mut out = Vec::new();
    let json_manifests: &[(&str, JsonManifestParser)] = &[
        ("agent/owner-map.json", |text| {
            validation::parse_json_value_strict(text)
                .map_err(|err| anyhow::anyhow!("parse agent/owner-map.json: {err}"))
                .and_then(|value| {
                    serde_json::from_value::<OwnerMapFile>(value)
                        .map(|_| ())
                        .map_err(Into::into)
                })
                .map_err(|err: anyhow::Error| err.to_string())
        }),
        ("agent/test-map.json", |text| {
            validation::parse_json_value_strict(text)
                .map_err(|err| anyhow::anyhow!("parse agent/test-map.json: {err}"))
                .and_then(|value| {
                    serde_json::from_value::<TestMapFile>(value)
                        .map(|_| ())
                        .map_err(Into::into)
                })
                .map_err(|err: anyhow::Error| err.to_string())
        }),
    ];
    for (path, parse) in json_manifests {
        let full = ctx.root.join(path);
        if !full.exists() {
            continue;
        }
        let text = std::fs::read_to_string(&full).unwrap_or_default();
        if let Err(err) = parse(&text) {
            out.push(FindingHit::new(path, 1, &err));
        }
    }
    for path in [
        "agent/generated-zones.toml",
        "agent/boundaries.toml",
        "agent/proof-lanes.toml",
        "agent/standard-version.toml",
        "agent/audit-policy.toml",
    ] {
        let full = ctx.root.join(path);
        if !full.exists() {
            continue;
        }
        let text = std::fs::read_to_string(&full).unwrap_or_default();
        if let Err(err) = toml::from_str::<toml::Value>(&text) {
            out.push(FindingHit::new(path, 1, &err.to_string()));
        }
    }
    out
}

pub fn ci_hardening_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let mut hits = vec![];
    for file in ctx.all_files.iter().filter(|f| {
        !f.is_generated
            && f.rel_path.starts_with(".github/workflows/")
            && (f.rel_path.ends_with(".yml") || f.rel_path.ends_with(".yaml"))
    }) {
        for (idx, line) in file.text.lines().enumerate() {
            let line_lower = line.to_ascii_lowercase();
            if (line_lower.contains("continue-on-error")
                || line_lower.contains("allow_failure")
                || line_lower.contains("allow-failure")
                || line_lower.contains("|| true"))
                && (line_lower.contains("security")
                    || line_lower.contains("secret")
                    || line_lower.contains("dependency")
                    || line_lower.contains("sbom")
                    || line_lower.contains("proof")
                    || line_lower.contains("audit"))
            {
                hits.push(FindingHit {
                    path: file.rel_path.clone(),
                    line: Some(idx + 1),
                    text: line.to_string(),
                    matched_term: Some("nonblocking security job".into()),
                    agent_fix: "remove the nonblocking override and let the security or proof job fail so the CI gate actually proves the change".into(),
                    problem: "security or proof job is marked nonblocking".into(),
                });
            }
            if line_lower.contains("uses:") && line_lower.contains("@master") {
                hits.push(FindingHit {
                    path: file.rel_path.clone(),
                    line: Some(idx + 1),
                    text: line.to_string(),
                    matched_term: Some("@master".into()),
                    agent_fix: "pin action to a specific commit SHA or stable semver tag".into(),
                    problem: "workflow uses unpinned @master action".into(),
                });
            }
            if hits.len() >= 20 {
                return hits;
            }
        }
    }
    hits
}

/// Phase 07 H1: Detect contract source files under `contracts/` that have no matching
/// `[[zone]]` entry in `agent/generated-zones.toml`, indicating handwritten drift risk.
pub fn contract_source_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    const CONTRACT_EXTENSIONS: &[&str] = &[".yaml", ".yml", ".json", ".proto", ".tsp"];
    let contract_sources: Vec<&FileInfo> = ctx
        .all_files
        .iter()
        .filter(|f| {
            f.rel_path.starts_with("contracts/")
                && CONTRACT_EXTENSIONS
                    .iter()
                    .any(|ext| f.rel_path.ends_with(ext))
        })
        .collect();

    if contract_sources.is_empty() {
        return vec![];
    }

    // Load generated-zones.toml to check for matching zone sources
    let zones_path = ctx.root.join(GENERATED_ZONES_MANIFEST);
    let zone_sources: Vec<String> = if zones_path.exists() {
        std::fs::read_to_string(&zones_path)
            .ok()
            .and_then(|text| {
                toml::from_str::<crate::commands::context_data::GeneratedZonesFile>(&text).ok()
            })
            .map(|file| {
                file.zone
                    .iter()
                    .map(|z| z.source.trim().to_string())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        vec![]
    };

    let mut hits = vec![];
    for source in &contract_sources {
        let has_zone = zone_sources.iter().any(|zs| {
            zs == &source.rel_path
                || zs.contains(&source.rel_path)
                || source.rel_path.contains(zs.as_str())
        });
        if !has_zone {
            hits.push(FindingHit {
                path: source.rel_path.clone(),
                line: Some(1),
                text: format!(
                    "contract source `{}` has no generated zone entry — handwritten drift is likely",
                    source.rel_path
                ),
                matched_term: Some("orphaned-contract-source".into()),
                agent_fix: "add a `[[zone]]` in `agent/generated-zones.toml` with `source`, `command`, and `path` for this contract, or generate typed clients from it".into(),
                problem: format!(
                    "contract source `{}` has no generated zone entry",
                    source.rel_path
                ),
            });
        }
    }
    hits
}

/// Phase 07 H2: Verify that files declared in `agent/generated-zones.toml` actually exist
/// on disk and carry a `DO NOT EDIT` or `Generated by:` header.
pub fn generated_zone_existence_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let zones_path = ctx.root.join(GENERATED_ZONES_MANIFEST);
    if !zones_path.exists() {
        return vec![];
    }
    let text = match std::fs::read_to_string(&zones_path) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let file: crate::commands::context_data::GeneratedZonesFile = match toml::from_str(&text) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let mut hits = vec![];
    for zone in &file.zone {
        let path = zone.path.trim();
        if path.is_empty() {
            continue; // already caught by generated_zone_manifest_metadata_issues
        }
        let write_policy = zone.write_policy.trim();
        let optional_auditor_output = write_policy == "auditor_output";
        let full = ctx.root.join(path);
        if !full.exists() {
            if optional_auditor_output {
                continue;
            }
            hits.push(FindingHit {
                path: GENERATED_ZONES_MANIFEST.into(),
                line: Some(1),
                text: format!("declared generated zone `{path}` does not exist on disk"),
                matched_term: Some("missing-generated-zone-file".into()),
                agent_fix: format!("regenerate `{path}` using the declared command, or remove the zone entry if the file was deleted intentionally"),
                problem: format!("generated zone file `{path}` is missing"),
            });
            continue;
        }
        // Check text headers and structured metadata for generated identity.
        if let Ok(content) = std::fs::read_to_string(&full) {
            if !generated_zone_has_identity(path, &content) {
                hits.push(FindingHit {
                    path: path.into(),
                    line: Some(1),
                    text: format!(
                        "generated zone file `{path}` lacks a `Generated by:` or `DO NOT EDIT` header"
                    ),
                    matched_term: Some("missing-generated-header".into()),
                    agent_fix: "add a `Generated by: <tool>` / `DO NOT EDIT BY HAND` header block with source and regeneration command".into(),
                    problem: format!("generated zone file `{path}` missing generated header"),
                });
            }
        }
    }
    hits
}

/// Phase 07 H4: Verify that event contract paths declared in `agent/boundaries.toml`
/// actually exist on disk.
pub fn event_contract_path_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let manifest = match boundary_manifest(ctx) {
        Some(m) => m,
        None => return vec![],
    };
    let queues = match manifest.queues {
        Some(q) => q,
        None => return vec![],
    };

    let mut hits = vec![];
    for path in &queues.event_contract_paths {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }
        let full = ctx.root.join(trimmed);
        if !full.exists() && !full.is_dir() {
            hits.push(FindingHit {
                path: "agent/boundaries.toml".into(),
                line: Some(1),
                text: format!(
                    "declared event contract path `{trimmed}` does not exist"
                ),
                matched_term: Some("missing-event-contract-path".into()),
                agent_fix: format!("create `{trimmed}` with event schemas, or update `agent/boundaries.toml` if the path was moved"),
                problem: format!("event contract path `{trimmed}` is missing"),
            });
        }
    }
    hits
}

/// Write policies whose declared generated zone is EXPECTED to mutate during
/// normal operation, so a working-tree change is not a hand-edit. `auditor_output`
/// zones are refreshed by the auditor itself; `lockfile` zones are refreshed by
/// the package manager. Both are skipped by the generated-zone governance guard.
const GENERATED_ZONE_EXPECTED_MUTATION_POLICIES: &[&str] = &["auditor_output", "lockfile"];

/// HLT-045-GENERATED-ZONE-GOVERNANCE: flags hand-edits inside declared generated
/// zones. A "hand-edit" is a working-tree modification of a file that lives in a
/// generated zone whose `write_policy` is generator-managed (anything other than
/// `auditor_output` or `lockfile`, which are expected to mutate).
///
/// Zones are read from `agent/generated-zones.toml` via [`generated_zone_paths`]
/// (paired here with their write policy). Modified files are read from
/// `git status --porcelain`, so a clean generated zone — like jankurai's own —
/// yields zero findings. The guard is advisory and never recurses into siblings.
pub fn generated_zone_edit_hits(ctx: &AuditContext) -> Vec<FindingHit> {
    let zones = governed_generated_zone_paths(ctx);
    if zones.is_empty() {
        return vec![];
    }
    let modified = match crate::audit::smart_scan::git_status_changed_files(&ctx.root) {
        Ok(paths) => paths,
        Err(_) => return vec![],
    };
    let mut hits = vec![];
    for path in modified {
        let rel = path.to_string_lossy().replace('\\', "/");
        let Some(zone) = zones
            .iter()
            .find(|zone| path_matches_prefix(&rel, zone))
            .cloned()
        else {
            continue;
        };
        hits.push(FindingHit {
            path: rel.clone(),
            line: None,
            text: format!(
                "`{rel}` was hand-edited inside declared generated zone `{zone}`"
            ),
            matched_term: Some("generated-zone-hand-edit".into()),
            agent_fix: format!(
                "revert the in-place edit to `{rel}` and regenerate it from the declared source/command in `agent/generated-zones.toml`; do not patch generated output by hand"
            ),
            problem: format!(
                "generated zone `{zone}` has an uncommitted hand-edit at `{rel}` instead of a regeneration"
            ),
        });
        if hits.len() >= 20 {
            break;
        }
    }
    hits
}

/// Returns the declared generated-zone paths whose `write_policy` is
/// generator-managed (i.e. NOT `auditor_output` or `lockfile`). Reuses the same
/// manifest read path as [`generated_zone_paths`] but keeps only the zones that
/// the governance guard should police.
fn governed_generated_zone_paths(ctx: &AuditContext) -> Vec<String> {
    let path = ctx.root.join(GENERATED_ZONES_MANIFEST);
    if !path.exists() {
        return vec![];
    }
    let Ok(text) = std::fs::read_to_string(&path) else {
        return vec![];
    };
    let Ok(file) = toml::from_str::<crate::commands::context_data::GeneratedZonesFile>(&text)
    else {
        return vec![];
    };
    file.zone
        .into_iter()
        .filter(|zone| {
            !GENERATED_ZONE_EXPECTED_MUTATION_POLICIES.contains(&zone.write_policy.trim())
        })
        .map(|zone| zone.path.trim().to_string())
        .filter(|zone_path| !zone_path.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn generated_path_dir_prefixes_remain_recognized() {
        assert!(is_generated_or_reference_path("docs/index.md"));
        assert!(is_generated_or_reference_path("paper/intro.md"));
        assert!(is_generated_or_reference_path("reference/foo.md"));
        assert!(is_generated_or_reference_path("tips/bar.md"));
        assert!(is_generated_or_reference_path("generated/types.ts"));
        assert!(is_generated_or_reference_path(
            "crates/foo/src/generated/api.rs"
        ));
        assert!(is_generated_or_reference_path("target/debug/build.txt"));
    }

    #[test]
    fn generated_path_recognizes_gen_suffixes() {
        assert!(is_generated_or_reference_path("apps/web/src/api.gen.ts"));
        assert!(is_generated_or_reference_path("apps/web/src/api.gen.tsx"));
        assert!(is_generated_or_reference_path(
            "packages/sdk/dist/index.gen.js"
        ));
        assert!(is_generated_or_reference_path(
            "packages/sdk/dist/worker.gen.mjs"
        ));
    }

    #[test]
    fn generated_path_recognizes_sst_env_anywhere() {
        assert!(is_generated_or_reference_path("sst-env.d.ts"));
        assert!(is_generated_or_reference_path("packages/core/sst-env.d.ts"));
        assert!(is_generated_or_reference_path(
            "apps/web/nested/dir/sst-env.d.ts"
        ));
    }

    #[test]
    fn generated_path_does_not_match_unrelated_files() {
        assert!(!is_generated_or_reference_path("apps/web/src/main.ts"));
        assert!(!is_generated_or_reference_path("packages/foo/src/lib.ts"));
        assert!(!is_generated_or_reference_path("crates/foo/src/lib.rs"));
        // similar names that should not match the suffix pattern
        assert!(!is_generated_or_reference_path("apps/web/sst-env.ts"));
        assert!(!is_generated_or_reference_path("apps/web/regen.ts"));
    }

    #[test]
    fn generated_zone_protected_paths_do_not_suppress_protected_sources() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("agent")).unwrap();
        std::fs::write(
            dir.path().join("agent/generated-zones.toml"),
            r#"[[zone]]
path = "crates/foo/src/lib.rs"
source = "contracts/openapi.yaml"
command = "cargo run -p jankurai -- generate"
"#,
        )
        .unwrap();
        let ctx = AuditContext {
            root: dir.path().to_path_buf(),
            all_files: vec![
                FileInfo {
                    rel_path: "agent/generated-zones.toml".into(),
                    name: "generated-zones.toml".into(),
                    suffix: ".toml".into(),
                    size: 0,
                    line_count: 1,
                    text: r#"[[zone]]
path = "crates/foo/src/lib.rs"
source = "contracts/openapi.yaml"
command = "cargo run -p jankurai -- generate"
"#
                    .into(),
                    is_generated: false,
                    is_code: false,
                },
                FileInfo {
                    rel_path: "crates/foo/src/lib.rs".into(),
                    name: "lib.rs".into(),
                    suffix: ".rs".into(),
                    size: 0,
                    line_count: 1,
                    text: "pub fn demo() {\n    // TODO: fix me\n}\n".into(),
                    is_generated: false,
                    is_code: true,
                },
            ],
            scope_files: vec![],
            scope_paths: vec![],
            self_audit: true,
            boundary_reclassifications: vec![],
            copy_code: None,
        };
        let protected = crate::audit::helpers::generated_zone_protected_paths(&ctx);
        assert_eq!(protected, vec!["crates/foo/src/lib.rs".to_string()]);
        let suppression = crate::audit::helpers::generated_zone_suppression_paths(&ctx);
        assert!(suppression.is_empty());
        let issues = generated_zone_issues(&ctx);
        assert!(issues
            .iter()
            .any(|issue| issue.path == "agent/generated-zones.toml"));
        assert!(issues
            .iter()
            .any(|issue| issue.path == "crates/foo/src/lib.rs"));
    }

    #[test]
    fn report_post_processing_issues_detects_fallback_shape_changes() {
        let ctx = AuditContext {
            root: std::path::PathBuf::from("/repo"),
            all_files: vec![
                FileInfo {
                    rel_path: "crates/jankurai/src/commands/badge.rs".into(),
                    name: "badge.rs".into(),
                    suffix: ".rs".into(),
                    size: 0,
                    line_count: 1,
                    text: r#"
let findings = value
    .get("findings")
    .and_then(Value::as_array)
    .map(Vec::len)
    .unwrap_or(0);
let caps = value
    .get("caps_applied")
    .and_then(Value::as_array)
    .map(Vec::len)
    .unwrap_or(0);
"#
                    .into(),
                    is_generated: false,
                    is_code: true,
                },
                FileInfo {
                    rel_path: "crates/jankurai/src/commands/paper.rs".into(),
                    name: "paper.rs".into(),
                    suffix: ".rs".into(),
                    size: 0,
                    line_count: 1,
                    text: r#"
fn finding_count(row: &Value) -> Result<u64> {
    if row.get("issues").is_some() {
        integer(&row["issues"])
    } else {
        integer(&row["finding_count"])
    }
}
"#
                    .into(),
                    is_generated: false,
                    is_code: true,
                },
            ],
            scope_files: vec![],
            scope_paths: vec![],
            self_audit: true,
            boundary_reclassifications: vec![],
            copy_code: None,
        };
        let hits = report_post_processing_issues(&ctx);
        assert!(hits.iter().any(|hit| hit.path.ends_with("badge.rs")));
        assert!(hits.iter().any(|hit| hit.path.ends_with("paper.rs")));
    }

    fn product_file(rel_path: &str, text: &str) -> FileInfo {
        FileInfo {
            rel_path: rel_path.into(),
            name: std::path::PathBuf::from(rel_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            suffix: std::path::PathBuf::from(rel_path)
                .extension()
                .map(|ext| format!(".{}", ext.to_string_lossy()))
                .unwrap_or_default(),
            size: text.len() as u64,
            line_count: text.lines().count(),
            text: text.into(),
            is_generated: false,
            is_code: true,
        }
    }

    #[test]
    fn pattern_needs_word_boundary_only_for_bare_words() {
        assert!(pattern_needs_word_boundary("retry"));
        assert!(pattern_needs_word_boundary("TODO"));
        assert!(pattern_needs_word_boundary("stub"));
        assert!(!pattern_needs_word_boundary("not implemented"));
        assert!(!pattern_needs_word_boundary("// placeholder"));
        assert!(!pattern_needs_word_boundary("placeholder!("));
        assert!(!pattern_needs_word_boundary("<placeholder>"));
        assert!(!pattern_needs_word_boundary("silent retry"));
        assert!(!pattern_needs_word_boundary("todo!("));
    }

    #[test]
    fn pattern_match_with_boundary_skips_substrings_in_identifiers() {
        // bare word inside a longer identifier must not match
        assert_eq!(
            pattern_matches_with_boundary("pub retry_after_seconds: Option<u64>,", "retry"),
            None
        );
        assert_eq!(
            pattern_matches_with_boundary("let argumentSlots = match(re);", "stub"),
            None
        );
        // word boundary on its own does match
        assert!(pattern_matches_with_boundary("we should retry here", "retry").is_some());
    }

    #[test]
    fn pattern_match_with_boundary_keeps_shape_substrings() {
        // shape patterns are flagged anywhere they appear because they are already specific
        assert!(
            pattern_matches_with_boundary("// placeholder until ready", "// placeholder").is_some()
        );
        assert!(pattern_matches_with_boundary("placeholder!(\"x\")", "placeholder!(").is_some());
        assert!(pattern_matches_with_boundary("not implemented yet", "not implemented").is_some());
    }

    #[test]
    fn fallback_patterns_no_longer_match_retry_struct_field() {
        let line = "    pub retry_after_seconds: Option<u64>,";
        for pattern in FALLBACK_PATTERNS {
            assert!(
                pattern_matches_with_boundary(line, pattern).is_none(),
                "pattern `{}` must not match `{}`",
                pattern,
                line
            );
        }
    }

    #[test]
    fn fallback_patterns_match_specific_retry_phrases() {
        let line = "we have an unbounded retry loop here";
        let matched = FALLBACK_PATTERNS
            .iter()
            .find(|pat| pattern_matches_with_boundary(line, pat).is_some());
        assert!(matched.is_some(), "expected an FALLBACK_PATTERNS match");
    }

    #[test]
    fn fallback_hits_skips_comment_only_lines_and_keeps_real_error_hiding_fallbacks() {
        let text = [
            "let policy = \"silent retry\";",
            "let mode = \"best effort\";",
            "let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();",
            "// fallback: use typed state",
            "router.fallback_service(handler);",
        ]
        .join("\n");
        let ctx = make_ctx(vec![product_file("apps/api/src/router.rs", &text)]);
        let hits = fallback_hits(&ctx);
        assert_eq!(
            hits.len(),
            2,
            "expected only the real error-hiding fallback markers"
        );
        assert!(
            hits.iter().all(|hit| !hit.text.contains("file_name")),
            "deterministic path parsing should not be reported"
        );
        assert!(hits.iter().any(|hit| hit.text.contains("silent retry")));
        assert!(hits.iter().any(|hit| hit.text.contains("best effort")));
    }

    #[test]
    fn fallback_hits_skips_unwrap_or_candidate_defaults() {
        let text = "let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or(candidate);\n";
        let ctx = make_ctx(vec![product_file("apps/api/src/router.rs", text)]);
        assert!(
            fallback_hits(&ctx).is_empty(),
            "unwrap_or(candidate) should not be considered fallback soup"
        );
    }

    #[test]
    fn todo_patterns_no_longer_flag_argument_slots_identifier() {
        let line = "const argumentSlots = commandPrompt.match(argumentSlotRegex);";
        for pattern in TODO_PATTERNS {
            assert!(
                pattern_matches_with_boundary(line, pattern).is_none(),
                "pattern `{}` must not match `{}`",
                pattern,
                line
            );
        }
    }

    #[test]
    fn todo_patterns_flag_actual_placeholder_comment() {
        let line = "// TODO: placeholder until v2 lands";
        let matched: Vec<&&str> = TODO_PATTERNS
            .iter()
            .filter(|pat| pattern_matches_with_boundary(line, pat).is_some())
            .collect();
        assert!(
            !matched.is_empty(),
            "expected at least one TODO/placeholder pattern to match"
        );
    }

    #[test]
    fn todo_hits_ignores_comments_and_test_scaffolding() {
        let text = [
            "#[cfg(test)]",
            "mod tests {",
            "    #[test]",
            "    fn smoke() {",
            "        // TODO: stub fallback legacy stale shim",
            "        let fallback = 1;",
            "    }",
            "}",
        ]
        .join("\n");
        let ctx = make_ctx(vec![product_file("crates/app/src/lib.rs", &text)]);
        assert!(
            todo_hits(&ctx).is_empty(),
            "comment-only TODOs inside Rust test scaffolding should be ignored"
        );
    }

    #[test]
    fn future_hostile_hits_keep_runtime_strings_but_skip_local_bindings_and_comments() {
        let text = [
            "pub const MODE: &str = \"legacy mode\";",
            "let fallback = choose_default();",
            "Settings { params: fallback }",
            "// stale shim fallback stub",
        ]
        .join("\n");
        let ctx = make_ctx(vec![product_file("apps/api/src/config.rs", &text)]);
        let hits = future_hostile_hits(&ctx);
        assert!(
            hits.iter().any(|hit| hit.text.contains("legacy mode")),
            "runtime string marker should remain visible"
        );
        assert!(
            hits.iter().all(|hit| !hit.text.contains("let fallback")),
            "local binding-only fallback should not be reported"
        );
    }

    #[test]
    fn false_green_patterns_do_not_flag_iterator_skip() {
        let text = "const values = items.iter().skip(1).collect();\n";
        let ctx = make_ctx(vec![product_file("apps/web/src/widgets.test.ts", text)]);
        assert!(
            false_green_hits(&ctx).is_empty(),
            "iterator skip should not be treated as a false-green test skip"
        );
    }

    #[test]
    fn false_green_patterns_flag_test_framework_skip() {
        let text = "it.skip(\"smoke path\", () => {});\n";
        let ctx = make_ctx(vec![product_file("apps/web/src/widgets.test.ts", text)]);
        let hits = false_green_hits(&ctx);
        assert_eq!(hits.len(), 1);
        assert!(
            hits[0].text.contains("it.skip("),
            "framework skip call should be detected"
        );
    }

    fn make_ctx(files: Vec<FileInfo>) -> AuditContext {
        AuditContext {
            root: std::path::PathBuf::from("."),
            scope_files: files.clone(),
            all_files: files,
            scope_paths: vec![],
            self_audit: false,
            boundary_reclassifications: vec![],
            copy_code: None,
        }
    }

    fn synthetic_secret(parts: &[&str]) -> String {
        parts.concat()
    }

    fn quoted_synthetic_secret(parts: &[&str]) -> String {
        format!("\"{}\"", synthetic_secret(parts))
    }

    #[test]
    fn nearby_allow_suppresses_secret_hit() {
        let text = format!(
            "// jankurai:allow HLT-010-SECRET-SPRAWL reason=test fixture expires=2099-12-31\nlet api_key = \"{}\";\n",
            synthetic_secret(&["sk", "-test-AAAAAAAAAAAAAAAA"])
        );
        let ctx = make_ctx(vec![product_file("apps/api/src/keys.rs", &text)]);
        let hits = secret_hits(&ctx);
        assert!(
            hits.is_empty(),
            "nearby allow comment should suppress secret_hits, got {} hits",
            hits.len()
        );
    }

    #[test]
    fn nearby_allow_suppresses_input_boundary_hit() {
        let text = "// jankurai:allow HLT-023-INPUT-BOUNDARY-GAP reason=isolated test expires=2099-12-31\nelement.innerHTML = userInput;\n";
        let ctx = make_ctx(vec![product_file("apps/web/src/dom.ts", text)]);
        let hits = input_boundary_hits(&ctx);
        assert!(
            hits.is_empty(),
            "nearby allow comment should suppress input_boundary_hits, got {} hits",
            hits.len()
        );
    }

    #[test]
    fn duplicate_blocks_ignores_overlapping_windows_from_same_file() {
        let text = [
            "fn overlap() {",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "}",
        ]
        .join("\n");
        let ctx = make_ctx(vec![product_file("apps/api/src/overlap.rs", &text)]);
        assert!(
            duplicate_blocks(&ctx).is_empty(),
            "overlapping duplicate windows in the same file should be ignored"
        );
    }

    #[test]
    fn duplicate_blocks_still_flags_same_block_in_another_file() {
        let block = [
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
            "    let alpha = 1234567890;",
        ]
        .join("\n");
        let ctx = make_ctx(vec![
            product_file("apps/api/src/a.rs", &block),
            product_file("apps/api/src/b.rs", &block),
        ]);
        let hits = duplicate_blocks(&ctx);
        assert_eq!(hits.len(), 1, "expected a cross-file duplicate finding");
        assert!(
            hits[0].text.contains("apps/api/src/a.rs"),
            "duplicate report should reference the first file"
        );
    }

    #[test]
    fn secret_assignment_skips_bare_identifier_paths() {
        assert!(!secret_assignment_value_is_secret_like("model.api_key"));
        assert!(!secret_assignment_value_is_secret_like("config.token"));
        assert!(!secret_assignment_value_is_secret_like("env.API_KEY"));
        assert!(!secret_assignment_value_is_secret_like("${API_KEY}"));
    }

    #[test]
    fn secret_assignment_accepts_literal_string_secrets() {
        assert!(secret_assignment_value_is_secret_like(
            &quoted_synthetic_secret(&["sk", "-proj-AAAAAAAAAAAAAAAAAAAA"])
        ));
        assert!(secret_assignment_value_is_secret_like(
            &quoted_synthetic_secret(&["eyJ", "hbGciOiJIUzI1NiJ9.AAAAAAAAAA.BBBBBBBBBB"])
        ));
        assert!(secret_assignment_value_is_secret_like(
            &quoted_synthetic_secret(&["AK", "IAABCDEFGHIJKLMNOP"])
        ));
    }

    #[test]
    fn secret_assignment_accepts_high_entropy_unquoted_prefixes() {
        assert!(secret_assignment_value_is_secret_like(&synthetic_secret(
            &["gh", "p_aaaaaaaaaaaaaaaaaaaaaaaaaaaa"]
        )));
        assert!(secret_assignment_value_is_secret_like(&synthetic_secret(
            &["sk", "-test-aaaaaaaaaaaaaaaa"]
        )));
        assert!(secret_assignment_value_is_secret_like(&synthetic_secret(
            &["xox", "b-1234567890-abcdefghijklmnop"]
        )));
        assert!(secret_assignment_value_is_secret_like(&synthetic_secret(
            &["eyJ", "hbGciOiJIUzI1NiJ9.AAAAAAAAAA.BBBBBBBBBB"]
        )));
    }

    #[test]
    fn secret_hits_skips_identifier_assignment() {
        // `api_key: model.api_key` is a parameter forwarding pattern, not a literal credential.
        let text = "fn build(model: &Model) -> Settings {\n    Settings { api_key: model.api_key.clone() }\n}\n";
        let ctx = make_ctx(vec![product_file("apps/api/src/build.rs", text)]);
        let hits = secret_hits(&ctx);
        assert!(
            hits.is_empty(),
            "bare identifier RHS should not flag, got {} hits",
            hits.len()
        );
    }

    #[test]
    fn secret_hits_flags_literal_string_credential() {
        let text = format!(
            "let cfg = Cfg {{ api_key: \"{}\" }};\n",
            synthetic_secret(&["sk", "-proj-AAAAAAAAAAAAAAAAAAAA"])
        );
        let ctx = make_ctx(vec![product_file("apps/api/src/cfg.rs", &text)]);
        let hits = secret_hits(&ctx);
        assert!(
            !hits.is_empty(),
            "literal string credential should flag HLT-010"
        );
    }

    #[test]
    fn secret_hits_flags_jwt_access_token() {
        let text = format!(
            "let cfg = Cfg {{ access_token: \"{}\" }};\n",
            synthetic_secret(&["eyJ", "hbGciOiJIUzI1NiJ9.AAAAAAAAAA.BBBBBBBBBB"])
        );
        let ctx = make_ctx(vec![product_file("apps/api/src/cfg.rs", &text)]);
        let hits = secret_hits(&ctx);
        assert!(
            !hits.is_empty(),
            "literal JWT access_token should flag HLT-010"
        );
    }

    #[test]
    fn secret_hits_ignores_regex_scan_examples_but_flags_real_literals() {
        let example = product_file(
            "CHANGELOG.md",
            "Use `grep -rEn 'sk-|sk_|hf_|AIza|gsk_'` to audit samples.\n",
        );
        let literal_text = format!(
            "Rotate `{}` immediately.\n",
            synthetic_secret(&["sk", "-proj-AAAAAAAAAAAAAAAAAAAA"])
        );
        let literal = product_file("CHANGELOG.md", &literal_text);
        let example_ctx = make_ctx(vec![example]);
        let literal_ctx = make_ctx(vec![literal]);
        assert!(secret_hits(&example_ctx).is_empty());
        assert!(!secret_hits(&literal_ctx).is_empty());
    }

    // --- HLT-045 generated-zone governance (hand-edit) guard ---------------

    fn git_run(dir: &std::path::Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git available");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn init_git_repo(dir: &std::path::Path) {
        git_run(dir, &["init", "-q"]);
        git_run(dir, &["config", "user.email", "t@example.com"]);
        git_run(dir, &["config", "user.name", "Test"]);
    }

    fn zone_edit_ctx(root: std::path::PathBuf) -> AuditContext {
        AuditContext {
            root,
            all_files: vec![],
            scope_files: vec![],
            scope_paths: vec![],
            self_audit: true,
            boundary_reclassifications: vec![],
            copy_code: None,
        }
    }

    #[test]
    fn generated_zone_edit_flags_hand_edit_in_governed_zone() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_git_repo(root);
        std::fs::create_dir_all(root.join("agent")).unwrap();
        std::fs::create_dir_all(root.join("generated")).unwrap();
        std::fs::write(
            root.join("agent/generated-zones.toml"),
            r#"[[zone]]
path = "generated/api.ts"
source = "contracts/openapi.yaml"
command = "cargo run -p jankurai -- generate"
read_only = true
write_policy = "generator_only"
"#,
        )
        .unwrap();
        std::fs::write(
            root.join("generated/api.ts"),
            "// Generated\nexport const x = 1;\n",
        )
        .unwrap();
        git_run(root, &["add", "-A"]);
        git_run(root, &["commit", "-q", "-m", "init"]);
        // Hand-edit the generated file (now dirty in the worktree).
        std::fs::write(
            root.join("generated/api.ts"),
            "// Generated\nexport const x = 2;\n",
        )
        .unwrap();

        let hits = generated_zone_edit_hits(&zone_edit_ctx(root.to_path_buf()));
        assert_eq!(
            hits.len(),
            1,
            "hand-edit in governed zone expected: {hits:?}"
        );
        assert_eq!(hits[0].path, "generated/api.ts");
        assert_eq!(
            hits[0].matched_term.as_deref(),
            Some("generated-zone-hand-edit")
        );
    }

    #[test]
    fn generated_zone_edit_skips_auditor_output_and_lockfile() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_git_repo(root);
        std::fs::create_dir_all(root.join("agent/baselines")).unwrap();
        std::fs::write(
            root.join("agent/generated-zones.toml"),
            r#"[[zone]]
path = "agent/baselines/main.repo-score.json"
source = "audit output"
command = "cargo run -p jankurai -- audit"
read_only = true
write_policy = "auditor_output"

[[zone]]
path = "package-lock.json"
source = "package.json"
command = "npm install"
read_only = true
write_policy = "lockfile"
"#,
        )
        .unwrap();
        std::fs::write(root.join("agent/baselines/main.repo-score.json"), "{}\n").unwrap();
        std::fs::write(root.join("package-lock.json"), "{}\n").unwrap();
        git_run(root, &["add", "-A"]);
        git_run(root, &["commit", "-q", "-m", "init"]);
        // Mutate both expected-mutation zones; neither should be flagged.
        std::fs::write(
            root.join("agent/baselines/main.repo-score.json"),
            "{\"x\":1}\n",
        )
        .unwrap();
        std::fs::write(root.join("package-lock.json"), "{\"x\":1}\n").unwrap();

        let hits = generated_zone_edit_hits(&zone_edit_ctx(root.to_path_buf()));
        assert!(
            hits.is_empty(),
            "auditor_output and lockfile zones are expected to mutate: {hits:?}"
        );
    }

    #[test]
    fn generated_zone_edit_clean_zone_is_advisory_no_finding() {
        // Ratchet-readiness: a clean governed zone (no working-tree edit) yields
        // zero findings, so the guard cannot auto-fail a currently-green repo.
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_git_repo(root);
        std::fs::create_dir_all(root.join("agent")).unwrap();
        std::fs::create_dir_all(root.join("generated")).unwrap();
        std::fs::write(
            root.join("agent/generated-zones.toml"),
            r#"[[zone]]
path = "generated/api.ts"
source = "contracts/openapi.yaml"
command = "cargo run -p jankurai -- generate"
read_only = true
write_policy = "generator_only"
"#,
        )
        .unwrap();
        std::fs::write(
            root.join("generated/api.ts"),
            "// Generated\nexport const x = 1;\n",
        )
        .unwrap();
        git_run(root, &["add", "-A"]);
        git_run(root, &["commit", "-q", "-m", "init"]);

        let hits = generated_zone_edit_hits(&zone_edit_ctx(root.to_path_buf()));
        assert!(
            hits.is_empty(),
            "clean governed zone must be silent: {hits:?}"
        );
    }

    #[test]
    fn generated_zone_edit_ignores_edits_outside_zones() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_git_repo(root);
        std::fs::create_dir_all(root.join("agent")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("agent/generated-zones.toml"),
            r#"[[zone]]
path = "generated/api.ts"
source = "contracts/openapi.yaml"
command = "cargo run -p jankurai -- generate"
read_only = true
write_policy = "generator_only"
"#,
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
        git_run(root, &["add", "-A"]);
        git_run(root, &["commit", "-q", "-m", "init"]);
        // Edit a NON-zone file; must not be flagged.
        std::fs::write(root.join("src/lib.rs"), "pub fn a() {}\npub fn b() {}\n").unwrap();

        let hits = generated_zone_edit_hits(&zone_edit_ctx(root.to_path_buf()));
        assert!(hits.is_empty(), "non-zone edits must be ignored: {hits:?}");
    }
}
