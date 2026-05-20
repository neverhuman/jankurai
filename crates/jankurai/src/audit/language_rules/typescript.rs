use super::catalog::{
    ConfidencePolicy, Language, LanguageFinding, LanguageRule, Matcher, ProofWindow,
};
use crate::audit::helpers::AuditContext;
use crate::audit::scan;
use crate::model::FileInfo;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value as JsonValue;
use std::collections::BTreeSet;

const HLT_RULE_ID: &str = "HLT-031-TYPESCRIPT-BAD-BEHAVIOR";

const HARD_RULES: &[LanguageRule] = &[
    LanguageRule {
        id: "typescript.suppress.ts-nocheck",
        language: Language::TypeScript,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "boundary",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["@ts-nocheck", "@ts-ignore", "eslint-disable"]),
        proof_window: ProofWindow::None,
        problem: "TypeScript suppression comment hides type checking or lint evidence",
        fix: "remove the broad suppression or scope it to a single justified line",
    },
    LanguageRule {
        id: "typescript.types.any-boundary",
        language: Language::TypeScript,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "boundary",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "as any",
            "as unknown as",
            "json.parse(",
            "response.json(",
            "req.body",
        ]),
        proof_window: ProofWindow::None,
        problem: "unchecked boundary cast or parse result crosses a trust boundary",
        fix: "validate the value first, then narrow it with a proof-aware decoder",
    },
    LanguageRule {
        id: "typescript.config.strict-disabled",
        language: Language::TypeScript,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "boundary",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "\"strict\": false",
            "\"strictnullchecks\": false",
            "\"noemitonerror\": false",
        ]),
        proof_window: ProofWindow::None,
        problem: "TypeScript compiler strictness is disabled in a repo config file",
        fix: "restore strict compiler settings and narrow the exception to a local test fixture",
    },
    LanguageRule {
        id: "typescript.runtime.dangerous-eval-dom",
        language: Language::TypeScript,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "eval(",
            "new function(",
            "dangerouslysetinnerhtml",
            ".innerhtml =",
            "innerhtml =",
        ]),
        proof_window: ProofWindow::None,
        problem: "dynamic code or raw HTML sink appears in product TypeScript",
        fix: "replace the dynamic sink with a bounded parser, sanitizer, or typed renderer",
    },
    LanguageRule {
        id: "typescript.security.raw-command-sql",
        language: Language::TypeScript,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "fast",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["exec(", "spawn(", ".query(", ".execute(", ".raw("]),
        proof_window: ProofWindow::None,
        problem: "raw shell or SQL text is built from untrusted TypeScript input",
        fix: "use argv arrays, prepared statements, or a safe allowlisted command path",
    },
];

pub fn catalog() -> &'static [LanguageRule] {
    HARD_RULES
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TypeScriptSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn summary(ctx: &AuditContext) -> TypeScriptSummary {
    TypeScriptSummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: advisory_signals(ctx).len(),
    }
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    sort_and_cap_findings(hard_findings(ctx), 50)
}

pub fn advisory_signals(ctx: &AuditContext) -> Vec<LanguageFinding> {
    sort_and_cap_findings(advisory_hits(ctx), 50)
}

fn hard_findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in typescript_files(ctx) {
        if is_ts_config(file) {
            out.extend(tsconfig_hard_hits(file));
            continue;
        }
        out.extend(ts_source_hard_hits(file));
    }
    out
}

fn advisory_hits(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in typescript_files(ctx) {
        if is_ts_config(file) {
            out.extend(tsconfig_advisory_hits(file));
            continue;
        }
        out.extend(ts_source_advisory_hits(file));
    }
    out
}

fn typescript_files(ctx: &AuditContext) -> Vec<&FileInfo> {
    let zone_paths = crate::audit::helpers::generated_zone_suppression_paths(ctx);
    ctx.all_files
        .iter()
        .filter(|file| is_typescript_surface(file, &zone_paths))
        .collect()
}

fn is_typescript_surface(file: &FileInfo, generated_zone_paths: &[String]) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    if scan::is_generated_or_reference_path(&file.rel_path)
        || scan::is_test_or_example_path(&file.rel_path)
        || lower.starts_with("fixtures/")
        || lower.contains("/fixtures/")
    {
        return false;
    }
    if generated_zone_paths
        .iter()
        .any(|zone| crate::audit::helpers::path_matches_prefix(&file.rel_path, zone))
    {
        return false;
    }
    lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".mts")
        || lower.ends_with(".cts")
        || is_ts_config(file)
}

fn is_ts_config(file: &FileInfo) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    lower.ends_with("tsconfig.json") || lower.starts_with("tsconfig.") && lower.ends_with(".json")
}

fn ts_source_hard_hits(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for (idx, line) in file.text.lines().enumerate() {
        let line_no = idx + 1;
        let lower = line.to_ascii_lowercase();
        if lower.contains("@ts-nocheck")
            || lower.contains("@ts-ignore")
            || lower.contains("eslint-disable")
        {
            out.push(finding(
                HLT_RULE_ID,
                "typescript.suppress.ts-nocheck",
                file,
                line_no,
                "TypeScript suppression comment hides type checking or lint evidence",
                "broad suppression is hard to audit",
                "remove the broad suppression or scope it to a single justified line",
            ));
        }
        if casts_at_trust_boundary(&lower) {
            out.push(finding(
                HLT_RULE_ID,
                "typescript.types.any-boundary",
                file,
                line_no,
                "unchecked boundary cast or parse result crosses a trust boundary",
                "value shape is not proven before the cast",
                "validate the value first, then narrow it with a proof-aware decoder",
            ));
        }
        if dangerous_eval_or_html(&lower) {
            out.push(finding(
                HLT_RULE_ID,
                "typescript.runtime.dangerous-eval-dom",
                file,
                line_no,
                "dynamic code or raw HTML sink appears in product TypeScript",
                "sink is not proven safe locally",
                "replace the dynamic sink with a bounded parser, sanitizer, or typed renderer",
            ));
        }
        if raw_shell_or_sql(&lower) {
            out.push(finding(
                HLT_RULE_ID,
                "typescript.security.raw-command-sql",
                file,
                line_no,
                "raw shell or SQL text is built from untrusted TypeScript input",
                "trusted input proof is missing",
                "use argv arrays, prepared statements, or a safe allowlisted command path",
            ));
        }
    }
    out
}

fn tsconfig_hard_hits(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    if let Ok(parsed) = serde_json::from_str::<JsonValue>(&file.text) {
        if let Some(options) = parsed.get("compilerOptions").and_then(JsonValue::as_object) {
            for (key, detector_id, _matched_term, problem, reason, fix) in [
                (
                    "strict",
                    "typescript.config.strict-disabled",
                    "strict",
                    "TypeScript compiler strictness is disabled in a repo config file",
                    "strict mode is explicitly off",
                    "restore strict compiler settings and narrow the exception to a local test fixture",
                ),
                (
                    "strictNullChecks",
                    "typescript.config.strict-disabled",
                    "strictNullChecks",
                    "TypeScript compiler strictness is disabled in a repo config file",
                    "nullability discipline is explicitly off",
                    "restore strict compiler settings and narrow the exception to a local test fixture",
                ),
                (
                    "noEmitOnError",
                    "typescript.config.strict-disabled",
                    "noEmitOnError",
                    "TypeScript compiler strictness is disabled in a repo config file",
                    "compiler emits on errors",
                    "restore strict compiler settings and narrow the exception to a local test fixture",
                ),
            ] {
                if options.get(key).and_then(JsonValue::as_bool) == Some(false) {
                    if let Some(line_no) = json_key_line(&file.text, key, false) {
                        out.push(finding(
                            HLT_RULE_ID,
                            detector_id,
                            file,
                            line_no,
                            problem,
                            reason,
                            fix,
                        ));
                    }
                }
            }
        }
    } else {
        for (idx, line) in file.text.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            if lower.contains("\"strict\": false")
                || lower.contains("\"strictnullchecks\": false")
                || lower.contains("\"noemitonerror\": false")
            {
                out.push(finding(
                    HLT_RULE_ID,
                    "typescript.config.strict-disabled",
                    file,
                    idx + 1,
                    "TypeScript compiler strictness is disabled in a repo config file",
                    "strict compiler settings are off",
                    "restore strict compiler settings and narrow the exception to a local test fixture",
                ));
            }
        }
    }
    out
}

fn ts_source_advisory_hits(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for (idx, line) in file.text.lines().enumerate() {
        let line_no = idx + 1;
        let lower = line.to_ascii_lowercase();
        if NON_NULL_ASSERTION_RE.is_match(line) {
            out.push(finding(
                HLT_RULE_ID,
                "typescript.review.non-null-assertion",
                file,
                line_no,
                "non-null assertion deserves a proof check",
                "nullability proof is review-worthy",
                "prefer an explicit guard or a decoded value instead of a non-null assertion",
            ));
        }
        if lower.contains("partial<") {
            out.push(finding(
                HLT_RULE_ID,
                "typescript.review.partial-patch",
                file,
                line_no,
                "Partial<T> patch shape deserves a proof check",
                "patch semantics are review-worthy",
                "model the patch shape explicitly or validate the object before merging it",
            ));
        }
    }
    out
}

fn tsconfig_advisory_hits(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    if let Ok(parsed) = serde_json::from_str::<JsonValue>(&file.text) {
        if let Some(options) = parsed.get("compilerOptions").and_then(JsonValue::as_object) {
            if options.get("skipLibCheck").and_then(JsonValue::as_bool) == Some(true) {
                if let Some(line_no) = json_key_line(&file.text, "skipLibCheck", true) {
                    out.push(finding(
                        HLT_RULE_ID,
                        "typescript.review.skip-lib-check",
                        file,
                        line_no,
                        "skipLibCheck is enabled in a repo config file",
                        "library type checking is review-worthy",
                        "remove the override or justify it in a local test-only config",
                    ));
                }
            }
        }
    }
    out
}

fn casts_at_trust_boundary(lower: &str) -> bool {
    let boundary_markers = [
        "req.body",
        "request.body",
        "response.json(",
        "fetch(",
        "json.parse(",
        "process.env",
        "params",
        "query",
        "input",
    ];
    let cast_markers = [" as any", " as unknown as", " as "];
    cast_markers.iter().any(|cast| lower.contains(cast))
        && boundary_markers.iter().any(|marker| lower.contains(marker))
}

fn dangerous_eval_or_html(lower: &str) -> bool {
    if lower.contains("dangerouslysetinnerhtml") {
        return !lower.contains("sanitize")
            && !lower.contains("dompurify")
            && !lower.contains("trusted");
    }
    if lower.contains(".innerhtml =") || lower.contains("innerhtml =") {
        return !lower.contains("sanitize")
            && !lower.contains("dompurify")
            && !lower.contains("trusted");
    }
    lower.contains("eval(") || lower.contains("new function(")
}

fn raw_shell_or_sql(lower: &str) -> bool {
    let untrusted = [
        "req.",
        "request.",
        "body",
        "params",
        "query",
        "process.env",
        "user",
        "input",
        "command",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    let interpolated = lower.contains("${") || lower.contains('+');
    let shell = lower.contains("exec(")
        || lower.contains("spawn(")
        || lower.contains("execsync(")
        || lower.contains("spawnsync(");
    let sql = lower.contains(".query(")
        || lower.contains(".execute(")
        || lower.contains(".raw(")
        || lower.contains("select ")
        || lower.contains("update ")
        || lower.contains("delete ")
        || lower.contains("insert ")
        || lower.contains("drop ");
    (shell || sql) && untrusted && interpolated
}

fn json_key_line(text: &str, key: &str, value: bool) -> Option<usize> {
    let needle = format!("\"{key}\": {value}");
    let lower_needle = needle.to_ascii_lowercase();
    text.lines().enumerate().find_map(|(idx, line)| {
        if line.to_ascii_lowercase().contains(&lower_needle) {
            Some(idx + 1)
        } else {
            None
        }
    })
}

fn sort_and_cap_findings(mut findings: Vec<LanguageFinding>, max: usize) -> Vec<LanguageFinding> {
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

fn finding(
    rule_id: &'static str,
    detector_id: &'static str,
    file: &FileInfo,
    line_no: usize,
    problem: &str,
    reason: &str,
    fix: &str,
) -> LanguageFinding {
    let snippet = file
        .text
        .lines()
        .nth(line_no.saturating_sub(1))
        .map(|line| line.trim().chars().take(160).collect::<String>())
        .unwrap_or_default();
    LanguageFinding::new(
        rule_id,
        detector_id,
        file.rel_path.clone(),
        Some(line_no),
        snippet.clone(),
        problem,
        reason,
        fix,
        vec![
            format!("detector={detector_id}"),
            format!("path={}", file.rel_path),
            format!("line={line_no}"),
            format!("snippet={snippet}"),
        ],
    )
}

static NON_NULL_ASSERTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[A-Za-z_$][A-Za-z0-9_$]*!\s*(?:\.|\[|\(|;|,|\)|\}|:|$)")
        .expect("non-null assertion regex is valid")
});
