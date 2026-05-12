use super::catalog::{
    ConfidencePolicy, Language, LanguageFinding, LanguageRule, Matcher, ProofWindow,
};
use super::common::{
    contains_unqualified_python_builtin_call, finding, is_docs_reference_tips_or_generated,
    is_test_fixture_or_example, python_code_lines, sort_and_cap_findings,
};
use crate::audit::helpers::{python_scoring_exempt, AuditContext};
use crate::model::FileInfo;

const HLT_RULE_ID: &str = "HLT-033-PYTHON-BAD-BEHAVIOR";
const DETECTOR_DYNAMIC_CODE: &str = "python.exec.dynamic-code";
const DETECTOR_UNSAFE_DESER: &str = "python.deser.unsafe-object";
const DETECTOR_SHELL_DYNAMIC: &str = "python.shell.dynamic";
const DETECTOR_SQL_STRING_BUILT: &str = "python.sql.string-built";
const DETECTOR_TLS_DEBUG: &str = "python.net.tls-debug";
const DETECTOR_BROAD_EXCEPT: &str = "python.review.broad-except";

const RULES: &[LanguageRule] = &[
    LanguageRule {
        id: DETECTOR_DYNAMIC_CODE,
        language: Language::Python,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "contract",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["eval(", "exec(", "compile(", "exec(open("]),
        proof_window: ProofWindow::None,
        problem: "dynamic code execution runs attacker-controlled source directly",
        fix: "replace dynamic execution with a parser, dispatch table, or typed plugin boundary",
    },
    LanguageRule {
        id: DETECTOR_UNSAFE_DESER,
        language: Language::Python,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "contract",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "pickle.load",
            "pickle.loads",
            "dill.load",
            "dill.loads",
            "cloudpickle.load",
            "cloudpickle.loads",
            "marshal.load",
            "marshal.loads",
            "shelve.open",
            "joblib.load",
            "yaml.load(",
        ]),
        proof_window: ProofWindow::None,
        problem: "unsafe deserialisation can instantiate attacker-controlled objects",
        fix: "use `safe_load` or a schema-based decoder for untrusted input",
    },
    LanguageRule {
        id: DETECTOR_SHELL_DYNAMIC,
        language: Language::Python,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "contract",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["os.system(", "os.popen(", "shell=True"]),
        proof_window: ProofWindow::None,
        problem: "shell=True or os.system lets input reach a shell",
        fix: "pass argv arrays to subprocess and keep shell off",
    },
    LanguageRule {
        id: DETECTOR_SQL_STRING_BUILT,
        language: Language::Python,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "contract",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["execute(", "executemany(", "query("]),
        proof_window: ProofWindow::None,
        problem: "string-built SQL reaches a database sink without parameter binding",
        fix: "parameterize the statement or move identifier handling through an allowlist",
    },
    LanguageRule {
        id: DETECTOR_TLS_DEBUG,
        language: Language::Python,
        hlt_rule_id: HLT_RULE_ID,
        severity: "high",
        category: "security",
        lane: "contract",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "verify=False",
            "disable_warnings",
            "_create_unverified_context",
        ]),
        proof_window: ProofWindow::None,
        problem: "certificate verification is disabled in runtime code",
        fix: "remove the debug bypass and pin a trusted CA bundle",
    },
    LanguageRule {
        id: DETECTOR_BROAD_EXCEPT,
        language: Language::Python,
        hlt_rule_id: HLT_RULE_ID,
        severity: "medium",
        category: "advisory",
        lane: "contract",
        confidence: ConfidencePolicy::Low,
        matcher: Matcher::ContainsAny(&["except exception", "except baseexception"]),
        proof_window: ProofWindow::None,
        problem: "broad exception handling hides real failures and control flow",
        fix: "catch specific exceptions and keep the failure surface explicit",
    },
];

#[derive(Debug, Clone, Copy, Default)]
pub struct PythonSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn catalog() -> &'static [LanguageRule] {
    RULES
}

pub fn summary(ctx: &AuditContext) -> PythonSummary {
    PythonSummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: advisory_signals(ctx).len(),
    }
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    sort_and_cap_findings(hard_findings(ctx), 50)
}

pub fn advisory_signals(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in python_files(ctx) {
        for (idx, line) in python_code_lines(&file.text) {
            if let Some(hit) = advisory_hit_for_line(&file, idx, &line) {
                out.push(hit);
            }
        }
    }
    sort_and_cap_findings(out, 50)
}

fn hard_findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in python_files(ctx) {
        for (idx, line) in python_code_lines(&file.text) {
            if let Some(hit) = hard_hit_for_line(&file, idx, &line) {
                out.push(hit);
            }
        }
    }
    out
}

fn python_files(ctx: &AuditContext) -> Vec<FileInfo> {
    ctx.all_files
        .iter()
        .filter(|file| is_python_candidate(ctx, file))
        .cloned()
        .collect()
}

fn is_python_candidate(ctx: &AuditContext, file: &FileInfo) -> bool {
    let rel = file.rel_path.to_ascii_lowercase();
    !file.is_generated
        && !is_docs_reference_tips_or_generated(&rel)
        && !is_test_fixture_or_example(&rel)
        && !python_scoring_exempt(ctx, &rel)
        && matches!(file.suffix.as_str(), ".py" | ".pyi")
}

fn hard_hit_for_line(file: &FileInfo, line_no: usize, line: &str) -> Option<LanguageFinding> {
    let normalized = normalize_python_line(line)?;
    let lower = normalized.to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }

    if is_dynamic_code_line(&lower) {
        return Some(finding(
            HLT_RULE_ID,
            DETECTOR_DYNAMIC_CODE,
            file,
            line_no,
            "dynamic code execution runs attacker-controlled source directly",
            "the line executes text as code instead of keeping it as data",
            "replace dynamic execution with a parser, dispatch table, or typed plugin boundary",
            ProofWindow::None,
        ));
    }

    if is_unsafe_deserialisation_line(&lower) {
        return Some(finding(
            HLT_RULE_ID,
            DETECTOR_UNSAFE_DESER,
            file,
            line_no,
            "unsafe deserialisation can instantiate attacker-controlled objects",
            "a loader accepts data that can control object construction",
            "use `safe_load` or a schema-based decoder for untrusted input",
            ProofWindow::None,
        ));
    }

    if is_shell_dynamic_line(&lower) {
        return Some(finding(
            HLT_RULE_ID,
            DETECTOR_SHELL_DYNAMIC,
            file,
            line_no,
            "shell=True or os.system lets input reach a shell",
            "command execution is routed through a shell boundary",
            "pass argv arrays to subprocess and keep shell off",
            ProofWindow::None,
        ));
    }

    if is_sql_string_built_line(&lower) {
        return Some(finding(
            HLT_RULE_ID,
            DETECTOR_SQL_STRING_BUILT,
            file,
            line_no,
            "string-built SQL reaches a database sink without parameter binding",
            "the SQL text is assembled inline before the execute/query call",
            "parameterize the statement or move identifier handling through an allowlist",
            ProofWindow::None,
        ));
    }

    if is_tls_debug_line(&lower) {
        return Some(finding(
            HLT_RULE_ID,
            DETECTOR_TLS_DEBUG,
            file,
            line_no,
            "certificate verification is disabled in runtime code",
            "the request path bypasses TLS verification or trust checks",
            "remove the debug bypass and pin a trusted CA bundle",
            ProofWindow::None,
        ));
    }

    None
}

fn advisory_hit_for_line(file: &FileInfo, line_no: usize, line: &str) -> Option<LanguageFinding> {
    let normalized = normalize_python_line(line)?;
    let lower = normalized.to_ascii_lowercase();
    if lower.contains("except exception") || lower.contains("except baseexception") {
        return Some(finding(
            HLT_RULE_ID,
            DETECTOR_BROAD_EXCEPT,
            file,
            line_no,
            "broad exception handling hides real failures and control flow",
            "the catch-all scope can swallow unrelated errors",
            "catch specific exceptions and keep the failure surface explicit",
            ProofWindow::None,
        ));
    }
    None
}

fn is_dynamic_code_line(lower: &str) -> bool {
    ["eval", "exec", "compile"]
        .iter()
        .any(|builtin| contains_unqualified_python_builtin_call(lower, builtin))
        || lower.contains("exec(open(")
}

fn is_unsafe_deserialisation_line(lower: &str) -> bool {
    if lower.contains("yaml.load(") {
        return !lower.contains("safe_load") && !lower.contains("safeloader");
    }
    [
        "pickle.load",
        "pickle.loads",
        "dill.load",
        "dill.loads",
        "cloudpickle.load",
        "cloudpickle.loads",
        "marshal.load",
        "marshal.loads",
        "shelve.open",
        "joblib.load",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_shell_dynamic_line(lower: &str) -> bool {
    lower.contains("shell=true")
        || lower.contains("os.system(")
        || lower.contains("os.popen(")
        || (lower.contains("subprocess.") && lower.contains("shell=true"))
}

fn is_sql_string_built_line(lower: &str) -> bool {
    let has_sink =
        lower.contains("execute(") || lower.contains("executemany(") || lower.contains("query(");
    let has_sql = lower.contains("select ")
        || lower.contains("insert ")
        || lower.contains("update ")
        || lower.contains("delete ")
        || lower.contains("drop ")
        || lower.contains("truncate ");
    let has_dynamic = lower.contains("f\"")
        || lower.contains("f'")
        || lower.contains(".format(")
        || lower.contains(" + ")
        || lower.contains("${");
    has_sink && has_sql && has_dynamic
}

fn is_tls_debug_line(lower: &str) -> bool {
    lower.contains("verify=false")
        || lower.contains("disable_warnings")
        || lower.contains("_create_unverified_context")
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::audit::helpers::AuditContext;
    use tempfile::tempdir;

    fn file_info(rel_path: &str, text: &str) -> FileInfo {
        let name = std::path::Path::new(rel_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let suffix = std::path::Path::new(rel_path)
            .extension()
            .map(|ext| format!(".{}", ext.to_string_lossy()))
            .unwrap_or_default();
        FileInfo {
            rel_path: rel_path.into(),
            name,
            suffix,
            size: text.len() as u64,
            line_count: text.lines().count(),
            text: text.into(),
            is_generated: false,
            is_code: true,
        }
    }

    fn ctx_with_files(files: Vec<FileInfo>) -> AuditContext {
        let root = tempdir().unwrap();
        AuditContext {
            root: root.path().to_path_buf(),
            all_files: files.clone(),
            scope_files: files,
            scope_paths: vec![],
            self_audit: false,
            boundary_reclassifications: vec![],
            copy_code: None,
        }
    }

    #[test]
    fn risky_python_snippets_emit_hlt033_findings() {
        let ctx = ctx_with_files(vec![
            file_info("src/dynamic_code.py", "eval(payload)\n"),
            file_info("src/unsafe_deser.py", "pickle.loads(data)\n"),
            file_info("src/shell_dynamic.py", "subprocess.run(cmd, shell=True)\n"),
            file_info(
                "src/sql_string_built.py",
                "cursor.execute(f\"SELECT * FROM users WHERE id={user_id}\")\n",
            ),
            file_info("src/tls_debug.py", "requests.get(url, verify=False)\n"),
        ]);

        let findings = findings(&ctx);
        assert_eq!(findings.len(), 5, "{findings:?}");
        assert!(findings
            .iter()
            .any(|finding| finding.path == "src/dynamic_code.py"));
        assert!(findings
            .iter()
            .any(|finding| finding.path == "src/unsafe_deser.py"));
        assert!(findings
            .iter()
            .any(|finding| finding.path == "src/shell_dynamic.py"));
        assert!(findings
            .iter()
            .any(|finding| finding.path == "src/sql_string_built.py"));
        assert!(findings
            .iter()
            .any(|finding| finding.path == "src/tls_debug.py"));
    }

    #[test]
    fn safe_python_snippets_emit_no_hlt033_findings() {
        let ctx = ctx_with_files(vec![
            file_info(
                "src/parameterized.py",
                "cursor.execute(query, (user_id,))\n",
            ),
            file_info("src/safe_deser.py", "yaml.safe_load(data)\n"),
            file_info(
                "src/safe_shell.py",
                "subprocess.run([\"git\", \"status\"], check=True)\n",
            ),
            file_info("src/secure_tls.py", "requests.get(url, verify=True)\n"),
        ]);

        assert!(findings(&ctx).is_empty());
        assert!(advisory_signals(&ctx).is_empty());
    }

    #[test]
    fn persistent_repo_python_files_are_absent_outside_allowed_roots() {
        let output = std::process::Command::new("git")
            .args([
                "ls-files",
                "*.py",
                ":!:reference/**",
                ":!:target/**",
                ":!:tests/fixtures/**",
                ":!:crates/jankurai/tests/fixtures/**",
            ])
            .output()
            .expect("run git ls-files");
        assert!(output.status.success(), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stdout).trim().is_empty(),
            "tracked .py files remain outside allowed roots:\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

fn normalize_python_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('#') || trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
        return None;
    }
    let without_trailing_comment = trimmed.split('#').next().unwrap_or(trimmed).trim();
    if without_trailing_comment.is_empty() {
        return None;
    }
    Some(
        without_trailing_comment
            .trim_end_matches(';')
            .trim()
            .to_string(),
    )
}
