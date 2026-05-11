use anyhow::{bail, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct PromptVerifyArgs {
    pub repo: PathBuf,
    pub document: String,
    pub out: Option<String>,
    pub md: Option<String>,
    pub strict: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptVerificationReport {
    pub schema_version: String,
    pub command: String,
    pub status: String,
    pub decision: String,
    pub repo: String,
    pub document: String,
    pub claims_total: usize,
    pub claims_verified: usize,
    pub claims_invalid: usize,
    pub claims_review: usize,
    pub claims: Vec<PromptClaim>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptClaim {
    pub claim_type: String,
    pub claim: String,
    pub decision: String,
    pub evidence: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
struct ClaimCandidate {
    claim_type: ClaimType,
    claim: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimType {
    PathLine,
    ModuleSymbol,
    ClassClaim,
    LlmCall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimDecision {
    Verified,
    Invalid,
    Review,
}

const SUPPORTED_LLM_PROVIDERS: &[LlmProvider] = &[
    LlmProvider {
        name: "openai",
        import_patterns: &[
            r"(?m)^\s*use\s+openai::",
            r"(?m)^\s*import\s+openai\b",
            r"(?m)^\s*from\s+openai\s+import\s+OpenAI\b",
            r#"(?m)^\s*import\s+.*\bOpenAI\b.*\bfrom\s+["']openai["']"#,
            r#"(?m)^\s*(const|let|var)\s+\{?\s*OpenAI\s*\}?\s*=\s*require\(["']openai["']\)"#,
        ],
        invocation_patterns: &[
            r"\.responses\.create\s*\(",
            r"\.chat\.completions\.create\s*\(",
            r"\.completions\.create\s*\(",
        ],
    },
    LlmProvider {
        name: "anthropic",
        import_patterns: &[
            r"(?m)^\s*import\s+anthropic\b",
            r"(?m)^\s*from\s+anthropic\s+import\s+Anthropic\b",
            r"(?m)^\s*use\s+anthropic::",
            r#"(?m)^\s*import\s+.*\bAnthropic\b.*\bfrom\s+["'](@anthropic-ai/sdk|anthropic)["']"#,
            r#"(?m)^\s*(const|let|var)\s+\{?\s*Anthropic\s*\}?\s*=\s*require\(["'](@anthropic-ai/sdk|anthropic)["']\)"#,
        ],
        invocation_patterns: &[r"\.messages\.create\s*\(", r"\.completions\.create\s*\("],
    },
    LlmProvider {
        name: "langchain",
        import_patterns: &[
            r"(?m)^\s*(from\s+langchain|import\s+langchain|use\s+langchain::)",
            r#"(?m)^\s*import\s+.*\bfrom\s+["'](@?langchain/[^"']+|langchain)["']"#,
            r#"(?m)^\s*(const|let|var)\s+.*=\s*require\(["'](@?langchain/[^"']+|langchain)["']\)"#,
        ],
        invocation_patterns: &[r"\.invoke\s*\(", r"\.predict\s*\(", r"\.generate\s*\("],
    },
    LlmProvider {
        name: "llamaindex",
        import_patterns: &[
            r"(?m)^\s*(from\s+llama_index|import\s+llama_index|use\s+llama_index::)",
            r#"(?m)^\s*import\s+.*\bfrom\s+["'](llamaindex|llama_index)["']"#,
            r#"(?m)^\s*(const|let|var)\s+.*=\s*require\(["'](llamaindex|llama_index)["']\)"#,
        ],
        invocation_patterns: &[r"\.query\s*\(", r"\.chat\s*\(", r"\.complete\s*\("],
    },
    LlmProvider {
        name: "nano-bridge",
        import_patterns: &[
            r"(?m)^\s*(from\s+nano[_-]?bridge|import\s+nano[_-]?bridge|use\s+nano[_-]?bridge::)",
            r"(?m)^\s*(from\s+nano_bridge|import\s+nano_bridge|use\s+nano_bridge::)",
            r#"(?m)^\s*import\s+.*\bfrom\s+["']nano[_-]?bridge["']"#,
            r#"(?m)^\s*(const|let|var)\s+.*=\s*require\(["']nano[_-]?bridge["']\)"#,
        ],
        invocation_patterns: &[r"\.call_llm\s*\(", r"\.generate\s*\(", r"\.complete\s*\("],
    },
];

#[derive(Debug, Clone, Copy)]
struct LlmProvider {
    name: &'static str,
    import_patterns: &'static [&'static str],
    invocation_patterns: &'static [&'static str],
}

static PATH_LINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?P<path>[A-Za-z0-9_./\\-]+):(?P<line>\d{1,6})").expect("path-line regex")
});
static MODULE_SYMBOL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?P<module>[A-Za-z_][A-Za-z0-9_:.\\/-]*?)::(?P<symbol>[A-Za-z_][A-Za-z0-9_]*)")
        .expect("module-symbol regex")
});
static CLASS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"class\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(\s*(?P<base>[A-Za-z_][A-Za-z0-9_.,\s]*)\s*\)")
        .expect("class regex")
});
static LLM_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\bllm call\b").expect("llm regex"));

pub fn run(args: PromptVerifyArgs) -> Result<()> {
    let repo = canonicalize_repo(&args.repo)?;
    let document_path = resolve_repo_relative_existing(&repo, &args.document)?;
    let document_text = fs::read_to_string(&document_path)
        .with_context(|| format!("read {}", document_path.display()))?;

    let candidates = extract_claims(&document_text);
    let mut claims = Vec::with_capacity(candidates.len());
    let mut verified = 0usize;
    let mut invalid = 0usize;
    let mut review = 0usize;

    for candidate in candidates {
        let (decision, evidence, note) = match candidate.claim_type {
            ClaimType::PathLine => verify_path_line(&repo, &candidate.claim)?,
            ClaimType::ModuleSymbol => verify_module_symbol(&repo, &candidate.claim)?,
            ClaimType::ClassClaim => verify_class_claim(&repo, &candidate.claim)?,
            ClaimType::LlmCall => verify_llm_call(&repo, &candidate.claim)?,
        };

        match decision {
            ClaimDecision::Verified => verified += 1,
            ClaimDecision::Invalid => invalid += 1,
            ClaimDecision::Review => review += 1,
        }

        claims.push(PromptClaim {
            claim_type: claim_type_label(candidate.claim_type).to_string(),
            claim: candidate.claim,
            decision: decision_label(decision).to_string(),
            evidence,
            note,
        });
    }

    let decision = if invalid > 0 {
        "fail"
    } else if review > 0 {
        "review"
    } else {
        "pass"
    };

    let report = PromptVerificationReport {
        schema_version: "1.0.0".to_string(),
        command: "jankurai migrate verify-prompt".to_string(),
        status: "complete".to_string(),
        decision: decision.to_string(),
        repo: repo.display().to_string(),
        document: document_path.display().to_string(),
        claims_total: claims.len(),
        claims_verified: verified,
        claims_invalid: invalid,
        claims_review: review,
        claims,
    };

    if let Some(path) = args.out.as_deref() {
        crate::validation::write_json(
            &repo,
            crate::validation::ArtifactSchema::MigrationPromptVerification,
            path,
            &report,
        )?;
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&report))?;
    }

    if args.strict && invalid > 0 {
        bail!("{} invalid claim(s) detected", invalid);
    }

    Ok(())
}

fn extract_claims(document: &str) -> Vec<ClaimCandidate> {
    let mut claims = Vec::new();
    let mut in_fence = false;
    let mut seen = BTreeSet::new();

    for line in document.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || trimmed.is_empty() || line_is_refutation(trimmed) {
            continue;
        }

        for cap in PATH_LINE_RE.captures_iter(line) {
            let raw = format!("{}:{}", &cap["path"], &cap["line"]);
            if is_extension_like_ref(&cap["path"]) {
                continue;
            }
            if seen.insert(format!("path:{raw}")) {
                claims.push(ClaimCandidate {
                    claim_type: ClaimType::PathLine,
                    claim: raw,
                });
            }
        }

        for cap in MODULE_SYMBOL_RE.captures_iter(line) {
            let raw = format!("{}::{}", &cap["module"], &cap["symbol"]);
            if seen.insert(format!("module:{raw}")) {
                claims.push(ClaimCandidate {
                    claim_type: ClaimType::ModuleSymbol,
                    claim: raw,
                });
            }
        }

        for cap in CLASS_RE.captures_iter(line) {
            let raw = format!("class {}({})", &cap["name"], cap["base"].trim());
            if seen.insert(format!("class:{raw}")) {
                claims.push(ClaimCandidate {
                    claim_type: ClaimType::ClassClaim,
                    claim: raw,
                });
            }
        }

        if LLM_RE.is_match(line) && seen.insert(format!("llm:{}", trimmed)) {
            claims.push(ClaimCandidate {
                claim_type: ClaimType::LlmCall,
                claim: trimmed.to_string(),
            });
        }
    }

    claims
}

fn line_is_refutation(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with('>')
        || (lower.starts_with('|')
            && (lower.contains("false")
                || lower.contains("reality")
                || lower.contains("actually")
                || lower.contains("no llm call")))
}

fn is_extension_like_ref(path: &str) -> bool {
    let normalized = path.trim();
    if normalized.contains('/') {
        return false;
    }
    if normalized.ends_with(".md")
        || normalized.ends_with(".txt")
        || normalized.ends_with(".rs")
        || normalized.ends_with(".py")
        || normalized.ends_with(".ts")
        || normalized.ends_with(".tsx")
        || normalized.ends_with(".js")
        || normalized.ends_with(".jsx")
    {
        return false;
    }
    normalized.contains('.')
}

fn verify_path_line(
    repo: &Path,
    claim: &str,
) -> Result<(ClaimDecision, Vec<String>, Option<String>)> {
    let (path, line) = claim.rsplit_once(':').context("parse path:line claim")?;
    let line_number: usize = line
        .parse()
        .with_context(|| format!("parse line number in `{claim}`"))?;
    let resolved = match resolve_repo_relative_candidate(repo, path) {
        Ok(path) => path,
        Err(err) => {
            return Ok((
                ClaimDecision::Invalid,
                vec![err.to_string()],
                Some("invalid path".to_string()),
            ));
        }
    };
    if !resolved.exists() {
        return Ok((
            ClaimDecision::Invalid,
            vec![format!("no repo-local file matched {}", resolved.display())],
            Some("file missing".to_string()),
        ));
    }
    let canonical = match fs::canonicalize(&resolved) {
        Ok(path) => path,
        Err(err) => {
            return Ok((
                ClaimDecision::Invalid,
                vec![format!("canonicalize {}: {err}", resolved.display())],
                Some("file unreadable".to_string()),
            ));
        }
    };
    if !canonical.starts_with(repo) {
        return Ok((
            ClaimDecision::Invalid,
            vec![format!("path escapes repo root: {}", resolved.display())],
            Some("path escapes repo".to_string()),
        ));
    }
    if canonical.is_dir() {
        return Ok((
            ClaimDecision::Invalid,
            vec![format!("path is a directory: {}", resolved.display())],
            Some("not a file".to_string()),
        ));
    }
    let text = match fs::read_to_string(&canonical) {
        Ok(text) => text,
        Err(err) => {
            return Ok((
                ClaimDecision::Invalid,
                vec![format!("read {}: {err}", canonical.display())],
                Some("file unreadable or non-text".to_string()),
            ));
        }
    };
    let lines: Vec<&str> = text.lines().collect();
    if line_number == 0 || line_number > lines.len() {
        return Ok((
            ClaimDecision::Invalid,
            vec![format!(
                "line {} out of range for {}",
                line_number,
                canonical.display()
            )],
            Some("line not present".to_string()),
        ));
    }
    let line_text = lines[line_number - 1];
    if is_comment_only(line_text) {
        return Ok((
            ClaimDecision::Invalid,
            vec![
                format!("{}:{}", canonical.display(), line_number),
                "line is blank or comment-only".to_string(),
            ],
            Some("comment-only line".to_string()),
        ));
    }
    Ok((
        ClaimDecision::Verified,
        vec![
            format!("{}:{}", canonical.display(), line_number),
            line_text.trim().to_string(),
        ],
        None,
    ))
}

fn verify_module_symbol(
    repo: &Path,
    claim: &str,
) -> Result<(ClaimDecision, Vec<String>, Option<String>)> {
    let (module, symbol) = claim
        .rsplit_once("::")
        .context("parse module::symbol claim")?;
    let module_suffix = module
        .split("::")
        .filter(|segment| {
            !segment.is_empty() && *segment != "crate" && *segment != "self" && *segment != "super"
        })
        .collect::<Vec<_>>()
        .join("/");
    let mut matches = Vec::new();

    for path in candidate_source_files(repo)? {
        let rel = path
            .strip_prefix(repo)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let normalized = rel
            .trim_start_matches("./")
            .trim_end_matches(".rs")
            .trim_end_matches(".ts")
            .trim_end_matches(".tsx")
            .trim_end_matches(".js")
            .trim_end_matches(".jsx")
            .trim_end_matches(".py")
            .trim_end_matches(".java")
            .trim_end_matches(".go")
            .trim_end_matches(".kt")
            .to_string();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let code = strip_obvious_comments(&text);
        let file_matches_module = if module_suffix.is_empty() {
            normalized.ends_with(module)
        } else {
            normalized.ends_with(&module_suffix)
        };
        let file_matches_symbol = symbol_declaration_regex(symbol).is_match(&code);
        if file_matches_module && file_matches_symbol {
            matches.push(format!("{} matches {}::{}", rel, module, symbol));
        }
    }

    match matches.len() {
        0 => Ok((
            ClaimDecision::Invalid,
            vec![format!("no repo-local file matched {}::{}", module, symbol)],
            Some("no exact or suffix match".to_string()),
        )),
        1 => Ok((ClaimDecision::Verified, matches, None)),
        _ => Ok((
            ClaimDecision::Review,
            matches,
            Some("multiple candidate files matched".to_string()),
        )),
    }
}

fn verify_class_claim(
    repo: &Path,
    claim: &str,
) -> Result<(ClaimDecision, Vec<String>, Option<String>)> {
    let cap = CLASS_RE.captures(claim).context("parse class claim")?;
    let class_name = &cap["name"];
    let expected_base = cap["base"].trim();
    let mut matches = Vec::new();
    let mut ambiguous = Vec::new();

    for path in candidate_source_files(repo)? {
        let rel = path
            .strip_prefix(repo)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        match class_match_in_text(&text, class_name, expected_base, &rel) {
            ClassMatch::Verified(evidence) => matches.push(evidence),
            ClassMatch::Ambiguous(evidence) => ambiguous.push(evidence),
            ClassMatch::None => {}
        }
    }

    match matches.len() {
        0 if ambiguous.is_empty() => Ok((
            ClaimDecision::Invalid,
            vec![format!(
                "no class named {} with base {} found",
                class_name, expected_base
            )],
            Some("class declaration not found".to_string()),
        )),
        0 => Ok((
            ClaimDecision::Review,
            ambiguous,
            Some("class-like declaration found, but base relationship is not provable".to_string()),
        )),
        1 => Ok((ClaimDecision::Verified, matches, None)),
        _ => Ok((
            ClaimDecision::Review,
            matches,
            Some("multiple classes matched the claim".to_string()),
        )),
    }
}

fn verify_llm_call(
    repo: &Path,
    claim: &str,
) -> Result<(ClaimDecision, Vec<String>, Option<String>)> {
    let mut matches = Vec::new();
    for path in candidate_source_files(repo)? {
        let rel = path
            .strip_prefix(repo)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let code = strip_obvious_comments(&text);
        for provider in SUPPORTED_LLM_PROVIDERS {
            let import_hit = provider
                .import_patterns
                .iter()
                .any(|pattern| regex(pattern).is_match(&code));
            if !import_hit {
                continue;
            }
            let invocation_lines = invocation_lines(&code, provider);
            for line in invocation_lines {
                if enclosing_scope_ok(&code, line) {
                    matches.push(format!(
                        "{rel}:{} uses {} in scope",
                        line + 1,
                        provider.name
                    ));
                    break;
                }
            }
        }
    }

    match matches.len() {
        0 => Ok((
            ClaimDecision::Invalid,
            vec![format!(
                "no provider import/use plus call site found for `{claim}`"
            )],
            Some("provider import plus invocation missing".to_string()),
        )),
        1 => Ok((ClaimDecision::Verified, matches, None)),
        _ => Ok((
            ClaimDecision::Review,
            matches,
            Some("multiple provider call sites matched".to_string()),
        )),
    }
}

fn candidate_source_files(repo: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(repo)
        .into_iter()
        .filter_entry(|entry| !entry.file_type().is_dir() || !is_skipped_dir(entry.path()))
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if entry.file_type().is_dir() {
            continue;
        }
        if !is_candidate_source(path) {
            continue;
        }
        if is_ignored_path(path) {
            continue;
        }
        files.push(path.to_path_buf());
    }
    Ok(files)
}

fn is_candidate_source(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default(),
        "rs" | "py"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "java"
            | "kt"
            | "go"
            | "mjs"
            | "cjs"
            | "json"
            | "toml"
    )
}

fn is_skipped_dir(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(part) => {
            matches!(
                part.to_string_lossy().as_ref(),
                ".git" | "target" | "node_modules" | "dist" | "build"
            )
        }
        _ => false,
    })
}

fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(part) => {
            matches!(part.to_string_lossy().as_ref(), "generated" | "vendor")
        }
        _ => false,
    })
}

enum ClassMatch {
    Verified(String),
    Ambiguous(String),
    None,
}

fn class_match_in_text(text: &str, class_name: &str, expected_base: &str, rel: &str) -> ClassMatch {
    let code = strip_obvious_comments(text);
    let python = Regex::new(&format!(
        r"(?m)^\s*class\s+{}\s*\(\s*([A-Za-z_][A-Za-z0-9_.,\s]*)\s*\)\s*:",
        regex::escape(class_name)
    ))
    .expect("python class regex");
    if let Some(cap) = python.captures(&code) {
        let bases = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        if bases.split(',').any(|base| base.trim() == expected_base) {
            return ClassMatch::Verified(format!(
                "{rel} matches python class {class_name}({expected_base})"
            ));
        }
        return ClassMatch::None;
    }

    let ts = Regex::new(&format!(
        r"(?m)^\s*class\s+{}\s+extends\s+{}\b",
        regex::escape(class_name),
        regex::escape(expected_base)
    ))
    .expect("typescript class regex");
    if ts.is_match(&code) {
        return ClassMatch::Verified(format!("{rel} matches extends {expected_base}"));
    }

    let impl_trait = Regex::new(&format!(
        r"(?m)^\s*impl(?:<[^>]+>)?\s+{}\s+for\s+{}\b",
        regex::escape(expected_base),
        regex::escape(class_name)
    ))
    .expect("rust impl trait regex");
    if impl_trait.is_match(&code) {
        return ClassMatch::Verified(format!(
            "{rel} matches impl {expected_base} for {class_name}"
        ));
    }

    let type_alias = Regex::new(&format!(
        r"(?m)^\s*(pub\s+)?type\s+{}\s*=\s*{}\b",
        regex::escape(class_name),
        regex::escape(expected_base)
    ))
    .expect("rust type alias regex");
    if type_alias.is_match(&code) {
        return ClassMatch::Verified(format!(
            "{rel} matches type alias {class_name} = {expected_base}"
        ));
    }

    let rustish = Regex::new(&format!(
        r"(?m)^\s*(pub\s+)?(struct|enum|type)\s+{}\b",
        regex::escape(class_name)
    ))
    .expect("rust class-like regex");
    if rustish.is_match(&code) {
        return ClassMatch::Ambiguous(format!(
            "{rel} has Rust-like {class_name}, but no concrete {expected_base} relationship"
        ));
    }

    ClassMatch::None
}

fn invocation_lines(text: &str, provider: &LlmProvider) -> Vec<usize> {
    let mut lines = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if is_comment_only(line) {
            continue;
        }
        if provider
            .invocation_patterns
            .iter()
            .any(|pattern| regex(pattern).is_match(line))
        {
            lines.push(idx);
        }
    }
    lines
}

fn enclosing_scope_ok(text: &str, invocation_line_idx: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let line = lines.get(invocation_line_idx).copied().unwrap_or("");
    let indent = line.chars().take_while(|c| c.is_whitespace()).count();
    if indent == 0 {
        return false;
    }
    for prev in lines[..invocation_line_idx].iter().rev() {
        let trimmed = prev.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        let prev_indent = prev.chars().take_while(|c| c.is_whitespace()).count();
        if prev_indent < indent
            && (trimmed.starts_with("def ")
                || trimmed.starts_with("async def ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("function ")
                || trimmed.starts_with("export function ")
                || trimmed.starts_with("const ")
                || trimmed.starts_with("let "))
        {
            return true;
        }
    }
    false
}

fn is_comment_only(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }
    let prefixes = ["//", "#", "/*", "*", "--", ";", "%", "<!--"];
    prefixes.iter().any(|prefix| trimmed.starts_with(prefix))
}

fn symbol_declaration_regex(symbol: &str) -> Regex {
    Regex::new(&format!(
        r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:fn|struct|enum|trait|impl|const|static|mod|type)\s+{}\b|^\s*(?:async\s+)?def\s+{}\b|^\s*class\s+{}\b|^\s*(?:export\s+)?(?:async\s+)?function\s+{}\b|^\s*(?:export\s+)?(?:const|let|var)\s+{}\b|^\s*import\s+.*\b{}\b|^\s*(?:from\s+\S+\s+)?import\s+.*\b{}\b|^\s*(?:pub\s+)?use\s+.*\b{}\b",
        regex::escape(symbol),
        regex::escape(symbol),
        regex::escape(symbol),
        regex::escape(symbol),
        regex::escape(symbol),
        regex::escape(symbol),
        regex::escape(symbol),
        regex::escape(symbol)
    ))
    .expect("symbol declaration regex")
}

fn strip_obvious_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
        {
            out.push('\n');
            continue;
        }
        let without_comment = line
            .split_once("//")
            .map(|(code, _)| code)
            .unwrap_or(line)
            .split_once('#')
            .map(|(code, _)| code)
            .unwrap_or_else(|| line.split_once('#').map(|(code, _)| code).unwrap_or(line));
        out.push_str(without_comment);
        out.push('\n');
    }
    out
}

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid regex")
}

fn decision_label(decision: ClaimDecision) -> &'static str {
    match decision {
        ClaimDecision::Verified => "verified",
        ClaimDecision::Invalid => "invalid",
        ClaimDecision::Review => "review",
    }
}

fn claim_type_label(claim_type: ClaimType) -> &'static str {
    match claim_type {
        ClaimType::PathLine => "path-line",
        ClaimType::ModuleSymbol => "module-symbol",
        ClaimType::ClassClaim => "class",
        ClaimType::LlmCall => "llm-call",
    }
}

fn render_markdown(report: &PromptVerificationReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Migration Prompt Verification");
    let _ = writeln!(out);
    let _ = writeln!(out, "- repo: `{}`", report.repo);
    let _ = writeln!(out, "- document: `{}`", report.document);
    let _ = writeln!(out, "- decision: `{}`", report.decision);
    let _ = writeln!(out, "- claims total: `{}`", report.claims_total);
    let _ = writeln!(out, "- verified: `{}`", report.claims_verified);
    let _ = writeln!(out, "- invalid: `{}`", report.claims_invalid);
    let _ = writeln!(out, "- review: `{}`", report.claims_review);
    let _ = writeln!(out);
    for claim in &report.claims {
        let _ = writeln!(out, "## `{}`", claim.claim);
        let _ = writeln!(out, "- type: `{}`", claim.claim_type);
        let _ = writeln!(out, "- decision: `{}`", claim.decision);
        if !claim.evidence.is_empty() {
            let _ = writeln!(out, "- evidence: `{}`", claim.evidence.join(" | "));
        }
        if let Some(note) = &claim.note {
            let _ = writeln!(out, "- note: {note}");
        }
        let _ = writeln!(out);
    }
    out
}

fn canonicalize_repo(repo: &Path) -> Result<PathBuf> {
    fs::canonicalize(repo).with_context(|| format!("canonicalize {}", repo.display()))
}

fn resolve_repo_relative_candidate(repo: &Path, value: &str) -> Result<PathBuf> {
    let normalized = normalize_rel(value)?;
    Ok(repo.join(normalized))
}

fn resolve_repo_relative_existing(repo: &Path, value: &str) -> Result<PathBuf> {
    let resolved = resolve_repo_relative_candidate(repo, value)?;
    let canonical = fs::canonicalize(&resolved)
        .with_context(|| format!("canonicalize {}", resolved.display()))?;
    if !canonical.starts_with(repo) {
        bail!("path escapes repo root: `{value}`");
    }
    Ok(canonical)
}

fn normalize_rel(path: &str) -> Result<PathBuf> {
    if path.trim().is_empty() {
        bail!("path must not be empty");
    }
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        bail!("absolute paths are not allowed: `{path}`");
    }
    let mut parts = Vec::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("path traversal is not allowed: `{path}`");
            }
        }
    }
    if parts.is_empty() {
        bail!("path must not be empty");
    }
    Ok(PathBuf::from(parts.join("/")))
}
