use super::catalog::{ConfidencePolicy, Language, LanguageRule, Matcher, ProofWindow};
use super::common::{
    finding, is_docs_reference_tips_or_generated, is_test_fixture_or_example, nearby_allow,
    sort_and_cap_findings, strip_comments_for_line_language,
};
use super::LanguageFinding;
use crate::audit::helpers::AuditContext;
use crate::model::FileInfo;
use serde_json::Value as JsonValue;
use std::collections::BTreeSet;

const RULE_ID: &str = "HLT-036-GITTOOLS-BAD-BEHAVIOR";

const HARD_RULES: &[LanguageRule] = &[
    LanguageRule {
        id: "gittools.bypass.no-verify-automation",
        language: Language::GitTools,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "git commit --no-verify",
            "git push --no-verify",
            "--no-verify",
        ]),
        proof_window: ProofWindow::None,
        problem: "hook or release automation normalizes --no-verify",
        fix: "remove the routine bypass and keep emergency exceptions explicit, reviewed, and CI-covered",
    },
    LanguageRule {
        id: "gittools.hooks-path.disabled",
        language: Language::GitTools,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["core.hooksPath /dev/null", "core.hookspath=/dev/null"]),
        proof_window: ProofWindow::None,
        problem: "checked-in automation disables Git hooks",
        fix: "remove the disabled hooksPath setting or replace it with a versioned repo-local hook path",
    },
    LanguageRule {
        id: "gittools.raw-hooks.unversioned-install",
        language: Language::GitTools,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[".git/hooks"]),
        proof_window: ProofWindow::None,
        problem: "team hook installer mutates unversioned .git/hooks files",
        fix: "use a committed hook directory or hook manager and document the install/update flow",
    },
    LanguageRule {
        id: "gittools.hook.destructive-git-command",
        language: Language::GitTools,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["git reset --hard", "git clean -fd", "git push --force"]),
        proof_window: ProofWindow::None,
        problem: "hook automation performs destructive Git mutation",
        fix: "remove destructive mutation from hooks and move any reviewed repair flow to an explicit command",
    },
    LanguageRule {
        id: "gittools.hook.unbounded-stage",
        language: Language::GitTools,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["git add .", "git add -A", "git commit -am"]),
        proof_window: ProofWindow::None,
        problem: "hook automation stages the whole worktree",
        fix: "stage only explicit paths or rely on the hook manager's staged-file handling",
    },
    LanguageRule {
        id: "gittools.lint-staged.manual-restage",
        language: Language::GitTools,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["git add .", "git add -A"]),
        proof_window: ProofWindow::None,
        problem: "lint-staged command manually restages project-wide changes",
        fix: "let lint-staged manage the index and remove project-wide git add commands",
    },
];

#[derive(Debug, Clone, Copy, Default)]
pub struct GitToolsSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn summary(ctx: &AuditContext) -> GitToolsSummary {
    GitToolsSummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: advisory_signals(ctx),
    }
}

pub fn catalog() -> &'static [LanguageRule] {
    HARD_RULES
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in gittools_files(ctx) {
        out.extend(findings_for_file(&file));
    }
    sort_and_cap_findings(out, 50)
}

pub fn is_gittools_owned_surface(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with(".husky/")
        || lower.starts_with(".githooks/")
        || lower.starts_with("hooks/")
        || lower.starts_with("tools/jankurai-hooks/")
        || is_hook_manager_config(&lower)
}

fn gittools_files(ctx: &AuditContext) -> Vec<FileInfo> {
    ctx.all_files
        .iter()
        .filter(|file| is_gittools_file(file))
        .cloned()
        .collect()
}

fn is_gittools_file(file: &FileInfo) -> bool {
    if is_docs_reference_tips_or_generated(&file.rel_path)
        || is_test_fixture_or_example(&file.rel_path)
    {
        return false;
    }
    let lower = file.rel_path.to_ascii_lowercase();
    is_gittools_owned_surface(&lower)
        || lower == "package.json"
        || ((lower.ends_with("makefile")
            || lower.ends_with("justfile")
            || lower.starts_with("scripts/")
            || lower.starts_with(".github/workflows/")
            || lower == ".gitlab-ci.yml")
            && configures_hook_tooling(&file.text))
}

fn is_hook_manager_config(lower: &str) -> bool {
    matches!(
        lower,
        ".pre-commit-config.yaml"
            | ".pre-commit-config.yml"
            | "lefthook.yml"
            | "lefthook.yaml"
            | ".lefthook.yml"
            | ".overcommit.yml"
            | "overcommit.yml"
            | "captainhook.json"
    ) || lower.starts_with(".lintstagedrc")
        || lower.starts_with("commitlint.config.")
}

fn configures_hook_tooling(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "core.hookspath",
        "pre-commit install",
        "husky install",
        "lefthook install",
        "overcommit --install",
        "captainhook install",
        ".git/hooks",
        ".husky/",
        ".githooks/",
        "lint-staged",
        "commitlint",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn findings_for_file(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    if file.rel_path.eq_ignore_ascii_case("package.json") {
        out.extend(package_json_findings(file, &mut seen));
        return sort_and_cap_findings(out, 50);
    }

    let file_kind = line_kind(file);
    let in_lint_staged = is_lint_staged_surface(&file.rel_path);
    for (line_no, raw_line) in file.text.lines().enumerate() {
        let line = strip_comments_for_line_language(raw_line, file_kind);
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        emit_line_findings(
            file,
            line_no + 1,
            &lower,
            in_lint_staged,
            &mut out,
            &mut seen,
        );
    }
    sort_and_cap_findings(out, 50)
}

fn package_json_findings(
    file: &FileInfo,
    seen: &mut BTreeSet<(String, usize, &'static str)>,
) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    let Ok(parsed) = serde_json::from_str::<JsonValue>(&file.text) else {
        return out;
    };
    for key in ["scripts", "husky", "lint-staged"] {
        if let Some(value) = parsed.get(key) {
            emit_json_value_findings(file, key, value, key == "lint-staged", &mut out, seen);
        }
    }
    if let Some(value) = parsed.pointer("/config/commitizen") {
        emit_json_value_findings(file, "commitizen", value, false, &mut out, seen);
    }
    out
}

fn emit_json_value_findings(
    file: &FileInfo,
    key: &str,
    value: &JsonValue,
    in_lint_staged: bool,
    out: &mut Vec<LanguageFinding>,
    seen: &mut BTreeSet<(String, usize, &'static str)>,
) {
    match value {
        JsonValue::String(command) => {
            let line = find_line(file, command)
                .or_else(|| find_line(file, key))
                .unwrap_or(1);
            emit_line_findings(
                file,
                line,
                &command.to_ascii_lowercase(),
                in_lint_staged,
                out,
                seen,
            );
        }
        JsonValue::Array(items) => {
            for item in items {
                emit_json_value_findings(file, key, item, in_lint_staged, out, seen);
            }
        }
        JsonValue::Object(map) => {
            for (child_key, child) in map {
                emit_json_value_findings(
                    file,
                    child_key,
                    child,
                    in_lint_staged || key == "lint-staged",
                    out,
                    seen,
                );
            }
        }
        _ => {}
    }
}

fn emit_line_findings(
    file: &FileInfo,
    line: usize,
    lower: &str,
    in_lint_staged: bool,
    out: &mut Vec<LanguageFinding>,
    seen: &mut BTreeSet<(String, usize, &'static str)>,
) {
    if bypass_hit(lower) {
        push_once(
            out,
            seen,
            file,
            line,
            "gittools.bypass.no-verify-automation",
            "hook or release automation normalizes --no-verify",
            "checked-in hook tooling makes bypassing verification part of the workflow",
            "remove the routine bypass and keep emergency exceptions explicit, reviewed, and CI-covered",
        );
    }
    if hooks_path_disabled_hit(lower) {
        push_once(
            out,
            seen,
            file,
            line,
            "gittools.hooks-path.disabled",
            "checked-in automation disables Git hooks",
            "core.hooksPath is configured to /dev/null in repository automation",
            "remove the disabled hooksPath setting or replace it with a versioned repo-local hook path",
        );
    }
    if raw_hooks_install_hit(lower) {
        push_once(
            out,
            seen,
            file,
            line,
            "gittools.raw-hooks.unversioned-install",
            "team hook installer mutates unversioned .git/hooks files",
            "checked-in automation writes to per-clone hook files that are not versioned policy",
            "use a committed hook directory or hook manager and document the install/update flow",
        );
    }
    if destructive_hook_hit(lower) {
        push_once(
            out,
            seen,
            file,
            line,
            "gittools.hook.destructive-git-command",
            "hook automation performs destructive Git mutation",
            "hook or hook-manager command can delete local worktree, index, or ref state",
            "remove destructive mutation from hooks and move any reviewed repair flow to an explicit command",
        );
    }
    if !in_lint_staged && unbounded_stage_hit(lower) {
        push_once(
            out,
            seen,
            file,
            line,
            "gittools.hook.unbounded-stage",
            "hook automation stages the whole worktree",
            "hook tooling can add unrelated local files to a commit",
            "stage only explicit paths or rely on the hook manager's staged-file handling",
        );
    }
    if in_lint_staged && lint_staged_manual_restage_hit(lower) {
        push_once(
            out,
            seen,
            file,
            line,
            "gittools.lint-staged.manual-restage",
            "lint-staged command manually restages project-wide changes",
            "lint-staged should manage the index instead of running project-wide git add",
            "let lint-staged manage the index and remove project-wide git add commands",
        );
    }
}

// Deduplicated finding helper threads detector context through to the shared report shape.
#[allow(clippy::too_many_arguments)]
fn push_once(
    out: &mut Vec<LanguageFinding>,
    seen: &mut BTreeSet<(String, usize, &'static str)>,
    file: &FileInfo,
    line: usize,
    detector_id: &'static str,
    problem: &'static str,
    reason: &'static str,
    fix: &'static str,
) {
    if nearby_allow(&file.text, line, detector_id) {
        return;
    }
    let key = (file.rel_path.clone(), line, detector_id);
    if seen.insert(key) {
        out.push(finding(
            RULE_ID,
            detector_id,
            file,
            line,
            problem,
            reason,
            fix,
            ProofWindow::None,
        ));
    }
}

fn bypass_hit(lower: &str) -> bool {
    lower.contains("git commit --no-verify")
        || lower.contains("git push --no-verify")
        || lower.split_whitespace().any(|part| part == "--no-verify")
}

fn hooks_path_disabled_hit(lower: &str) -> bool {
    let compact = lower.replace(' ', "");
    compact.contains("core.hookspath=/dev/null")
        || lower.contains("core.hookspath /dev/null")
        || lower.contains("core.hookspath=/dev/null")
}

fn raw_hooks_install_hit(lower: &str) -> bool {
    lower.contains(".git/hooks/")
        && [
            " cp ",
            "cp ",
            "install ",
            "ln -s",
            "ln -sf",
            "chmod ",
            "rm ",
            "rm -f",
            "rm -rf",
            "cat >",
            "tee ",
            "writefile",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn destructive_hook_hit(lower: &str) -> bool {
    [
        "git reset --hard",
        "git restore .",
        "git checkout -- .",
        "git clean -fd",
        "git clean -fx",
        "git clean -ffdx",
        "git stash -u",
        "git stash --all",
        "git stash clear",
        "git branch -d",
        "git branch -D",
        "git tag -d",
        "git update-ref",
        "git filter-branch",
        "git filter-repo",
        "rm -rf .git",
        "rm -fr .git",
        "git push --force",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn unbounded_stage_hit(lower: &str) -> bool {
    lower.contains("git add .") || lower.contains("git add -a") || lower.contains("git commit -am")
}

fn lint_staged_manual_restage_hit(lower: &str) -> bool {
    lower.contains("git add .") || lower.contains("git add -a")
}

fn is_lint_staged_surface(path: &str) -> bool {
    path.to_ascii_lowercase().starts_with(".lintstagedrc")
}

fn advisory_signals(ctx: &AuditContext) -> usize {
    let mut total = 0;
    let files = gittools_files(ctx);
    let has_hook_manager = files.iter().any(|file| {
        let lower = file.rel_path.to_ascii_lowercase();
        is_gittools_owned_surface(&lower) || lower == "package.json"
    });
    let has_ci_mirror = ctx.all_files.iter().any(|file| {
        let lower = file.rel_path.to_ascii_lowercase();
        lower.starts_with(".github/workflows/")
            && (file.text.contains("pre-commit")
                || file.text.contains("lint-staged")
                || file.text.contains("commitlint")
                || file.text.contains("lefthook")
                || file.text.contains("just check"))
    });
    if has_hook_manager && !has_ci_mirror {
        total += 1;
    }
    for file in files {
        let lower = file.text.to_ascii_lowercase();
        if hook_runs_slow_suite(&file.rel_path, &lower) {
            total += 1;
        }
        if (file.rel_path == ".pre-commit-config.yaml" || file.rel_path == ".pre-commit-config.yml")
            && lower.contains("repo:")
            && !lower.contains("rev:")
        {
            total += 1;
        }
        if lower.contains("core.hookspath")
            && lower.contains('/')
            && !lower.contains("/dev/null")
            && !lower.contains(".githooks")
            && !lower.contains(".husky")
        {
            total += 1;
        }
    }
    total
}

fn hook_runs_slow_suite(path: &str, lower: &str) -> bool {
    let hook_path = path.to_ascii_lowercase();
    (hook_path.contains("pre-commit") || lower.contains("pre-commit"))
        && [
            "cargo test",
            "npm test",
            "pytest",
            "go test ./...",
            "docker build",
            "terraform plan",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn find_line(file: &FileInfo, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    file.text
        .lines()
        .position(|line| line.contains(needle))
        .map(|idx| idx + 1)
}

fn line_kind(file: &FileInfo) -> &'static str {
    let lower = file.rel_path.to_ascii_lowercase();
    if lower.ends_with(".yml") || lower.ends_with(".yaml") {
        "yaml"
    } else if lower.ends_with(".json") || lower == "package.json" {
        "source"
    } else {
        "shell"
    }
}
