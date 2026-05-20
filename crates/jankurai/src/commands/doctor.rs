use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::postmortem::parse_failure_mode;
use crate::local_state;
use crate::validation::{self, ArtifactSchema};

pub struct DoctorArgs {
    pub repo: PathBuf,
    pub fail_on: String,
    pub json: Option<String>,
    pub md: Option<String>,
}

pub fn run(args: DoctorArgs) -> Result<()> {
    let repo = args.repo;
    let mut diagnostics = Vec::new();
    let progress = crate::ui::CliProgress::new("checking repository health", 11);
    progress.tick("required files");
    for rel in [
        "AGENTS.md",
        "agent/JANKURAI_STANDARD.md",
        "agent/owner-map.json",
        "agent/test-map.json",
        "agent/generated-zones.toml",
        "agent/proof-lanes.toml",
        "agent/standard-version.toml",
        ".jankurai/repo-score.json",
        ".jankurai/repo-score.md",
        "agent/boundaries.toml",
        "agent/security-policy.toml",
        "tools/security-lane.sh",
    ] {
        push_local_or_legacy_file_check(&repo, &mut diagnostics, rel);
    }

    if repo.join("db").exists() {
        push_file_check(&repo, &mut diagnostics, "db/README.md");
    }

    progress.tick("manifest schemas");
    check_boundaries_manifest_schema(&repo, &mut diagnostics);
    check_ux_qa_policy_schema(&repo, &mut diagnostics);
    check_security_policy_schema(&repo, &mut diagnostics);
    check_audit_policy_schema(&repo, &mut diagnostics);
    check_owner_map_schema(&repo, &mut diagnostics);
    check_test_map_schema(&repo, &mut diagnostics);
    check_generated_zones_schema(&repo, &mut diagnostics);
    check_proof_lanes_schema(&repo, &mut diagnostics);
    check_standard_version_schema(&repo, &mut diagnostics);
    progress.tick("lockfiles and score freshness");
    check_lockfiles(&repo, &mut diagnostics);
    check_root_score_artifacts(&repo, &mut diagnostics);
    check_stale_score(&repo, &mut diagnostics);
    progress.tick("local path and false-green checks");
    check_local_path_leaks(&repo, &mut diagnostics);
    check_echo_only_proof(&repo, &mut diagnostics);
    progress.tick("security and UX tools");
    check_security_tools(&repo, &mut diagnostics);
    check_committed_ux_artifacts(&repo, &mut diagnostics);
    progress.tick("paper and receipt checks");
    check_legacy_paper_sources(&repo, &mut diagnostics);
    check_severity_discipline(&repo, &mut diagnostics);
    check_receipt_exports(&repo, &mut diagnostics);
    check_proof_ledger(&repo, &mut diagnostics);
    check_security_evidence(&repo, &mut diagnostics);
    progress.tick("target artifacts");
    check_json_artifact(
        &repo,
        &mut diagnostics,
        "target/jankurai/context-pack.json",
        ArtifactSchema::ContextPack,
        "context-pack-read",
        "context-pack-json",
        "context-pack-schema",
    );
    check_json_artifact(
        &repo,
        &mut diagnostics,
        "target/jankurai/repair-plan.json",
        ArtifactSchema::RepairPlan,
        "repair-plan-read",
        "repair-plan-json",
        "repair-plan-schema",
    );
    check_json_artifact(
        &repo,
        &mut diagnostics,
        "target/jankurai/ux-qa.json",
        ArtifactSchema::UxQaReport,
        "ux-qa-report-read",
        "ux-qa-report-json",
        "ux-qa-report-schema",
    );
    check_json_artifact(
        &repo,
        &mut diagnostics,
        "target/jankurai/migration-report.json",
        ArtifactSchema::MigrationReport,
        "migration-report-read",
        "migration-report-json",
        "migration-report-schema",
    );
    check_json_artifact(
        &repo,
        &mut diagnostics,
        "target/jankurai/migration-plan.json",
        ArtifactSchema::MigrationPlan,
        "migration-plan-read",
        "migration-plan-json",
        "migration-plan-schema",
    );

    progress.tick("rank diagnostics");
    let diagnostics = enrich_diagnostics(diagnostics);

    let color = crate::ui::stdout_color_enabled();
    for diagnostic in &diagnostics {
        let style = match diagnostic.severity.as_str() {
            "ok" => crate::ui::Style::Good,
            "low" | "medium" => crate::ui::Style::Warn,
            "high" | "critical" => crate::ui::Style::Error,
            _ => crate::ui::Style::Muted,
        };
        println!(
            "{}: {} - {}",
            crate::ui::paint(style, &diagnostic.severity, color),
            diagnostic.check_id,
            diagnostic.message
        );
    }
    progress.tick("write receipts");
    if let Some(path) = args.json.as_deref() {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(&diagnostics)?)?;
    }
    if let Some(path) = args.md.as_deref() {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, render_markdown(&diagnostics))?;
    }
    write_receipt(&repo, "doctor", &diagnostics)?;
    progress.finish(format!(
        "doctor complete: {} diagnostics",
        diagnostics.len()
    ));
    if diagnostics
        .iter()
        .any(|diagnostic| severity_rank(&diagnostic.severity) <= severity_rank(&args.fail_on))
    {
        anyhow::bail!("doctor found diagnostics at or above {}", args.fail_on);
    }
    Ok(())
}

fn render_markdown(diagnostics: &[DoctorDiagnostic]) -> String {
    let mut out = String::from("# jankurai doctor\n\n");
    for diagnostic in diagnostics {
        out.push_str(&format!(
            "- **{}** `{}` `{}` [{}]: {}\n",
            diagnostic.severity,
            diagnostic.check_id,
            diagnostic.path,
            diagnostic.kind.as_str(),
            diagnostic.message
        ));
        if diagnostic.strictly_blocking {
            out.push_str("  - blocking: yes\n");
        }
        if diagnostic.environment_sensitive {
            out.push_str("  - environment sensitive\n");
        }
        if !diagnostic.common_fixes.is_empty() {
            out.push_str("  - common fixes:\n");
            for fix in &diagnostic.common_fixes {
                out.push_str(&format!("    - {}\n", fix));
            }
        }
    }
    out
}

#[allow(dead_code)]
fn _exists(path: &Path) -> bool {
    path.exists()
}

#[derive(Debug)]
struct Diagnostic {
    check_id: String,
    severity: String,
    path: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct DoctorDiagnostic {
    check_id: String,
    severity: String,
    path: String,
    message: String,
    kind: DiagnosticKind,
    environment_sensitive: bool,
    strictly_blocking: bool,
    common_fixes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum DiagnosticKind {
    FileMissing,
    Schema,
    Policy,
    Tool,
    Freshness,
    Receipt,
    Workflow,
    Lockfile,
    Export,
    Other,
}

impl DiagnosticKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::FileMissing => "file-missing",
            Self::Schema => "schema",
            Self::Policy => "policy",
            Self::Tool => "tool",
            Self::Freshness => "freshness",
            Self::Receipt => "receipt",
            Self::Workflow => "workflow",
            Self::Lockfile => "lockfile",
            Self::Export => "export",
            Self::Other => "other",
        }
    }
}

fn push_file_check(repo: &Path, diagnostics: &mut Vec<Diagnostic>, rel: &str) {
    if repo.join(rel).exists() {
        diagnostics.push(ok(rel, "present"));
    } else {
        diagnostics.push(Diagnostic {
            check_id: format!("file:{rel}"),
            severity: "high".into(),
            path: rel.into(),
            message: "missing required jankurai control file".into(),
        });
    }
}

fn push_local_or_legacy_file_check(
    repo: &Path,
    diagnostics: &mut Vec<Diagnostic>,
    local_rel: &str,
) {
    let legacy_rel = match local_rel {
        ".jankurai/repo-score.json" => Some("agent/repo-score.json"),
        ".jankurai/repo-score.md" => Some("agent/repo-score.md"),
        _ => None,
    };
    if repo.join(local_rel).exists() || legacy_rel.is_some_and(|rel| repo.join(rel).exists()) {
        diagnostics.push(ok(local_rel, "present"));
    } else {
        diagnostics.push(Diagnostic {
            check_id: format!("file:{local_rel}"),
            severity: "high".into(),
            path: local_rel.into(),
            message: "missing required jankurai control file".into(),
        });
    }
}

fn check_boundaries_manifest_schema(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let path = repo.join("agent/boundaries.toml");
    if !path.is_file() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        diagnostics.push(Diagnostic {
            check_id: "boundaries-manifest-read".into(),
            severity: "medium".into(),
            path: "agent/boundaries.toml".into(),
            message: "could not read agent/boundaries.toml".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_boundaries_toml_text(repo, &text) {
        diagnostics.push(Diagnostic {
            check_id: "boundaries-manifest-schema".into(),
            severity: "medium".into(),
            path: "agent/boundaries.toml".into(),
            message: format!("boundary manifest failed schema validation: {err}"),
        });
    }
}

fn check_ux_qa_policy_schema(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let path = repo.join("agent/ux-qa.toml");
    if !path.is_file() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        diagnostics.push(Diagnostic {
            check_id: "ux-qa-policy-read".into(),
            severity: "medium".into(),
            path: "agent/ux-qa.toml".into(),
            message: "could not read agent/ux-qa.toml".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_ux_qa_policy_toml_text(repo, &text) {
        diagnostics.push(Diagnostic {
            check_id: "ux-qa-policy-schema".into(),
            severity: "medium".into(),
            path: "agent/ux-qa.toml".into(),
            message: format!("UX QA policy failed schema validation: {err}"),
        });
    }
}

fn check_security_policy_schema(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let path = repo.join("agent/security-policy.toml");
    if !path.is_file() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        diagnostics.push(Diagnostic {
            check_id: "security-policy-read".into(),
            severity: "medium".into(),
            path: "agent/security-policy.toml".into(),
            message: "could not read agent/security-policy.toml".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_security_policy_toml_text(repo, &text) {
        diagnostics.push(Diagnostic {
            check_id: "security-policy-schema".into(),
            severity: "medium".into(),
            path: "agent/security-policy.toml".into(),
            message: format!("security policy failed schema validation: {err}"),
        });
    }
}

fn check_audit_policy_schema(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let path = repo.join("agent/audit-policy.toml");
    if !path.is_file() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        diagnostics.push(Diagnostic {
            check_id: "audit-policy-read".into(),
            severity: "medium".into(),
            path: "agent/audit-policy.toml".into(),
            message: "could not read agent/audit-policy.toml".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_audit_policy_toml_text(repo, &text) {
        diagnostics.push(Diagnostic {
            check_id: "audit-policy-schema".into(),
            severity: "medium".into(),
            path: "agent/audit-policy.toml".into(),
            message: format!("audit policy failed schema validation: {err}"),
        });
    }
}

fn check_owner_map_schema(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let path = repo.join("agent/owner-map.json");
    if !path.is_file() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        diagnostics.push(Diagnostic {
            check_id: "owner-map-read".into(),
            severity: "medium".into(),
            path: "agent/owner-map.json".into(),
            message: "could not read agent/owner-map.json".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_owner_map_json_text(repo, &text) {
        diagnostics.push(Diagnostic {
            check_id: "owner-map-schema".into(),
            severity: "medium".into(),
            path: "agent/owner-map.json".into(),
            message: format!("owner map failed schema validation: {err}"),
        });
    }
}

fn check_test_map_schema(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let path = repo.join("agent/test-map.json");
    if !path.is_file() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        diagnostics.push(Diagnostic {
            check_id: "test-map-read".into(),
            severity: "medium".into(),
            path: "agent/test-map.json".into(),
            message: "could not read agent/test-map.json".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_test_map_json_text(repo, &text) {
        diagnostics.push(Diagnostic {
            check_id: "test-map-schema".into(),
            severity: "medium".into(),
            path: "agent/test-map.json".into(),
            message: format!("test map failed schema validation: {err}"),
        });
    }
}

fn check_generated_zones_schema(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let path = repo.join("agent/generated-zones.toml");
    if !path.is_file() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        diagnostics.push(Diagnostic {
            check_id: "generated-zones-read".into(),
            severity: "medium".into(),
            path: "agent/generated-zones.toml".into(),
            message: "could not read agent/generated-zones.toml".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_generated_zones_toml_text(repo, &text) {
        diagnostics.push(Diagnostic {
            check_id: "generated-zones-schema".into(),
            severity: "medium".into(),
            path: "agent/generated-zones.toml".into(),
            message: format!("generated zones manifest failed schema validation: {err}"),
        });
    }
}

fn check_proof_lanes_schema(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let path = repo.join("agent/proof-lanes.toml");
    if !path.is_file() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        diagnostics.push(Diagnostic {
            check_id: "proof-lanes-read".into(),
            severity: "medium".into(),
            path: "agent/proof-lanes.toml".into(),
            message: "could not read agent/proof-lanes.toml".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_proof_lanes_toml_text(repo, &text) {
        diagnostics.push(Diagnostic {
            check_id: "proof-lanes-schema".into(),
            severity: "medium".into(),
            path: "agent/proof-lanes.toml".into(),
            message: format!("proof lanes manifest failed schema validation: {err}"),
        });
    }
}

fn check_standard_version_schema(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let path = repo.join("agent/standard-version.toml");
    if !path.is_file() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        diagnostics.push(Diagnostic {
            check_id: "standard-version-read".into(),
            severity: "medium".into(),
            path: "agent/standard-version.toml".into(),
            message: "could not read agent/standard-version.toml".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_standard_version_toml_text(repo, &text) {
        diagnostics.push(Diagnostic {
            check_id: "standard-version-schema".into(),
            severity: "medium".into(),
            path: "agent/standard-version.toml".into(),
            message: format!("standard-version manifest failed schema validation: {err}"),
        });
    }
}

fn check_lockfiles(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let has_manifest = repo.join("Cargo.toml").exists() || repo.join("package.json").exists();
    let has_lock = repo.join("Cargo.lock").exists()
        || repo.join("package-lock.json").exists()
        || repo.join("pnpm-lock.yaml").exists()
        || repo.join("yarn.lock").exists();
    if has_manifest && !has_lock {
        diagnostics.push(Diagnostic {
            check_id: "lockfile".into(),
            severity: "high".into(),
            path: ".".into(),
            message: "package manifests exist without a committed lockfile".into(),
        });
    }
}

fn check_root_score_artifacts(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    for rel in ["repo-score.json", "repo-score.md"] {
        if repo.join(rel).exists() {
            diagnostics.push(Diagnostic {
                check_id: "root-score-artifact".into(),
                severity: "medium".into(),
                path: rel.into(),
                message: "score artifacts belong under agent/".into(),
            });
        }
    }
}

fn check_stale_score(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let score = local_state::preferred_repo_path(
        repo,
        local_state::SCORE_JSON,
        Some(local_state::LEGACY_SCORE_JSON),
    );
    let policy = repo.join("agent/audit-policy.toml");
    if let (Ok(score_meta), Ok(policy_meta)) = (score.metadata(), policy.metadata()) {
        if let (Ok(score_time), Ok(policy_time)) = (score_meta.modified(), policy_meta.modified()) {
            if score_time < policy_time {
                diagnostics.push(Diagnostic {
                    check_id: "stale-score".into(),
                    severity: "medium".into(),
                    path: local_state::SCORE_JSON.into(),
                    message: "score file is older than audit policy".into(),
                });
            }
        }
    }
}

fn check_local_path_leaks(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let roots = ["AGENTS.md", "agent", "docs", ".github"];
    for rel in roots {
        let path = repo.join(rel);
        if !path.exists() {
            continue;
        }
        for file in files_under(&path) {
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            if text.contains("/Users/") || text.contains("C:\\Users\\") {
                diagnostics.push(Diagnostic {
                    check_id: "local-path-leak".into(),
                    severity: "medium".into(),
                    path: display_rel(repo, &file),
                    message: "local absolute path appears in agent-facing text".into(),
                });
            }
        }
    }
}

fn check_echo_only_proof(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    for file in files_under(&repo.join(".github/workflows")) {
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        if text
            .lines()
            .any(|line| line.trim_start().starts_with("run: echo "))
        {
            diagnostics.push(Diagnostic {
                check_id: "echo-proof".into(),
                severity: "high".into(),
                path: display_rel(repo, &file),
                message: "workflow has echo-only proof instead of an operational command".into(),
            });
        }
    }
}

fn check_security_tools(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    if !repo.join("Justfile").exists() && !repo.join(".github/workflows").exists() {
        return;
    }
    for tool in ["gitleaks", "cargo-audit", "npm", "syft", "zizmor"] {
        if command_exists(tool) {
            continue;
        }
        diagnostics.push(Diagnostic {
            check_id: format!("security-tool:{tool}"),
            severity: "low".into(),
            path: "Justfile".into(),
            message: format!(
                "security lane tool `{tool}` is not installed; treat it as advisory outside strict mode"
            ),
        });
    }
}

fn check_committed_ux_artifacts(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    for rel in ["ux-qa-artifacts", "playwright-report", "test-results"] {
        if repo.join(rel).exists() {
            diagnostics.push(Diagnostic {
                check_id: "committed-ux-artifact".into(),
                severity: "medium".into(),
                path: rel.into(),
                message: "UX proof artifacts should be generated under target/ or CI artifacts"
                    .into(),
            });
        }
    }
}

fn check_legacy_paper_sources(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    if repo.join("paper/sections").exists()
        && repo.join("paper/tex").exists()
        && !legacy_paper_sources_marked(repo)
    {
        diagnostics.push(Diagnostic {
            check_id: "legacy-paper-source".into(),
            severity: "medium".into(),
            path: "paper/sections".into(),
            message: "legacy Markdown paper sections coexist with canonical TeX sources".into(),
        });
    }
}

fn check_severity_discipline(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    for root in prose_scan_roots(repo) {
        for file in files_under(&root) {
            if !is_prose_file(&file) {
                continue;
            }
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            scan_severity_prose(repo, &file, &text, diagnostics);
        }
    }
}

fn prose_scan_roots(repo: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for rel in [
        "README.md",
        "CHANGELOG.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
        "SUPPORT.md",
    ] {
        let path = repo.join(rel);
        if path.exists() {
            roots.push(path);
        }
    }
    let docs = repo.join("docs");
    if docs.exists() {
        roots.push(docs);
    }
    roots
}

fn is_prose_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default(),
        "md" | "markdown" | "txt" | "adoc"
    )
}

fn scan_severity_prose(repo: &Path, path: &Path, text: &str, diagnostics: &mut Vec<Diagnostic>) {
    let lines: Vec<&str> = text.lines().collect();
    let mut in_fence = false;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if is_trailer_line(trimmed) {
            continue;
        }
        if !contains_severity_claim(trimmed) {
            continue;
        }
        if is_bare_critical_word(trimmed) {
            continue;
        }
        if trailer_window_has_justification(&lines, idx) {
            continue;
        }
        if let Some(value) = blocker_type_value(trimmed) {
            let _ = parse_failure_mode(value);
        }
        diagnostics.push(Diagnostic {
            check_id: "severity-discipline".into(),
            severity: "medium".into(),
            path: display_rel(repo, path),
            message: "severity claim should carry Severity-Justified: or Blocker-Type: evidence"
                .into(),
        });
    }
}

fn contains_severity_claim(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if lower.contains("catastrophic") {
        return true;
    }
    if !(lower.contains("critical") || lower.contains("high")) {
        return false;
    }
    [
        "risk",
        "severity",
        "issue",
        "outage",
        "blocker",
        "failure",
        "regression",
        "impact",
        "problem",
        "bug",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn is_bare_critical_word(line: &str) -> bool {
    matches!(line.to_ascii_lowercase().as_str(), "critical" | "high")
}

fn is_trailer_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("severity-justified:") || lower.starts_with("blocker-type:")
}

fn blocker_type_value(line: &str) -> Option<&str> {
    line.split_once(':').and_then(|(left, right)| {
        if left.trim().eq_ignore_ascii_case("blocker-type") {
            Some(right.trim())
        } else {
            None
        }
    })
}

fn trailer_window_has_justification(lines: &[&str], idx: usize) -> bool {
    let start = idx.saturating_sub(2);
    let end = (idx + 2).min(lines.len().saturating_sub(1));
    for line in &lines[start..=end] {
        if is_trailer_line(line.trim()) {
            return true;
        }
    }
    false
}

fn legacy_paper_sources_marked(repo: &Path) -> bool {
    fs::read_to_string(repo.join("paper/sections/README.md"))
        .map(|text| text.contains("legacy-only") && text.contains("paper/tex/"))
        .unwrap_or(false)
}

fn check_proof_ledger(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let receipts_dir = repo.join("target/jankurai/proof-receipts");
    if receipts_dir.is_dir() {
        let Ok(entries) = fs::read_dir(&receipts_dir) else {
            return;
        };
        let head_now = git_head_short(repo);
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let rel = display_rel(repo, &path);
            let Ok(text) = fs::read_to_string(&path) else {
                diagnostics.push(Diagnostic {
                    check_id: "proof-receipt-read".into(),
                    severity: "medium".into(),
                    path: rel,
                    message: "could not read proof receipt".into(),
                });
                continue;
            };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
                diagnostics.push(Diagnostic {
                    check_id: "proof-receipt-json".into(),
                    severity: "medium".into(),
                    path: rel,
                    message: "proof receipt is not valid JSON".into(),
                });
                continue;
            };
            if let Err(err) = validation::validate_value(repo, ArtifactSchema::ProofReceipt, &value)
            {
                diagnostics.push(Diagnostic {
                    check_id: "proof-receipt-schema".into(),
                    severity: "medium".into(),
                    path: rel,
                    message: format!("proof receipt failed schema validation: {err}"),
                });
            } else if let (Some(stored), Some(now)) = (
                value.get("git_head").and_then(|v| v.as_str()),
                head_now.as_deref(),
            ) {
                if !stored.is_empty()
                    && stored != "unknown"
                    && !now.is_empty()
                    && now != "unknown"
                    && stored != now
                {
                    diagnostics.push(Diagnostic {
                        check_id: "proof-receipt-stale-head".into(),
                        severity: "low".into(),
                        path: rel,
                        message: format!(
                            "receipt git_head {stored} differs from current HEAD {now}"
                        ),
                    });
                }
            }
        }
    }

    let index_path = repo.join("target/jankurai/evidence-index.json");
    if index_path.is_file() {
        let rel = display_rel(repo, &index_path);
        let Ok(text) = fs::read_to_string(&index_path) else {
            diagnostics.push(Diagnostic {
                check_id: "evidence-index-read".into(),
                severity: "medium".into(),
                path: rel,
                message: "could not read evidence index".into(),
            });
            return;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            diagnostics.push(Diagnostic {
                check_id: "evidence-index-json".into(),
                severity: "medium".into(),
                path: rel,
                message: "evidence index is not valid JSON".into(),
            });
            return;
        };
        if let Err(err) = validation::validate_value(repo, ArtifactSchema::EvidenceIndex, &value) {
            diagnostics.push(Diagnostic {
                check_id: "evidence-index-schema".into(),
                severity: "medium".into(),
                path: rel,
                message: format!("evidence index failed schema validation: {err}"),
            });
        } else if let (Some(stored), Some(now)) = (
            value.get("git_head").and_then(|v| v.as_str()),
            git_head_short(repo).as_deref(),
        ) {
            if !stored.is_empty()
                && stored != "unknown"
                && !now.is_empty()
                && now != "unknown"
                && stored != now
            {
                diagnostics.push(Diagnostic {
                    check_id: "evidence-index-stale-head".into(),
                    severity: "low".into(),
                    path: rel,
                    message: format!(
                        "evidence index git_head {stored} differs from current HEAD {now}"
                    ),
                });
            }
        }
    }
}

fn check_security_evidence(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let evidence_path = repo.join("target/jankurai/security/evidence.json");
    if !evidence_path.is_file() {
        return;
    }
    let rel = display_rel(repo, &evidence_path);
    let Ok(text) = fs::read_to_string(&evidence_path) else {
        diagnostics.push(Diagnostic {
            check_id: "security-evidence-read".into(),
            severity: "medium".into(),
            path: rel,
            message: "could not read security evidence JSON".into(),
        });
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        diagnostics.push(Diagnostic {
            check_id: "security-evidence-json".into(),
            severity: "medium".into(),
            path: rel,
            message: "security evidence is not valid JSON".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_value(repo, ArtifactSchema::SecurityEvidence, &value) {
        diagnostics.push(Diagnostic {
            check_id: "security-evidence-schema".into(),
            severity: "medium".into(),
            path: rel,
            message: format!("security evidence failed schema validation: {err}"),
        });
        return;
    }
    if let (Some(stored), Some(now)) = (
        value.get("git_head").and_then(|v| v.as_str()),
        git_head_short(repo).as_deref(),
    ) {
        if !stored.is_empty()
            && stored != "unknown"
            && !now.is_empty()
            && now != "unknown"
            && stored != now
        {
            diagnostics.push(Diagnostic {
                check_id: "security-evidence-stale-head".into(),
                severity: "low".into(),
                path: rel,
                message: format!(
                    "security evidence git_head {stored} differs from current HEAD {now}"
                ),
            });
        }
    }
}

fn check_json_artifact(
    repo: &Path,
    diagnostics: &mut Vec<Diagnostic>,
    rel_path: &str,
    schema: ArtifactSchema,
    check_read: &str,
    check_json: &str,
    check_schema: &str,
) {
    let path = repo.join(rel_path);
    if !path.is_file() {
        return;
    }
    let rel = display_rel(repo, &path);
    let Ok(text) = fs::read_to_string(&path) else {
        diagnostics.push(Diagnostic {
            check_id: check_read.into(),
            severity: "medium".into(),
            path: rel,
            message: "could not read JSON".into(),
        });
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        diagnostics.push(Diagnostic {
            check_id: check_json.into(),
            severity: "medium".into(),
            path: rel,
            message: "file is not valid JSON".into(),
        });
        return;
    };
    if let Err(err) = validation::validate_value(repo, schema, &value) {
        diagnostics.push(Diagnostic {
            check_id: check_schema.into(),
            severity: "medium".into(),
            path: rel,
            message: format!("failed schema validation: {err}"),
        });
    }
}

fn git_head_short(repo: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn check_receipt_exports(repo: &Path, diagnostics: &mut Vec<Diagnostic>) {
    if !repo.join("target/jankurai").exists() {
        diagnostics.push(Diagnostic {
            check_id: "receipt-export".into(),
            severity: "low".into(),
            path: "target/jankurai".into(),
            message: "no local receipt/export directory exists yet".into(),
        });
    }
}

fn ok(path: &str, message: &str) -> Diagnostic {
    Diagnostic {
        check_id: format!("file:{path}"),
        severity: "ok".into(),
        path: path.into(),
        message: message.into(),
    }
}

fn enrich_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<DoctorDiagnostic> {
    diagnostics.into_iter().map(enrich_diagnostic).collect()
}

fn enrich_diagnostic(diagnostic: Diagnostic) -> DoctorDiagnostic {
    let kind = diagnostic_kind(&diagnostic);
    let environment_sensitive = matches!(kind, DiagnosticKind::Tool | DiagnosticKind::Freshness);
    let strictly_blocking = severity_rank(&diagnostic.severity) <= severity_rank("high")
        && !matches!(kind, DiagnosticKind::Other | DiagnosticKind::Export);
    let common_fixes = common_fixes(&diagnostic, &kind);
    DoctorDiagnostic {
        check_id: diagnostic.check_id,
        severity: diagnostic.severity,
        path: diagnostic.path,
        message: diagnostic.message,
        kind,
        environment_sensitive,
        strictly_blocking,
        common_fixes,
    }
}

fn diagnostic_kind(diagnostic: &Diagnostic) -> DiagnosticKind {
    let check_id = diagnostic.check_id.as_str();
    if check_id.starts_with("file:") {
        return DiagnosticKind::FileMissing;
    }
    if check_id.contains("security-tool:") {
        return DiagnosticKind::Tool;
    }
    if check_id.contains("stale") {
        return DiagnosticKind::Freshness;
    }
    if check_id.contains("receipt") || check_id.contains("evidence") {
        return DiagnosticKind::Receipt;
    }
    if check_id.contains("policy") {
        return DiagnosticKind::Policy;
    }
    if check_id.contains("schema") {
        return DiagnosticKind::Schema;
    }
    if check_id.contains("lockfile") {
        return DiagnosticKind::Lockfile;
    }
    if check_id.contains("echo-proof") {
        return DiagnosticKind::Workflow;
    }
    if check_id.contains("export") {
        return DiagnosticKind::Export;
    }
    if check_id.contains("severity") {
        return DiagnosticKind::Policy;
    }
    DiagnosticKind::Other
}

fn common_fixes(diagnostic: &Diagnostic, kind: &DiagnosticKind) -> Vec<String> {
    match kind {
        DiagnosticKind::FileMissing => vec![
            format!(
                "create `{}` or regenerate it from the source command",
                diagnostic.path
            ),
            "re-run the owning lane and validate the receipt".into(),
        ],
        DiagnosticKind::Schema => vec![
            "fix the JSON/TOML shape to match the schema".into(),
            "re-run `cargo test -p jankurai`".into(),
        ],
        DiagnosticKind::Policy => vec![
            if diagnostic.check_id.contains("severity") {
                "add a Severity-Justified: or Blocker-Type: trailer that names the failure mode"
                    .into()
            } else {
                "update the policy file so the runtime and schema agree".into()
            },
            "re-run `doctor` after the edit".into(),
        ],
        DiagnosticKind::Tool => vec![
            "install the missing tool or mark it advisory in policy".into(),
            "re-run the security lane".into(),
        ],
        DiagnosticKind::Freshness => vec![
            "regenerate the stale artifact from its source command".into(),
            "re-run the proof or audit command that owns the file".into(),
        ],
        DiagnosticKind::Receipt => vec![
            "regenerate the proof evidence with the current repo state".into(),
            "re-run the proof command and compare digests".into(),
        ],
        DiagnosticKind::Workflow => vec![
            "replace the echo-only step with the real command".into(),
            "re-run the workflow or lane locally".into(),
        ],
        DiagnosticKind::Lockfile => {
            vec!["commit the missing lockfile for the package manager in use".into()]
        }
        DiagnosticKind::Export => {
            vec!["write the receipt or export under `target/jankurai/receipts/`".into()]
        }
        DiagnosticKind::Other => Vec::new(),
    }
}

fn files_under(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let mut files = vec![];
    let Ok(entries) = fs::read_dir(path) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

fn display_rel(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn severity_rank(value: &str) -> i32 {
    match value {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        "ok" => 4,
        _ => 1,
    }
}

fn command_exists(tool: &str) -> bool {
    std::process::Command::new("bash")
        .arg("-lc")
        .arg(format!("command -v {tool} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn write_receipt(repo: &Path, action: &str, diagnostics: &[DoctorDiagnostic]) -> Result<()> {
    let dir = repo.join("target/jankurai/receipts");
    fs::create_dir_all(&dir)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = dir.join(format!("{action}-{now}.json"));
    let payload = serde_json::json!({
        "action": action,
        "created_at": now,
        "diagnostics": diagnostics,
    });
    validation::validate_value(repo, ArtifactSchema::DoctorReceipt, &payload)?;
    fs::write(path, serde_json::to_string_pretty(&payload)?)?;
    Ok(())
}
