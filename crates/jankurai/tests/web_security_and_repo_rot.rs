use jankurai::audit::{self, web_security};
use jankurai::model::{FileInfo, Finding};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn write_base_repo(repo: &Path) {
    write(
        &repo.join("AGENTS.md"),
        "Read `agent/JANKURAI_STANDARD.md` first.\n",
    );
    write(
        &repo.join("README.md"),
        "# Repo\n\nlayout map validate workspace\n",
    );
    write(&repo.join("Justfile"), "check:\n    cargo test\n");
    write(
        &repo.join("agent/JANKURAI_STANDARD.md"),
        "Standard version: `0.8.11`\n",
    );
    write(
        &repo.join("docs/agent-native-standard.md"),
        "Standard version: `0.8.11`\n",
    );
}

fn run(repo: &Path) -> jankurai::model::Report {
    audit::run_audit(repo, &[]).unwrap()
}

fn findings<'a>(report: &'a jankurai::model::Report, rule_id: &str) -> Vec<&'a Finding> {
    report
        .findings
        .iter()
        .filter(|finding| finding.rule_id.as_deref() == Some(rule_id))
        .collect()
}

fn assert_detector(findings: &[&Finding], detector_id: &str) {
    assert!(
        findings
            .iter()
            .any(|finding| finding.matched_term.as_deref() == Some(detector_id)),
        "missing detector `{detector_id}` in {findings:?}"
    );
}

fn file_info(rel_path: &str, text: &str) -> FileInfo {
    let path = PathBuf::from(rel_path);
    FileInfo {
        rel_path: rel_path.into(),
        name: path.file_name().unwrap().to_string_lossy().into_owned(),
        suffix: path
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

fn ctx_with_files(files: Vec<FileInfo>) -> jankurai::audit::helpers::AuditContext {
    let root = tempdir().unwrap();
    jankurai::audit::helpers::AuditContext {
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
fn web_security_flags_vite_public_dev_server_config() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    write(
        &repo.path().join("apps/web/vite.config.ts"),
        r#"
export default {
  server: {
    host: "0.0.0.0",
    allowedHosts: true,
    cors: true,
    fs: { strict: false },
  },
};
"#,
    );

    let report = run(repo.path());
    let hlt039 = findings(&report, "HLT-039-WEB-SECURITY-BAD-BEHAVIOR");

    assert_detector(&hlt039, "websec.vite.public-dev-server");
    assert!(report
        .caps_applied
        .iter()
        .any(|cap| cap == "web-security-bad-behavior"));
}

#[test]
fn web_security_flags_vite_client_secret_env_names() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    write(
        &repo.path().join(".env.production"),
        "VITE_SECRET_TOKEN=abc\nVITE_PUBLIC_KEY=public\n",
    );
    write(
        &repo.path().join("apps/web/src/env.ts"),
        "export const db = import.meta.env.VITE_DB_PASSWORD;\n",
    );

    let report = run(repo.path());
    let hlt039 = findings(&report, "HLT-039-WEB-SECURITY-BAD-BEHAVIOR");

    assert_detector(&hlt039, "websec.env.client-secret");
    assert!(hlt039.iter().any(|finding| {
        finding
            .evidence
            .iter()
            .any(|item| item.contains("VITE_SECRET_TOKEN") || item.contains("VITE_DB_PASSWORD"))
    }));
}

#[test]
fn web_security_flags_browser_token_storage() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    write(
        &repo.path().join("apps/web/src/session.ts"),
        r#"
export function save(accessToken: string) {
  localStorage.setItem("access_token", accessToken);
}
"#,
    );

    let report = run(repo.path());
    let hlt039 = findings(&report, "HLT-039-WEB-SECURITY-BAD-BEHAVIOR");

    assert_detector(&hlt039, "websec.storage.token");
}

#[test]
fn web_security_flags_credentialed_wildcard_cors() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    write(
        &repo.path().join("apps/api/src/cors.ts"),
        r#"
export const corsOptions = {
  origin: "*",
  credentials: true,
};
"#,
    );

    let report = run(repo.path());
    let hlt039 = findings(&report, "HLT-039-WEB-SECURITY-BAD-BEHAVIOR");

    assert_detector(&hlt039, "websec.cors.credential-wildcard");
}

#[test]
fn web_security_safe_vite_config_and_public_keys_do_not_flag() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    write(
        &repo.path().join("apps/web/vite.config.ts"),
        r#"
export default {
  server: {
    host: "127.0.0.1",
    allowedHosts: ["localhost", "127.0.0.1"],
    cors: false,
    fs: { strict: true },
  },
  build: { sourcemap: false },
};
"#,
    );
    write(
        &repo.path().join(".env.production"),
        "VITE_PUBLIC_KEY=public\nVITE_PUBLISHABLE_KEY=pk_test\nVITE_MAPBOX_PUBLIC_TOKEN=public\n",
    );

    let report = run(repo.path());

    assert!(
        findings(&report, "HLT-039-WEB-SECURITY-BAD-BEHAVIOR").is_empty(),
        "{:?}",
        report.findings
    );
}

#[test]
fn web_security_safe_sanitized_html_does_not_duplicate_typescript_rule() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    write(
        &repo.path().join("apps/web/src/SafeHtml.tsx"),
        r#"
import DOMPurify from "dompurify";

export function SafeHtml({ html }: { html: string }) {
  return <div dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(html) }} />;
}
"#,
    );

    let report = run(repo.path());

    assert!(findings(&report, "HLT-039-WEB-SECURITY-BAD-BEHAVIOR").is_empty());
    assert!(findings(&report, "HLT-031-TYPESCRIPT-BAD-BEHAVIOR").is_empty());
}

#[test]
fn web_security_open_redirect_and_sourcemap_are_advisory_only() {
    let ctx = ctx_with_files(vec![
        file_info(
            "apps/web/src/redirect.ts",
            r#"
const next = new URLSearchParams(location.search).get("next");
window.location.href = next ?? "/";
"#,
        ),
        file_info(
            "apps/web/vite.config.ts",
            "export default { build: { sourcemap: true } };\n",
        ),
    ]);

    let summary = web_security::summary(&ctx);

    assert_eq!(summary.hard_findings, 0);
    assert!(
        summary.advisory_signals >= 2,
        "expected redirect and sourcemap advisory signals, got {summary:?}"
    );
}

#[test]
fn web_security_ignores_docs_tips_reference_tests_and_generated() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    for rel in [
        "docs/vite.config.ts",
        "tips/vite.config.ts",
        "reference/vite.config.ts",
        "apps/web/tests/vite.config.ts",
        "generated/vite.config.ts",
        "target/vite.config.ts",
    ] {
        write(
            &repo.path().join(rel),
            "export default { server: { allowedHosts: true, cors: true } };\n",
        );
    }

    let report = run(repo.path());

    assert!(findings(&report, "HLT-039-WEB-SECURITY-BAD-BEHAVIOR").is_empty());
}

#[test]
fn repo_rot_flags_active_fake_versioned_paths() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    write(
        &repo.path().join("src/payment_old.ts"),
        "export const x = 1;\n",
    );
    write(
        &repo.path().join("apps/web/archive/Button.tsx"),
        "export function Button() { return null; }\n",
    );
    write(
        &repo.path().join("crates/application/backup/mod.rs"),
        "pub fn backup() {}\n",
    );

    let report = run(repo.path());
    let hlt040 = findings(&report, "HLT-040-REPO-ROT-BAD-BEHAVIOR");

    assert_detector(&hlt040, "repo-rot.path.fake-versioned-source");
    assert!(report
        .caps_applied
        .iter()
        .any(|cap| cap == "repo-rot-bad-behavior"));
}

#[test]
fn repo_rot_allows_contract_versions_and_migrations() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    write(
        &repo.path().join("contracts/api/v1/openapi.yaml"),
        "openapi: 3.1.0\n",
    );
    write(
        &repo.path().join("api/v2/routes.ts"),
        "export const version = 'v2';\n",
    );
    write(
        &repo
            .path()
            .join("db/migrations/20260506120000_add_users.sql"),
        "CREATE TABLE users(id bigint primary key);\n",
    );
    write(&repo.path().join("CHANGELOG.md"), "# Changelog\n");

    let report = run(repo.path());

    assert!(findings(&report, "HLT-040-REPO-ROT-BAD-BEHAVIOR").is_empty());
}

#[test]
fn repo_rot_flags_commented_out_code_block_and_if_false_without_cap() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    write(
        &repo.path().join("src/checkout.ts"),
        r#"
// old checkout implementation
// function oldCheckout() {
//   const total = 1;
//   if (total) {
//     return total;
//   }
// }
export function checkout() {
  if (false) {
    return "disabled";
  }
  return "ok";
}
"#,
    );

    let report = run(repo.path());
    let hlt040 = findings(&report, "HLT-040-REPO-ROT-BAD-BEHAVIOR");

    assert_detector(&hlt040, "repo-rot.comment.dead-code-block");
    assert_detector(&hlt040, "repo-rot.unreachable.hard-disabled");
    assert!(!report
        .caps_applied
        .iter()
        .any(|cap| cap == "repo-rot-bad-behavior"));
}

#[test]
fn repo_rot_allow_comment_suppresses_path_detector() {
    let repo = tempdir().unwrap();
    write_base_repo(repo.path());
    write(
        &repo.path().join("src/payment_old.ts"),
        "// jankurai:allow repo-rot.path.fake-versioned-source reason=compat-window expires=2026-12-31\nexport const x = 1;\n",
    );

    let report = run(repo.path());

    assert!(findings(&report, "HLT-040-REPO-ROT-BAD-BEHAVIOR").is_empty());
    assert!(!report
        .caps_applied
        .iter()
        .any(|cap| cap == "repo-rot-bad-behavior"));
}
