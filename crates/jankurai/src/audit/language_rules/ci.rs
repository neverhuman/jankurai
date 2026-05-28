use super::catalog::{ConfidencePolicy, Language, LanguageRule, Matcher, ProofWindow};
use super::common::{
    contains_secret_name, finding, is_docs_reference_tips_or_generated, is_test_fixture_or_example,
    nearby_proof, sort_and_cap_findings, strip_comments_for_line_language,
};
use super::LanguageFinding;
use crate::audit::helpers::AuditContext;
use crate::model::FileInfo;
use std::collections::BTreeSet;

const RULE_ID: &str = "HLT-034-CI-BAD-BEHAVIOR";

const HARD_RULES: &[LanguageRule] = &[
    LanguageRule {
        id: "ci.github.pull-request-target-checkout-head",
        language: Language::Ci,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["pull_request_target"]),
        proof_window: ProofWindow::None,
        problem: "pull_request_target workflow checks out the pull-request head",
        fix: "checkout the base branch or move the untrusted work into pull_request job context",
    },
    LanguageRule {
        id: "ci.permissions.write-all",
        language: Language::Ci,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["write-all"]),
        proof_window: ProofWindow::None,
        problem: "workflow declares broad write-all permissions",
        fix: "replace write-all permissions with explicit minimum required scopes",
    },
    LanguageRule {
        id: "ci.secret.echo-or-debug",
        language: Language::Ci,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["echo", "printenv", "env ", "set -x"]),
        proof_window: ProofWindow::None,
        problem: "workflow potentially leaks secrets via echo or debug output",
        fix: "never echo secrets; pass them directly to trusted binaries and keep shell tracing off",
    },
    LanguageRule {
        id: "ci.security-scan.nonblocking",
        language: Language::Ci,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["continue-on-error", "allow_failure", "|| true"]),
        proof_window: ProofWindow::None,
        problem: "security-related job is configured to continue on error",
        fix: "remove the non-blocking override so scan failures stop the pipeline",
    },
    LanguageRule {
        id: "ci.action.mutable-ref",
        language: Language::Ci,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["@main", "@master", "@latest"]),
        proof_window: ProofWindow::None,
        problem: "workflow uses a mutable action or image reference",
        fix: "pin the action to a commit SHA or stable release tag",
    },
    LanguageRule {
        id: "ci.untrusted-runner.privileged-docker",
        language: Language::Ci,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["self-hosted", "privileged: true", "--privileged", "/var/run/docker.sock", "docker:dind", "dind"]),
        proof_window: ProofWindow::None,
        problem: "untrusted CI job runs with privileged Docker access",
        fix: "remove the privileged runner, socket mount, or privileged container path from untrusted jobs",
    },
    LanguageRule {
        id: "ci.artifact.cache.secret-path",
        language: Language::Ci,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "security",
        lane: "security",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[".env", ".ssh", ".aws", ".docker", ".npmrc", ".pypirc", ".netrc", ".kube"]),
        proof_window: ProofWindow::None,
        problem: "artifact or cache path includes secret material",
        fix: "limit the path to build outputs and keep credential files out of caches and artifacts",
    },
];

#[derive(Debug, Clone, Copy, Default)]
pub struct CiSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn summary(ctx: &AuditContext) -> CiSummary {
    CiSummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: advisory_signals(ctx),
    }
}

pub fn catalog() -> &'static [LanguageRule] {
    HARD_RULES
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in ci_files(ctx) {
        out.extend(findings_for_file(&file));
    }
    sort_and_cap_findings(out, 50)
}

fn advisory_signals(ctx: &AuditContext) -> usize {
    let mut total = 0;
    for file in ci_files(ctx) {
        let text = file.text.to_ascii_lowercase();
        if (text.contains("jobs:") || text.contains("steps:")) && !text.contains("timeout-minutes:")
        {
            total += 1;
        }
        if (text.contains("deploy") || text.contains("release") || text.contains("publish"))
            && !text.contains("concurrency:")
        {
            total += 1;
        }
        if text.contains("actions/cache") && broad_cache_path(&text) {
            total += 1;
        }
        if text.contains("actions/upload-artifact") && !text.contains("retention-days:") {
            total += 1;
        }
    }
    total
}

fn ci_files(ctx: &AuditContext) -> Vec<FileInfo> {
    ctx.all_files
        .iter()
        .filter(|file| is_ci_file(file))
        .cloned()
        .collect()
}

fn is_ci_file(file: &FileInfo) -> bool {
    if is_docs_reference_tips_or_generated(&file.rel_path)
        || is_test_fixture_or_example(&file.rel_path)
    {
        return false;
    }
    let lower = file.rel_path.to_ascii_lowercase();
    lower.starts_with(".github/workflows/") && (lower.ends_with(".yml") || lower.ends_with(".yaml"))
        || lower == ".gitlab-ci.yml"
        || lower == "bitbucket-pipelines.yml"
        || lower == "jenkinsfile"
        || lower == "azure-pipelines.yml"
        || lower.starts_with(".circleci/") && lower.ends_with("config.yml")
        || lower.starts_with(".buildkite/") && (lower.ends_with(".yml") || lower.ends_with(".yaml"))
        || lower.contains("buildkite") && (lower.ends_with(".yml") || lower.ends_with(".yaml"))
}

fn findings_for_file(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let file_kind = line_kind(file);
    let text_lower = file.text.to_ascii_lowercase();
    let is_github_workflow = file
        .rel_path
        .to_ascii_lowercase()
        .starts_with(".github/workflows/");

    if is_github_workflow
        && text_lower.contains("pull_request_target")
        && text_lower.contains("actions/checkout")
        && (text_lower.contains("github.event.pull_request.head.sha")
            || text_lower.contains("github.event.pull_request.head.ref")
            || text_lower.contains("github.event.pull_request.head"))
    {
        let line = find_line(
            file,
            &[
                "github.event.pull_request.head.sha",
                "github.event.pull_request.head.ref",
                "github.event.pull_request.head",
                "actions/checkout",
            ],
        )
        .unwrap_or(1);
        push_once(
            &mut out,
            &mut seen,
            finding(
                RULE_ID,
                "ci.github.pull-request-target-checkout-head",
                file,
                line,
                "pull_request_target workflow checks out the pull-request head",
                "pull_request_target can run with repository secrets while executing PR-controlled content",
                "checkout the base branch or move the untrusted work into pull_request job context",
                super::catalog::ProofWindow::None,
            ),
        );
    }

    if is_github_workflow && !text_lower.contains("permissions:") {
        push_once(
            &mut out,
            &mut seen,
            finding(
                RULE_ID,
                "ci.permissions.missing",
                file,
                1,
                "workflow is missing explicit top-level permissions",
                "workflow permissions default is not pinned in source",
                "add top-level `permissions: contents: read` and job-specific write scopes only where needed",
                super::catalog::ProofWindow::None,
            ),
        );
    }
    if is_github_workflow && !text_lower.contains("timeout-minutes:") {
        push_once(
            &mut out,
            &mut seen,
            finding(
                RULE_ID,
                "ci.timeout.missing",
                file,
                1,
                "workflow job is missing timeout-minutes",
                "workflow can run without a checked time bound",
                "set an explicit timeout-minutes on each job",
                super::catalog::ProofWindow::None,
            ),
        );
    }
    if is_github_workflow && !text_lower.contains("concurrency:") {
        push_once(
            &mut out,
            &mut seen,
            finding(
                RULE_ID,
                "ci.concurrency.missing",
                file,
                1,
                "workflow is missing concurrency control",
                "workflow can run duplicate stale audits for the same ref",
                "add workflow-level concurrency with cancel-in-progress",
                super::catalog::ProofWindow::None,
            ),
        );
    }
    if is_github_workflow
        && text_lower.contains("sarif")
        && !text_lower.contains("github/codeql-action/upload-sarif@")
    {
        push_once(
            &mut out,
            &mut seen,
            finding(
                RULE_ID,
                "ci.sarif.not-uploaded",
                file,
                find_line(file, &["sarif"]).unwrap_or(1),
                "workflow creates SARIF without uploading it",
                "SARIF evidence is not published to code scanning",
                "upload the SARIF artifact with github/codeql-action/upload-sarif pinned to a full SHA",
                super::catalog::ProofWindow::None,
            ),
        );
    }
    if text_lower.contains("baseline-score.json")
        || (text_lower.contains("cp .jankurai/repo-score.json")
            && text_lower.contains("target/jankurai"))
    {
        push_once(
            &mut out,
            &mut seen,
            finding(
                RULE_ID,
                "ci.ratchet.self-generated-baseline",
                file,
                find_line(file, &["baseline-score.json", "cp .jankurai/repo-score.json"]).unwrap_or(1),
                "ratchet workflow creates a baseline from the candidate run",
                "candidate evidence can hide score regressions",
                "copy a reviewed accepted baseline from agent/baselines or origin/main before the final audit",
                super::catalog::ProofWindow::None,
            ),
        );
    }
    if (text_lower.contains("--mode ratchet") || text_lower.contains("--mode release"))
        && !text_lower.contains("jankurai security run . --strict --profile ci")
    {
        push_once(
            &mut out,
            &mut seen,
            finding(
                RULE_ID,
                "ci.security.strict-missing",
                file,
                find_line(file, &["jankurai audit", "--mode ratchet", "--mode release"]).unwrap_or(1),
                "protected workflow lacks strict CI security evidence before final audit",
                "final ratchet or release audit is not bound to strict security evidence",
                "run `jankurai security run . --strict --profile ci --out target/jankurai/security/evidence.json` before the final audit",
                super::catalog::ProofWindow::None,
            ),
        );
    }

    for (line_no, raw_line) in file.text.lines().enumerate() {
        let line = strip_comments_for_line_language(raw_line, file_kind);
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();

        if lower.contains("permissions:") && lower.contains("write-all") {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "ci.permissions.write-all",
                    file,
                    line_no + 1,
                    "workflow declares broad write-all permissions",
                    "workflow grants more permissions than needed",
                    "replace write-all permissions with explicit minimum required scopes",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if line_has_secret_dump(&lower) && contains_secret_name(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "ci.secret.echo-or-debug",
                    file,
                    line_no + 1,
                    "workflow potentially leaks secrets via echo or debug output",
                    "secret-bearing workflow step writes sensitive values to logs",
                    "never echo secrets; pass them directly to trusted binaries and keep shell tracing off",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if (line.contains("continue-on-error")
            || line.contains("allow_failure")
            || line.contains("|| true"))
            && nearby_proof(
                &file.text,
                line_no + 1,
                &["security", "proof", "sbom", "secret", "dependency"],
            )
        {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "ci.security-scan.nonblocking",
                    file,
                    line_no + 1,
                    "security-related job is configured to continue on error",
                    "security or proof job is explicitly non-blocking",
                    "remove the non-blocking override so scan failures stop the pipeline",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if line.contains("uses:") && mutable_ref_hit(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "ci.action.mutable-ref",
                    file,
                    line_no + 1,
                    "workflow uses a mutable action or image reference",
                    "action ref can change without review",
                    "pin the action to a commit SHA or stable release tag",
                    super::catalog::ProofWindow::None,
                ),
            );
        }
        if line.contains("uses:") && unpinned_action_ref(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "ci.action.not-full-sha",
                    file,
                    line_no + 1,
                    "workflow uses an external action not pinned to a full commit SHA",
                    "tag or branch refs can change without review",
                    "pin every external action to a 40-character commit SHA",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if line.contains("image:") && lower.contains(":latest") {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "ci.action.mutable-ref",
                    file,
                    line_no + 1,
                    "workflow uses a mutable container image tag",
                    "container image tag can move without review",
                    "pin the image to a digest or immutable version tag",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if line_no == 1 && privileged_runner_hit(&file.text) {
            let line = find_line(
                file,
                &[
                    "self-hosted",
                    "privileged: true",
                    "--privileged",
                    "/var/run/docker.sock",
                    "docker:dind",
                    "dind",
                ],
            )
            .unwrap_or(1);
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "ci.untrusted-runner.privileged-docker",
                    file,
                    line,
                    "untrusted CI job runs with privileged Docker access",
                    "PR or fork-triggered workflow can reach self-hosted or privileged Docker",
                    "remove the privileged runner, socket mount, or privileged container path from untrusted jobs",
                    super::catalog::ProofWindow::None,
                ),
            );
        }

        if secret_path_hit(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "ci.artifact.cache.secret-path",
                    file,
                    line_no + 1,
                    "artifact or cache path includes secret material",
                    "workflow stores a secret-bearing path in cache or artifact upload",
                    "limit the path to build outputs and keep credential files out of caches and artifacts",
                    super::catalog::ProofWindow::None,
                ),
            );
        }
    }

    sort_and_cap_findings(out, 50)
}

fn broad_cache_path(text: &str) -> bool {
    text.contains("path: .")
        || text.contains("path: ./")
        || text.contains("path: ${{ github.workspace }}")
        || text.contains("path: **")
        || text.contains("paths: .")
}

fn line_has_secret_dump(lower: &str) -> bool {
    (lower.contains("echo ") || lower.contains("printenv") || lower.contains("env "))
        && (lower.contains("secrets.")
            || contains_secret_name(lower)
            || lower.contains("$token")
            || lower.contains("$secret"))
        || (lower.contains("set -x") && contains_secret_name(lower))
}

fn mutable_ref_hit(lower: &str) -> bool {
    lower.contains("@main") || lower.contains("@master") || lower.contains("@latest")
}

fn unpinned_action_ref(lower: &str) -> bool {
    let Some((_, rest)) = lower.split_once('@') else {
        return false;
    };
    let reference = rest
        .split(|ch: char| ch.is_whitespace() || ch == '"' || ch == '\'')
        .next()
        .unwrap_or("");
    reference.len() != 40 || !reference.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn privileged_runner_hit(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    (lower.contains("pull_request")
        || lower.contains("pull_request_target")
        || lower.contains("fork"))
        && (lower.contains("self-hosted")
            || lower.contains("privileged: true")
            || lower.contains("--privileged")
            || lower.contains("/var/run/docker.sock")
            || lower.contains("docker:dind")
            || lower.contains("dind"))
        && lower.contains("jobs:")
}

fn secret_path_hit(lower: &str) -> bool {
    [
        ".env", ".ssh", ".aws", ".docker", ".npmrc", ".pypirc", ".netrc", ".kube",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn find_line(file: &FileInfo, needles: &[&str]) -> Option<usize> {
    for (idx, raw_line) in file.text.lines().enumerate() {
        let lower = raw_line.to_ascii_lowercase();
        if needles.iter().any(|needle| lower.contains(needle)) {
            return Some(idx + 1);
        }
    }
    None
}

fn line_kind(file: &FileInfo) -> &'static str {
    let lower = file.rel_path.to_ascii_lowercase();
    if lower.ends_with(".yml") || lower.ends_with(".yaml") {
        "yaml"
    } else if lower.ends_with("jenkinsfile")
        || lower.ends_with(".sh")
        || lower.ends_with(".ps1")
        || lower.ends_with(".bat")
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
