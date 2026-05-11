use super::catalog::{ConfidencePolicy, Language, LanguageRule, Matcher, ProofWindow};
use super::common::{
    contains_secret_name, finding, is_docs_reference_tips_or_generated,
    is_executable_policy_surface, is_test_fixture_or_example, sort_and_cap_findings,
    strip_comments_for_line_language,
};
use super::LanguageFinding;
use crate::audit::helpers::AuditContext;
use crate::model::FileInfo;
use std::collections::BTreeSet;

const RULE_ID: &str = "HLT-035-GIT-BAD-BEHAVIOR";

const HARD_RULES: &[LanguageRule] = &[
    LanguageRule {
        id: "git.destructive.reset-hard",
        language: Language::Git,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["git reset --hard", "git restore .", "git checkout -- ."]),
        proof_window: ProofWindow::None,
        problem: "script performs a hard reset or equivalent tree restore",
        fix: "replace the destructive reset with a targeted checkout or explicit path list",
    },
    LanguageRule {
        id: "git.destructive.clean",
        language: Language::Git,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "git clean -ffdx",
            "git clean -fdx",
            "git clean -fx",
            "git clean -fd",
        ]),
        proof_window: ProofWindow::None,
        problem: "script performs a destructive git clean",
        fix: "narrow the clean scope or keep the deleted paths explicit",
    },
    LanguageRule {
        id: "git.stash.hidden-state",
        language: Language::Git,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "git stash -u",
            "git stash --all",
            "git stash pop",
            "git stash drop",
            "git stash clear",
        ]),
        proof_window: ProofWindow::None,
        problem: "script hides state in git stash",
        fix: "avoid stash-based automation or make the hidden state explicit",
    },
    LanguageRule {
        id: "git.remote.force-mutation",
        language: Language::Git,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "git push --force",
            "git push --mirror",
            "git push --all",
            "git push --tags",
        ]),
        proof_window: ProofWindow::None,
        problem: "script force-pushes or mirrors refs to the remote",
        fix: "replace the force push with a reviewed fast-forward or a dedicated release branch",
    },
    LanguageRule {
        id: "git.refs.destructive",
        language: Language::Git,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "git branch -d",
            "git tag -d",
            "git update-ref",
            "git reflog expire",
            "git gc --prune=now",
            "git filter-branch",
            "git filter-repo",
        ]),
        proof_window: ProofWindow::None,
        problem: "script destructively mutates refs or repository history",
        fix: "replace the destructive ref edit with a scoped, reviewed history operation",
    },
    LanguageRule {
        id: "git.worktree.force-cleanup",
        language: Language::Git,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "git worktree remove --force",
            "git worktree prune",
            "rm -rf .git",
            "rm -fr .git",
        ]),
        proof_window: ProofWindow::None,
        problem: "script force-cleans a git worktree or repository metadata",
        fix: "avoid forced cleanup or constrain it to the exact temporary path",
    },
    LanguageRule {
        id: "git.stage.unbounded",
        language: Language::Git,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "git add .",
            "git add -a",
            "git commit -am",
            "--no-verify",
        ]),
        proof_window: ProofWindow::None,
        problem: "script stages the entire tree or skips verification",
        fix: "enumerate the exact paths and keep verification on",
    },
    LanguageRule {
        id: "git.remote.credential-url",
        language: Language::Git,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "agent",
        lane: "audit",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["remote set-url", "remote add", "url =", "git clone"]),
        proof_window: ProofWindow::None,
        problem: "remote URL embeds a credential",
        fix: "move credentials out of the URL and use a token helper or credential store",
    },
];

#[derive(Debug, Clone, Copy, Default)]
pub struct GitSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn summary(ctx: &AuditContext) -> GitSummary {
    GitSummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: advisory_signals(ctx),
    }
}

pub fn catalog() -> &'static [LanguageRule] {
    HARD_RULES
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in git_files(ctx) {
        out.extend(findings_for_file(&file));
    }
    sort_and_cap_findings(out, 50)
}

fn advisory_signals(ctx: &AuditContext) -> usize {
    let mut total = 0;
    for file in git_files(ctx) {
        let text = file.text.to_ascii_lowercase();
        if text.contains("git commit")
            && weak_commit_message(&text)
            && !text.contains("git status --porcelain")
            && !text.contains("git diff --quiet")
        {
            total += 1;
        }
        if (text.contains("git add .")
            || text.contains("git add -a")
            || text.contains("commit -am"))
            && !text.contains("git status --porcelain")
            && !text.contains("git ls-files --others --exclude-standard")
        {
            total += 1;
        }
        if text.contains("git push") && !text.contains("git status --porcelain") {
            total += 1;
        }
        if (text.contains("cp -r .") || text.contains("rsync -a .")) && !text.contains("/.git") {
            total += 1;
        }
    }
    total
}

fn git_files(ctx: &AuditContext) -> Vec<FileInfo> {
    ctx.all_files
        .iter()
        .filter(|file| is_git_file(file))
        .cloned()
        .collect()
}

fn is_git_file(file: &FileInfo) -> bool {
    if is_docs_reference_tips_or_generated(&file.rel_path)
        || is_test_fixture_or_example(&file.rel_path)
        || super::gittools::is_gittools_owned_surface(&file.rel_path)
    {
        return false;
    }
    is_executable_policy_surface(file)
        || file.rel_path.eq_ignore_ascii_case(".gitmodules")
        || file.rel_path.eq_ignore_ascii_case("package.json")
        || file.rel_path.ends_with("Makefile")
        || file.rel_path.ends_with("Justfile")
}

fn findings_for_file(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let file_kind = line_kind(file);

    for (line_no, raw_line) in file.text.lines().enumerate() {
        let line = strip_comments_for_line_language(raw_line, file_kind);
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();

        if lower.contains("git reset --hard")
            || lower.contains("git restore .")
            || lower.contains("git checkout -- .")
        {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "git.destructive.reset-hard",
                    file,
                    line_no + 1,
                    "script performs a hard reset or equivalent tree restore",
                    "script mutates the working tree destructively",
                    "replace the destructive reset with a targeted checkout or explicit path list",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if git_clean_hit(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "git.destructive.clean",
                    file,
                    line_no + 1,
                    "script performs a destructive git clean",
                    "script removes untracked files and directories without local proof",
                    "narrow the clean scope or keep the deleted paths explicit",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if lower.contains("git stash")
            && (lower.contains(" -u")
                || lower.contains(" --all")
                || lower.contains(" pop")
                || lower.contains(" drop")
                || lower.contains(" clear"))
        {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "git.stash.hidden-state",
                    file,
                    line_no + 1,
                    "script hides state in git stash",
                    "automation stores hidden worktree state in stash",
                    "avoid stash-based automation or make the hidden state explicit",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if lower.contains("git push")
            && (lower.contains(" --force")
                || lower.contains(" --mirror")
                || lower.contains(" --all")
                || lower.contains(" --tags"))
        {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "git.remote.force-mutation",
                    file,
                    line_no + 1,
                    "script force-pushes or mirrors refs to the remote",
                    "remote mutation can overwrite shared branch history",
                    "replace the force push with a reviewed fast-forward or a dedicated release branch",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if lower.contains("git branch -d")
            || lower.contains("git tag -d")
            || lower.contains("git update-ref")
            || lower.contains("git reflog expire")
            || lower.contains("git gc --prune=now")
            || lower.contains("git filter-branch")
            || lower.contains("git filter-repo")
        {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "git.refs.destructive",
                    file,
                    line_no + 1,
                    "script destructively mutates refs or repository history",
                    "git history rewrite or ref deletion is present in automation",
                    "replace the destructive ref edit with a scoped, reviewed history operation",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if lower.contains("git worktree remove --force")
            || lower.contains("git worktree prune")
            || lower.contains("rm -rf .git")
            || lower.contains("rm -fr .git")
        {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "git.worktree.force-cleanup",
                    file,
                    line_no + 1,
                    "script force-cleans a git worktree or repository metadata",
                    "cleanup path can remove worktree state without a local proof",
                    "avoid forced cleanup or constrain it to the exact temporary path",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if git_stage_unbounded_hit(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "git.stage.unbounded",
                    file,
                    line_no + 1,
                    "script stages the entire tree or skips verification",
                    "automation commits broad untracked state or bypasses verification",
                    "enumerate the exact paths and keep verification on",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if git_remote_credential_hit(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "git.remote.credential-url",
                    file,
                    line_no + 1,
                    "remote URL embeds a credential",
                    "remote credential is present in a command or config line",
                    "move credentials out of the URL and use a token helper or credential store",
                    super::catalog::ProofWindow::None,
                ),
            );
        }
    }

    sort_and_cap_findings(out, 50)
}

fn git_clean_hit(lower: &str) -> bool {
    lower.contains("git clean -ffdx")
        || lower.contains("git clean -fdx")
        || lower.contains("git clean -fx")
        || lower.contains("git clean -fd")
}

fn git_stage_unbounded_hit(lower: &str) -> bool {
    lower.contains("git add .")
        || lower.contains("git add -a")
        || lower.contains("git commit -am")
        || (lower.contains("git commit") && lower.contains("--no-verify"))
        || lower.contains("--no-verify")
}

fn git_remote_credential_hit(lower: &str) -> bool {
    (lower.contains("remote set-url")
        || lower.contains("remote add")
        || lower.contains("url =")
        || lower.contains("git clone"))
        && lower.contains("://")
        && lower.contains("@")
        && contains_secret_name(lower)
}

fn weak_commit_message(lower: &str) -> bool {
    [
        "\"update\"",
        "\"fix\"",
        "\"wip\"",
        "\"changes\"",
        "\"misc\"",
        "\"version bump\"",
        "\"bump\"",
        "\"tmp\"",
        "'update'",
        "'fix'",
        "'wip'",
        "'changes'",
        "'misc'",
        "'version bump'",
        "'bump'",
        "'tmp'",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn line_kind(file: &FileInfo) -> &'static str {
    let lower = file.rel_path.to_ascii_lowercase();
    if lower.ends_with(".yml") || lower.ends_with(".yaml") {
        "yaml"
    } else if lower.ends_with(".json") {
        "source"
    } else if lower.ends_with("makefile")
        || lower.ends_with("justfile")
        || lower.ends_with(".sh")
        || lower.ends_with(".ps1")
        || lower.ends_with(".bat")
        || lower.ends_with(".gitmodules")
    {
        "shell"
    } else {
        "source"
    }
}

fn push_once(
    out: &mut Vec<LanguageFinding>,
    _seen: &mut BTreeSet<&'static str>,
    finding: LanguageFinding,
) {
    out.push(finding);
}
