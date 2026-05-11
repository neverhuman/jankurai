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

const RULE_ID: &str = "HLT-037-RELEASE-BAD-BEHAVIOR";

const HARD_RULES: &[LanguageRule] = &[
    LanguageRule {
        id: "release.git.mutable-tag",
        language: Language::Release,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "release",
        lane: "release",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["git tag -f", "git tag --force", "refs/tags"]),
        proof_window: ProofWindow::None,
        problem: "release automation mutates an existing release tag",
        fix: "publish a new version from an immutable protected tag instead of moving or deleting release refs",
    },
    LanguageRule {
        id: "release.asset.mutable-upload",
        language: Language::Release,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "release",
        lane: "release",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["gh release upload --clobber", "gh release delete"]),
        proof_window: ProofWindow::None,
        problem: "release automation overwrites or deletes published release assets",
        fix: "keep released assets immutable and publish a replacement version with new checksums and notes",
    },
    LanguageRule {
        id: "release.verification.skipped",
        language: Language::Release,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "release",
        lane: "release",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[
            "SKIP_TESTS=1",
            "NO_TESTS=1",
            "cargo publish --no-verify",
            "npm publish --ignore-scripts",
        ]),
        proof_window: ProofWindow::None,
        problem: "release automation skips packaging, lifecycle, or proof checks",
        fix: "run the release from a green commit with tests, package verification, and security evidence enabled",
    },
    LanguageRule {
        id: "release.artifact.mutable-latest",
        language: Language::Release,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "release",
        lane: "release",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["docker push *:latest", "git tag latest"]),
        proof_window: ProofWindow::None,
        problem: "release publishes a mutable latest-only artifact or tag",
        fix: "publish immutable versioned artifacts and treat latest aliases as secondary pointers with digest evidence",
    },
    LanguageRule {
        id: "release.secret.packaged",
        language: Language::Release,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "release",
        lane: "release",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&[".env", ".npmrc", ".ssh", "id_rsa", "secret"]),
        proof_window: ProofWindow::None,
        problem: "release packaging includes secret-bearing local configuration",
        fix: "use an explicit publish allowlist and scan release artifacts before upload",
    },
    LanguageRule {
        id: "release.gh.unverified-tag",
        language: Language::Release,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "release",
        lane: "release",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAll(&["gh release create"]),
        proof_window: ProofWindow::None,
        problem: "GitHub release creation does not verify the tag or tag protection evidence",
        fix: "use --verify-tag or verify the signed/protected tag before creating the release",
    },
    LanguageRule {
        id: "release.integrity.missing",
        language: Language::Release,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "release",
        lane: "release",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["gh release create", "npm publish", "cargo publish"]),
        proof_window: ProofWindow::None,
        problem: "release publishing lacks checksum, SBOM, provenance, signature, or attestation evidence",
        fix: "attach checksums plus SBOM/provenance/signature/attestation evidence to the release witness",
    },
    LanguageRule {
        id: "release.workflow.untrusted-publish",
        language: Language::Release,
        hlt_rule_id: RULE_ID,
        severity: "high",
        category: "release",
        lane: "release",
        confidence: ConfidencePolicy::High,
        matcher: Matcher::ContainsAny(&["pull_request_target", "workflow_run"]),
        proof_window: ProofWindow::None,
        problem: "release workflow can publish from an untrusted or privilege-bridged trigger",
        fix: "separate untrusted validation from protected release publishing and require trusted tag/branch gates",
    },
];

#[derive(Debug, Clone, Copy, Default)]
pub struct ReleaseSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn summary(ctx: &AuditContext) -> ReleaseSummary {
    ReleaseSummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: advisory_signals(ctx),
    }
}

pub fn catalog() -> &'static [LanguageRule] {
    HARD_RULES
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in release_files(ctx) {
        out.extend(findings_for_file(&file));
    }
    sort_and_cap_findings(out, 50)
}

fn advisory_signals(ctx: &AuditContext) -> usize {
    release_files(ctx)
        .iter()
        .filter(|file| {
            let lower = file.text.to_ascii_lowercase();
            lower.contains("release")
                && !(lower.contains("rollback")
                    || lower.contains("changelog")
                    || lower.contains("checksum")
                    || lower.contains("sha256")
                    || lower.contains("provenance")
                    || lower.contains("attestation"))
        })
        .count()
}

fn release_files(ctx: &AuditContext) -> Vec<FileInfo> {
    ctx.all_files
        .iter()
        .filter(|file| is_release_file(file))
        .cloned()
        .collect()
}

fn is_release_file(file: &FileInfo) -> bool {
    if is_docs_reference_tips_or_generated(&file.rel_path)
        || is_test_fixture_or_example(&file.rel_path)
    {
        return false;
    }
    if !is_executable_policy_surface(file) {
        return false;
    }
    let lower_path = file.rel_path.to_ascii_lowercase();
    let lower_text = file.text.to_ascii_lowercase();
    let path_says_release = lower_path.contains("release")
        || lower_path.contains("publish")
        || lower_path.contains("deploy")
        || lower_path.ends_with("package.json")
        || lower_path.starts_with(".github/workflows/");
    let text_says_release = has_release_command(&lower_text)
        || lower_text.contains("gh release")
        || lower_text.contains("npm publish")
        || lower_text.contains("cargo publish")
        || lower_text.contains("docker push")
        || lower_text.contains("changelog")
        || lower_text.contains("rollback");

    (path_says_release && text_says_release)
        || (is_executable_policy_surface(file) && has_release_command(&lower_text))
}

fn has_release_command(lower: &str) -> bool {
    [
        "gh release",
        "npm publish",
        "cargo publish",
        "twine upload",
        "docker push",
        "git tag",
        "git push",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn findings_for_file(file: &FileInfo) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let file_kind = line_kind(file);
    let file_has_tag_verification = has_tag_verification_evidence(&file.text);
    let file_has_integrity = has_integrity_evidence(&file.text);
    let file_is_privilege_bridge = has_untrusted_release_trigger(&file.text);

    for (line_no, raw_line) in file.text.lines().enumerate() {
        let line = strip_comments_for_line_language(raw_line, file_kind);
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        let line_no = line_no + 1;

        if lower.contains("git tag -f")
            || lower.contains("git tag --force")
            || (lower.contains("git push")
                && lower.contains("refs/tags")
                && (lower.contains("--force")
                    || lower.contains("--delete")
                    || lower.contains(":refs/tags")))
        {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "release.git.mutable-tag",
                    file,
                    line_no,
                    "release automation mutates an existing release tag",
                    "released versions must point to immutable protected refs",
                    "publish a new version from an immutable protected tag instead of moving or deleting release refs",
                    ProofWindow::None,
                ),
            );
        }

        if (lower.contains("gh release upload") && lower.contains("--clobber"))
            || lower.contains("gh release delete")
        {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "release.asset.mutable-upload",
                    file,
                    line_no,
                    "release automation overwrites or deletes published release assets",
                    "published artifacts must remain traceable to their original checksum and provenance",
                    "keep released assets immutable and publish a replacement version with new checksums and notes",
                    ProofWindow::None,
                ),
            );
        }

        if skips_release_verification(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "release.verification.skipped",
                    file,
                    line_no,
                    "release automation skips packaging, lifecycle, or proof checks",
                    "release gates cannot be bypassed as routine automation",
                    "run the release from a green commit with tests, package verification, and security evidence enabled",
                    ProofWindow::None,
                ),
            );
        }

        if publishes_mutable_latest(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "release.artifact.mutable-latest",
                    file,
                    line_no,
                    "release publishes a mutable latest-only artifact or tag",
                    "latest aliases do not identify the shipped version or digest",
                    "publish immutable versioned artifacts and treat latest aliases as secondary pointers with digest evidence",
                    ProofWindow::None,
                ),
            );
        }

        if packages_secret_material(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "release.secret.packaged",
                    file,
                    line_no,
                    "release packaging includes secret-bearing local configuration",
                    "release artifacts must not include local credentials or private config",
                    "use an explicit publish allowlist and scan release artifacts before upload",
                    ProofWindow::None,
                ),
            );
        }

        if lower.contains("gh release create") && !file_has_tag_verification {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "release.gh.unverified-tag",
                    file,
                    line_no,
                    "GitHub release creation does not verify the tag or tag protection evidence",
                    "a release without tag verification can bind artifacts to the wrong or mutable commit",
                    "use --verify-tag or verify the signed/protected tag before creating the release",
                    ProofWindow::None,
                ),
            );
        }

        if publishes_release_artifact(&lower) && !file_has_integrity {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "release.integrity.missing",
                    file,
                    line_no,
                    "release publishing lacks checksum, SBOM, provenance, signature, or attestation evidence",
                    "published artifacts need machine-checkable integrity and supply-chain receipts",
                    "attach checksums plus SBOM/provenance/signature/attestation evidence to the release witness",
                    ProofWindow::None,
                ),
            );
        }

        if file_is_privilege_bridge && publishes_release_artifact(&lower) {
            push_once(
                &mut out,
                &mut seen,
                finding(
                    RULE_ID,
                    "release.workflow.untrusted-publish",
                    file,
                    line_no,
                    "release workflow can publish from an untrusted or privilege-bridged trigger",
                    "privileged release work must not consume untrusted PR code or artifacts",
                    "separate untrusted validation from protected release publishing and require trusted tag/branch gates",
                    ProofWindow::None,
                ),
            );
        }
    }

    sort_and_cap_findings(out, 50)
}

fn skips_release_verification(lower: &str) -> bool {
    [
        "skip_tests=1",
        "skip_tests=true",
        "no_tests=1",
        "no_tests=true",
        "cargo publish --no-verify",
        "npm publish --ignore-scripts",
        "pnpm publish --ignore-scripts",
        "yarn npm publish --ignore-scripts",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn publishes_mutable_latest(lower: &str) -> bool {
    (lower.contains("docker push") && lower.contains(":latest"))
        || lower.contains("git tag latest")
        || lower.contains("gh release create latest")
}

fn packages_secret_material(lower: &str) -> bool {
    let packaging = lower.contains("gh release upload")
        || lower.contains("zip ")
        || lower.contains("tar ")
        || lower.contains("npm publish")
        || lower.contains("twine upload");
    packaging
        && (lower.contains(".env")
            || lower.contains(".npmrc")
            || lower.contains(".pypirc")
            || lower.contains(".ssh")
            || lower.contains("id_rsa")
            || lower.contains("private_key")
            || contains_secret_name(lower))
}

fn publishes_release_artifact(lower: &str) -> bool {
    lower.contains("gh release create")
        || lower.contains("gh release upload")
        || lower.contains("npm publish")
        || lower.contains("cargo publish")
        || lower.contains("twine upload")
}

fn has_tag_verification_evidence(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "--verify-tag",
        "git tag -v",
        "git verify-tag",
        "git tag -s",
        "protected tag",
        "protected tags",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn has_integrity_evidence(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "sha256",
        "checksum",
        "shasum",
        "cosign",
        "slsa",
        "sbom",
        "provenance",
        "attestation",
        "attest",
        "sigstore",
        "jankurai witness",
        "jankurai audit",
        "jankurai publish",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn has_untrusted_release_trigger(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    (lower.contains("pull_request_target") || lower.contains("workflow_run"))
        && (lower.contains("secrets.") || lower.contains("permissions: write-all"))
}

fn line_kind(file: &FileInfo) -> &'static str {
    let lower = file.rel_path.to_ascii_lowercase();
    if lower.ends_with(".yml") || lower.ends_with(".yaml") {
        "yaml"
    } else if lower.ends_with(".toml") {
        "toml"
    } else if lower.ends_with(".sh") || lower.ends_with("justfile") || lower.ends_with("makefile") {
        "shell"
    } else {
        "source"
    }
}

fn push_once(
    out: &mut Vec<LanguageFinding>,
    seen: &mut BTreeSet<(String, usize, &'static str)>,
    finding: LanguageFinding,
) {
    let key = (
        finding.path.clone(),
        finding.line.unwrap_or(0),
        finding.matched_term,
    );
    if seen.insert(key) {
        out.push(finding);
    }
}
