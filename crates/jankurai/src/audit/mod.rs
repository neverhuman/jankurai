pub mod analyzers;
pub mod baseline;
pub mod boundaries_artifact;
pub mod boundary_reclassification;
pub mod caps;
pub mod ci_local_parity;
pub mod copy_code;
pub mod copy_code_cross_check;
pub mod coverage;
pub mod evidence;
pub mod file_kinds;
pub mod finding_builder;
pub mod fix_queue;
pub mod fs;
pub mod fs_policy;
pub mod helpers;
pub mod language_rules;
pub mod policy;
pub mod profile_structure;
pub mod proofbind_artifact;
pub mod prose;
pub mod repo_rot;
pub mod rule_analyzer;
pub mod rules;
pub mod save_gate;
pub mod scan;
pub mod security_artifact;
pub mod ux_artifact;
pub mod web_security;
pub mod zyal;

use crate::model::*;
use anyhow::Result;
use caps::{caps_applied, CAPS};
use finding_builder::{dimension_soft_route, FindingBuilder};
use helpers::AuditContext;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default)]
pub struct AuditOptions {
    pub self_audit: bool,
    pub proof_receipts: Option<String>,
    pub changed_fast: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct AuditTimings {
    pub total_ms: u128,
    pub phases: Vec<AuditTimingPhase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditTimingPhase {
    pub name: String,
    pub elapsed_ms: u128,
}

impl AuditTimings {
    pub fn record_duration(&mut self, name: impl Into<String>, duration: Duration) {
        self.phases.push(AuditTimingPhase {
            name: name.into(),
            elapsed_ms: duration.as_millis(),
        });
    }

    pub fn record_ms(&mut self, name: impl Into<String>, elapsed_ms: u128) {
        self.phases.push(AuditTimingPhase {
            name: name.into(),
            elapsed_ms,
        });
    }
}

pub fn run_audit(root: &Path, changed: &[PathBuf]) -> Result<Report> {
    run_audit_with_options(root, changed, AuditOptions::default())
}

pub fn run_audit_with_options(
    root: &Path,
    changed: &[PathBuf],
    options: AuditOptions,
) -> Result<Report> {
    Ok(run_audit_timed_with_options(root, changed, options)?.0)
}

pub fn run_audit_timed_with_options(
    root: &Path,
    changed: &[PathBuf],
    options: AuditOptions,
) -> Result<(Report, AuditTimings)> {
    let started = Instant::now();
    let mut timings = AuditTimings::default();
    let scope_paths: Vec<String> = changed
        .iter()
        .filter_map(|p| normalize_changed_path(root, p))
        .collect();
    let inventory_options = fs::InventoryOptions::from_policy(root);
    let inventory_started = Instant::now();
    let inventory = if options.changed_fast {
        let paths = changed_fast_inventory_paths(&scope_paths);
        fs::inventory_paths_detailed(root, &paths, &inventory_options)?
    } else {
        fs::inventory_repo_detailed(root, &inventory_options)?
    };
    timings.record_ms("walk", inventory.timings.walk_ms);
    timings.record_ms("metadata", inventory.timings.metadata_ms);
    timings.record_ms("text_capture", inventory.timings.text_capture_ms);
    timings.record_duration("inventory", inventory_started.elapsed());
    run_audit_inner(
        root,
        inventory.files,
        scope_paths,
        &options,
        changed,
        started,
        timings,
    )
}

/// Options for [`run_candidate_audit`]: a single unsaved file change overlaid
/// onto a scoped inventory so the audit engine can score it before it lands.
#[derive(Debug, Clone)]
pub struct CandidateAuditOptions {
    /// The candidate file change to overlay onto the inventory.
    pub overlay: fs::CandidateOverlay,
    /// Repo-relative paths the audit should score (the candidate plus any
    /// sibling files needed for context). Control files are added automatically.
    pub scope_paths: Vec<String>,
    /// Shared audit options.
    pub options: AuditOptions,
}

/// Audits a single candidate file change without writing it to disk. The
/// inventory is walked scoped to the candidate plus control files, the
/// candidate bytes are overlaid in memory, and the full analyzer pipeline runs
/// against that overlaid inventory.
pub fn run_candidate_audit(
    root: &Path,
    opts: CandidateAuditOptions,
) -> Result<(Report, AuditTimings)> {
    let started = Instant::now();
    let mut timings = AuditTimings::default();
    let inventory_options = fs::InventoryOptions::from_policy(root);
    let inventory_started = Instant::now();
    let walk_paths = changed_fast_inventory_paths(&opts.scope_paths);
    let mut inventory = fs::inventory_paths_detailed(root, &walk_paths, &inventory_options)?;
    fs::apply_overlay(
        &mut inventory.files,
        &opts.overlay,
        inventory_options.text_capture_chars,
    );
    timings.record_ms("walk", inventory.timings.walk_ms);
    timings.record_ms("metadata", inventory.timings.metadata_ms);
    timings.record_ms("text_capture", inventory.timings.text_capture_ms);
    timings.record_duration("inventory", inventory_started.elapsed());
    run_audit_inner(
        root,
        inventory.files,
        opts.scope_paths,
        &opts.options,
        &[],
        started,
        timings,
    )
}

fn run_audit_inner(
    root: &Path,
    all_files: Vec<FileInfo>,
    scope_paths: Vec<String>,
    options: &AuditOptions,
    changed: &[PathBuf],
    started: Instant,
    mut timings: AuditTimings,
) -> Result<(Report, AuditTimings)> {
    let scope_files = if scope_paths.is_empty() {
        all_files.clone()
    } else {
        all_files
            .iter()
            .filter(|f| path_matches_scope(&f.rel_path, &scope_paths))
            .cloned()
            .collect()
    };

    let index_started = Instant::now();
    let base_ctx = AuditContext {
        root: root.to_path_buf(),
        all_files,
        scope_files,
        scope_paths,
        self_audit: options.self_audit,
        boundary_reclassifications: vec![],
        copy_code: None,
    };
    let boundary_reclassifications = boundary_reclassification::evaluate(&base_ctx);
    let copy_code = copy_code::scan_files(
        root,
        &base_ctx.all_files,
        copy_code::CopyCodeOptions::default(),
    );
    let ctx = AuditContext {
        boundary_reclassifications,
        copy_code: Some(copy_code.clone()),
        ..base_ctx
    };
    timings.record_duration("index_build", index_started.elapsed());
    let profile_structure = profile_structure::analyze(&ctx);
    let analyzers_started = Instant::now();
    let dimensions = analyzers::all_dimensions(&ctx, &profile_structure);
    timings.record_duration("analyzers", analyzers_started.elapsed());
    let coverage_ingest = coverage::load_score_ingest(root);
    let raw_score = dimensions
        .iter()
        .map(|d| d.weighted_points)
        .sum::<f64>()
        .round() as i32;
    let destructive_sql_hits = scan::destructive_sql_hits(&ctx);
    let mut caps_applied = caps_applied(&ctx, !destructive_sql_hits.is_empty());
    coverage::apply_coverage_caps(&mut caps_applied, &coverage_ingest);
    let final_score = caps_applied
        .iter()
        .filter_map(|c| CAPS.iter().find(|(id, _)| id == c).map(|(_, m)| *m))
        .fold(raw_score, |acc, cap| acc.min(cap));
    let policy = load_policy(root)?;
    let ux_qa = attach_ux_report_artifact(root, analyzers::ux_qa_status(&ctx));
    let security_evidence_artifact = security_artifact::load_report_summary(root);
    let tool_adoption = analyzers::tool_adoption::status(&ctx);
    let findings_started = Instant::now();
    let mut findings = build_findings(
        &ctx,
        &dimensions,
        &profile_structure,
        &caps_applied,
        final_score,
        policy.minimum_score,
        ux_qa.artifact.as_ref(),
        security_evidence_artifact.as_ref(),
        &destructive_sql_hits,
    );
    findings.extend(coverage::score_findings(&coverage_ingest));
    let agent_fix_queue = fix_queue::build_agent_fix_queue(&findings);
    timings.record_duration("findings", findings_started.elapsed());
    let decision = report_decision(final_score, &findings, &policy);
    let (observed_conformance_level, conformance_decision, conformance_blockers) =
        conformance_summary(&decision, &findings);
    let git = git_summary(root, changed);
    let dirty_worktree = git.dirty_worktree.unwrap_or(false);
    let proof_receipts = load_proof_receipts(root, options.proof_receipts.as_deref())?;
    let versions = report_versions(root);
    let mut report = Report {
        report_fingerprint: "sha256:pending".into(),
        input_fingerprint: input_fingerprint(&ctx),
        policy_fingerprint: file_fingerprint(&root.join("agent/audit-policy.toml"))
            .unwrap_or_else(missing_sha256),
        manifest_fingerprints: manifest_fingerprints(root),
        dirty_worktree,
        generated_at: started_at(),
        schema_url: "schemas/repo-score.schema.json".into(),
        standard: "jankurai".into(),
        standard_version: versions.standard_version,
        auditor_version: versions.auditor_version,
        schema_version: versions.schema_version,
        paper_edition: versions.paper_edition,
        target_stack_id: versions.target_stack_id,
        target_stack: TARGET_STACK.into(),
        claimed_conformance_level: "HL3".into(),
        observed_conformance_level,
        conformance_decision,
        conformance_blockers,
        repo: root.display().to_string(),
        run_id: Some(run_id()),
        started_at: Some(started_at()),
        elapsed_ms: Some(started.elapsed().as_millis()),
        scope: Scope {
            mode: if options.changed_fast {
                "changed-fast".into()
            } else if ctx.scope_paths.is_empty() {
                "full".into()
            } else {
                "changed".into()
            },
            paths: ctx.scope_paths.clone(),
        },
        score: final_score,
        raw_score,
        decision: Some(decision),
        git: Some(git),
        policy: Some(policy),
        proof_receipts,
        caps_applied,
        hard_rules: CAPS
            .iter()
            .map(|(id, max_score)| HardRule {
                id: id.to_string(),
                max_score: *max_score,
            })
            .collect(),
        dimensions,
        ux_qa,
        tool_adoption,
        security_evidence: SecurityEvidenceReadiness {
            artifact: security_evidence_artifact,
        },
        boundaries: BoundariesReadiness {
            artifact: boundaries_artifact::load_manifest_summary(root),
            reclassifications: ctx.boundary_reclassifications.clone(),
        },
        copy_code: Some(copy_code),
        profile_structure: profile_structure.clone(),
        vibe_coverage: crate::commands::vibe::audit_summary(root),
        coverage_evidence: coverage_ingest.summary,
        findings,
        agent_fix_queue,
    };
    report.report_fingerprint = report_fingerprint(&report);
    timings.total_ms = started.elapsed().as_millis();
    Ok((report, timings))
}

struct ReportVersions {
    standard_version: String,
    auditor_version: String,
    schema_version: String,
    paper_edition: String,
    target_stack_id: String,
}

fn report_versions(root: &Path) -> ReportVersions {
    let mut versions = ReportVersions {
        standard_version: STANDARD_VERSION.into(),
        auditor_version: AUDITOR_VERSION.into(),
        schema_version: SCHEMA_VERSION.into(),
        paper_edition: PAPER_EDITION.into(),
        target_stack_id: TARGET_STACK_ID.into(),
    };
    if let Ok(text) = std::fs::read_to_string(root.join("agent/standard-version.toml")) {
        if let Ok(value) = toml::from_str::<toml::Value>(&text) {
            versions.standard_version =
                toml_string(&value, "standard_version").unwrap_or(versions.standard_version);
            versions.auditor_version =
                toml_string(&value, "auditor_version").unwrap_or(versions.auditor_version);
            versions.schema_version =
                toml_string(&value, "schema_version").unwrap_or(versions.schema_version);
            versions.paper_edition =
                toml_string(&value, "paper_edition").unwrap_or(versions.paper_edition);
            versions.target_stack_id =
                toml_string(&value, "target_stack").unwrap_or(versions.target_stack_id);
            return versions;
        }
    }
    if let Some(version) = standard_doc_version(root) {
        versions.standard_version = version;
    }
    versions
}

fn toml_string(value: &toml::Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(ToString::to_string)
}

fn standard_doc_version(root: &Path) -> Option<String> {
    for path in [
        root.join("agent/JANKURAI_STANDARD.md"),
        root.join("docs/agent-native-standard.md"),
    ] {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("Standard version: `") else {
                continue;
            };
            let Some((version, _)) = rest.split_once('`') else {
                continue;
            };
            if !version.trim().is_empty() {
                return Some(version.trim().to_string());
            }
        }
    }
    None
}

pub fn rebuild_agent_fix_queue(report: &mut Report) {
    report.agent_fix_queue = fix_queue::build_agent_fix_queue(&report.findings);
}

fn attach_ux_report_artifact(root: &Path, mut readiness: UxQaReadiness) -> UxQaReadiness {
    readiness.artifact = ux_artifact::load_report_summary(root);
    readiness
}

fn load_policy(root: &Path) -> Result<PolicySummary> {
    use serde::Deserialize;
    #[derive(Debug, Deserialize)]
    struct AuditPolicyFile {
        #[serde(default = "default_minimum_score")]
        minimum_score: i32,
        #[serde(default)]
        fail_on: Vec<String>,
        #[serde(default)]
        advisory_on: Vec<String>,
    }

    fn default_minimum_score() -> i32 {
        85
    }

    let path = root.join("agent/audit-policy.toml");
    let parsed = match std::fs::read_to_string(&path) {
        Ok(text) => toml::from_str::<AuditPolicyFile>(&text)
            .map_err(|err| anyhow::anyhow!("invalid audit policy {}: {err}", path.display()))?,
        Err(_) => AuditPolicyFile {
            minimum_score: default_minimum_score(),
            fail_on: vec!["critical".into(), "high".into()],
            advisory_on: vec!["medium".into(), "low".into()],
        },
    };
    validate_policy_severities("fail_on", &parsed.fail_on)?;
    validate_policy_severities("advisory_on", &parsed.advisory_on)?;
    Ok(PolicySummary {
        path: path.display().to_string(),
        minimum_score: parsed.minimum_score,
        fail_on: parsed.fail_on,
        advisory_on: parsed.advisory_on,
        mode: Some("standard".into()),
        standard_version: Some(STANDARD_VERSION.into()),
        auditor_version: Some(AUDITOR_VERSION.into()),
        schema_version: Some(SCHEMA_VERSION.into()),
        paper_edition: Some(PAPER_EDITION.into()),
        target_stack: Some(TARGET_STACK_ID.into()),
    })
}

fn validate_policy_severities(field: &str, severities: &[String]) -> Result<()> {
    for severity in severities {
        if !matches!(
            severity.as_str(),
            "critical" | "high" | "medium" | "low" | "info"
        ) {
            anyhow::bail!(
                "invalid audit policy severity `{severity}` in {field}; expected critical, high, medium, low, or info"
            );
        }
    }
    Ok(())
}

pub fn report_decision(score: i32, findings: &[Finding], policy: &PolicySummary) -> ReportDecision {
    let hard_findings = findings
        .iter()
        .filter(|f| {
            policy
                .fail_on
                .iter()
                .any(|severity| severity == &f.severity)
        })
        .count();
    let soft_findings = findings.len().saturating_sub(hard_findings);
    let passed = score >= policy.minimum_score && hard_findings == 0;
    ReportDecision {
        status: if passed { "pass".into() } else { "fail".into() },
        minimum_score: policy.minimum_score,
        passed,
        hard_findings,
        soft_findings,
        ratchet: Some(ReportRatchet {
            baseline_score: score,
            allowed_drop: 0,
            passed,
            score_delta: 0,
            baseline_report_fingerprint: missing_sha256(),
            baseline_input_fingerprint: missing_sha256(),
            baseline_policy_fingerprint: missing_sha256(),
            new_caps: vec![],
            new_hard_findings: vec![],
            policy_changed: false,
        }),
    }
}

fn conformance_summary(
    decision: &ReportDecision,
    findings: &[Finding],
) -> (String, String, Vec<String>) {
    let blockers: Vec<String> = findings
        .iter()
        .filter(|finding| matches!(finding.severity.as_str(), "critical" | "high"))
        .map(|finding| {
            format!(
                "{} on {}",
                finding.rule_id.as_deref().unwrap_or(&finding.check_id),
                finding.path
            )
        })
        .collect();
    if decision.passed {
        ("HL3".into(), "pass".into(), blockers)
    } else if blockers.is_empty() {
        ("HL2".into(), "review".into(), blockers)
    } else {
        ("HL2".into(), "block".into(), blockers)
    }
}

fn git_summary(root: &Path, changed: &[PathBuf]) -> GitSummary {
    GitSummary {
        head: git_output(root, &["rev-parse", "--short", "HEAD"]),
        base: None,
        changed_files: changed.len(),
        mode: if changed.is_empty() {
            "full".into()
        } else {
            "changed".into()
        },
        dirty_worktree: Some(
            Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(root)
                .output()
                .ok()
                .map(|out| !out.stdout.is_empty())
                .unwrap_or(false),
        ),
    }
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn run_id() -> String {
    started_at().replace(':', "-")
}

fn started_at() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

fn file_fingerprint(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn missing_sha256() -> String {
    "sha256:0000000000000000000000000000000000000000000000000000000000000000".into()
}

fn sha256_string(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

fn display_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn manifest_fingerprints(root: &Path) -> ManifestFingerprints {
    ManifestFingerprints {
        owner_map: file_fingerprint(&root.join("agent/owner-map.json")),
        test_map: file_fingerprint(&root.join("agent/test-map.json")),
        generated_zones: file_fingerprint(&root.join("agent/generated-zones.toml")),
        boundaries: file_fingerprint(&root.join("agent/boundaries.toml")),
        proof_lanes: file_fingerprint(&root.join("agent/proof-lanes.toml")),
        standard_version: file_fingerprint(&root.join("agent/standard-version.toml")),
    }
}

fn input_fingerprint(ctx: &AuditContext) -> String {
    let mut hasher = Sha256::new();
    for file in &ctx.all_files {
        hasher.update(file.rel_path.as_bytes());
        hasher.update([0]);
        if let Some(text) = prose::fingerprint_text(file) {
            hasher.update(text.as_bytes());
        }
        hasher.update([0xff]);
    }
    format!("sha256:{:x}", hasher.finalize())
}

pub fn report_fingerprint(report: &Report) -> String {
    let mut value = serde_json::to_value(report).unwrap_or_default();
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "report_fingerprint".into(),
            serde_json::Value::String("sha256:pending".into()),
        );
    }
    let bytes = serde_json::to_vec(&value).unwrap_or_default();
    format!("sha256:{:x}", Sha256::digest(bytes))
}

// The final finding pass combines all scored evidence families into one report stream.
#[allow(clippy::too_many_arguments)]
fn build_findings(
    ctx: &AuditContext,
    dimensions: &[DimensionResult],
    profile_structure: &ProfileStructureReadiness,
    caps_applied: &[String],
    final_score: i32,
    minimum_score: i32,
    ux_artifact: Option<&UxQaReportArtifactSummary>,
    security_artifact: Option<&SecurityEvidenceArtifactSummary>,
    destructive_sql_hits: &[scan::FindingHit],
) -> Vec<Finding> {
    let dim_by_name: HashMap<_, _> = dimensions.iter().map(|d| (d.name.as_str(), d)).collect();
    let mut b = FindingBuilder::new(ctx);

    if caps_applied.contains(&"no-root-agent-instructions".into()) {
        b.add(
            "medium",
            "context",
            "AGENTS.md",
            "no root agent/developer instruction file routes contributors at the repository root",
            "add a concise root `AGENTS.md` and move deeper ownership rules into local docs",
            vec!["no root `AGENTS.md` detected".into()],
            None,
            None,
        );
    }
    if caps_applied.contains(&"no-one-command-setup-or-validation".into()) {
        b.add(
            "high",
            "proof",
            ".",
            "no one-command setup or validation lane was detected",
            "add a canonical `setup`, `check`, `test`, or `verify` lane in one root command file",
            vec!["no root setup/check/test/verify target surfaced".into()],
            None,
            None,
        );
    }
    if caps_applied.contains(&"no-deterministic-fast-lane".into()) {
        b.add("high", "proof", ".", "no deterministic fast lane was detected", "add a fast lane that runs the narrowest deterministic proof loop and keep it canonical", vec!["no fast lane markers found".into()], Some("HLT-004-UNMAPPED-PROOF"), None);
    }
    if caps_applied.contains(&"no-security-lane-on-high-risk-repo".into()) {
        b.add_with_rule("HLT-009-GENERATED-SECURITY", ".github/workflows", "high-risk repo has no explicit security lane", "add a dedicated security lane with secret scanning, dependency review, and workflow linting", vec!["no security lane markers found".into()], None, None, None);
    }
    if caps_applied.contains(&"generated-contracts-or-public-api-drift-untested".into()) {
        b.add_with_rule(
            "HLT-007-HANDWRITTEN-CONTRACT",
            "contracts/",
            "generated contracts or public API drift are not being checked",
            "generate boundary clients and gate drift with public-API or semver checks",
            vec!["contract surface exists".into()],
            None,
            None,
            None,
        );
    }
    if caps_applied.contains(&"boundary-reclassification-evidence-gap".into()) {
        for boundary in ctx.boundary_reclassifications.iter().filter(|boundary| {
            boundary.status != "passed" && !boundary.reclassified_caps.is_empty()
        }) {
            let mut evidence = vec![
                format!("boundary `{}` status `{}`", boundary.id, boundary.status),
                format!("paths: {}", boundary.paths.join(", ")),
                format!("reclassifies: {}", boundary.reclassified_caps.join(", ")),
            ];
            evidence.extend(boundary.missing_checks.iter().take(4).cloned());
            evidence.extend(boundary.failed_checks.iter().take(4).cloned());
            let path = boundary
                .evidence_artifacts
                .first()
                .map(|artifact| artifact.path.as_str())
                .unwrap_or("agent/boundaries.toml");
            b.add_with_rule_and_rerun(
                "HLT-028-BOUNDARY-EVIDENCE-GAP",
                path,
                "audited runtime boundary reclassification evidence is missing, invalid, incomplete, or failing",
                "fix the boundary evidence artifact and rerun the configured boundary proof command; do not move or broaden product assets to bypass the check",
                evidence,
                None,
                Some(boundary.id.clone()),
                Some("declared runtime boundary reclassification needs deterministic evidence before Python exception caps can be removed".into()),
                Some(&boundary.rerun_command),
            );
        }
    }
    if caps_applied.contains(&"python-direct-product-truth-or-db-ownership".into()) {
        b.add(
            "high",
            "python",
            "python/",
            "Python appears without a rare advanced-ML/data exception or owns product truth",
            "remove Python or box it under `python/ai-service` only when a dated advanced-ML/data exception exists; otherwise migrate it to Rust",
            vec!["Python must stay away from product truth, repo tooling, proof lanes, backend glue, and production DB ownership".into()],
            Some("HLT-005-PYTHON-PRODUCT-TRUTH"),
            None,
        );
    }
    if caps_applied.contains(&"no-secret-or-dependency-scanning-in-ci".into()) {
        b.add_with_rule(
            "HLT-016-SUPPLY-CHAIN-DRIFT",
            ".github/workflows",
            "no secret or dependency scanning was found in CI",
            "add secret scanning, dependency review, and SBOM or provenance checks to CI",
            vec!["no CI scan markers found".into()],
            None,
            None,
            None,
        );
    }
    if let Some(artifact) = security_artifact {
        if artifact.profile == "ci" && !artifact.wrapper_strict {
            b.add_with_rule(
                "HLT-016-SUPPLY-CHAIN-DRIFT",
                &artifact.path,
                "CI security evidence was generated without strict mode",
                "run `jankurai security run . --strict --profile ci --out target/jankurai/security/evidence.json` before the final audit",
                vec![format!("profile={} strict={}", artifact.profile, artifact.wrapper_strict)],
                None,
                None,
                Some("security-lane-nonstrict-in-ci".into()),
            );
        }
        if !artifact.blocking_commands.is_empty() {
            b.add_with_rule(
                "HLT-016-SUPPLY-CHAIN-DRIFT",
                &artifact.path,
                "required security tool evidence is skipped, failed, or missing",
                "install and run the required security tools, then regenerate target/jankurai/security/evidence.json",
                vec![
                    format!("blocking commands: {}", artifact.blocking_commands.join(", ")),
                    format!(
                        "required skipped={} failed={}",
                        artifact.required_commands_skipped, artifact.required_commands_failed
                    ),
                ],
                None,
                None,
                Some("required-security-tool-skipped-or-failed".into()),
            );
        }
        if artifact.envelope_exit_code != 0 {
            b.add_with_rule(
                "HLT-027-HUMAN-REVIEW-EVIDENCE-GAP",
                &artifact.path,
                "security evidence artifact records a blocking wrapper exit",
                "fix the failed security command and regenerate security evidence before treating the score as current",
                vec![format!("security wrapper exit_code={}", artifact.envelope_exit_code)],
                None,
                None,
                Some("security-artifact-stale-or-git-mismatch".into()),
            );
        }
    }
    if caps_applied.contains(&"no-jankurai-audit-lane-in-ci".into()) {
        b.add("high", "audit", ".github/workflows", "CI does not run the jankurai audit lane", "add a CI job that runs `jankurai . --json agent/repo-score.json --md agent/repo-score.md` and uploads both artifacts", vec!["audit output must stay JSON plus Markdown for agent repair routing".into()], None, None);
    }

    for hit in scan::manifest_parse_findings(ctx) {
        b.add(
            "high",
            "audit",
            &hit.path,
            "jankurai manifest could not be parsed",
            "fix the manifest syntax so audit policy and routing maps are authoritative",
            vec![hit.problem],
            Some("HLT-017-OPAQUE-OBSERVABILITY"),
            hit.line,
        );
    }
    for path in crate::audit::helpers::missing_owner_paths(ctx)
        .into_iter()
        .take(10)
    {
        b.add(
            "high",
            "context",
            "agent/owner-map.json",
            &format!("path `{path}` has no owner-map route"),
            "add the narrowest stable prefix for this path to `agent/owner-map.json`",
            vec![path],
            Some("HLT-003-OWNERLESS-PATH"),
            None,
        );
    }
    for path in crate::audit::helpers::missing_test_paths(ctx)
        .into_iter()
        .take(10)
    {
        b.add(
            "high",
            "proof",
            "agent/test-map.json",
            &format!("path `{path}` has no test-map proof route"),
            "add the narrowest stable prefix and runnable proof command to `agent/test-map.json`",
            vec![path],
            Some("HLT-004-UNMAPPED-PROOF"),
            None,
        );
    }

    if caps_applied.contains(&"non-optimal-product-language-found".into())
        && !helpers::non_optimal_language_hits(ctx).is_empty()
    {
        let hit = helpers::non_optimal_language_hits(ctx)[0].clone();
        b.add("high", "stack", &hit.rel_path, "runtime code uses a language outside the chosen optimal stack", "move product runtime behavior to Rust core, TypeScript web, SQL migrations, or generated contracts; Python needs a dated advanced-ML/data exception", vec![format!("{} uses `{}`", hit.rel_path, hit.suffix), TARGET_STACK.into()], None, None);
    }
    let ratio = helpers::python_ratio(ctx);
    if caps_applied.contains(&"too-much-python-in-product-surface".into()) && ratio > 0.15 {
        b.add(if ratio > 0.30 { "high" } else { "medium" }, "python", "python/ai-service", "Python is too large a share of runtime product code for this standard", "remove Python unless it is a dated advanced-ML/data exception, and move durable product truth, authz, workflows, and core behavior into Rust", vec!["Python share is above the soft cap".into()], None, None);
    }
    if !scan::todo_hits(ctx).is_empty() {
        let hit = scan::todo_hits(ctx)[0].clone();
        b.add("high", "vibe", &hit.path, "product code contains TODO/stub/unimplemented/unreachable placeholder markers", "replace placeholders with implemented behavior, typed unsupported-state errors, or a tracked exception record with docs", vec![format!("{}:{} {}", hit.path, hit.line.unwrap_or(1), hit.text)], Some("HLT-001-DEAD-MARKER"), Some(hit.line.unwrap_or(1)));
    }
    if scan::fallback_hits(ctx).len() > 1 {
        let hit = scan::fallback_hits(ctx)[0].clone();
        b.add("high", "vibe", &hit.path, "fallback soup detected in product code", "collapse fallback chains into explicit typed states with bounded retry policy, telemetry, and documented repair guidance", vec![format!("{}:{} {}", hit.path, hit.line.unwrap_or(1), hit.text)], Some("HLT-001-DEAD-MARKER"), Some(hit.line.unwrap_or(1)));
    }
    for hit in scan::future_hostile_hits(ctx) {
        b.add(
            "high",
            "vibe",
            &hit.path,
            &format!(
                "future-hostile/dead-language term `{}` appears in product/runtime code",
                hit.matched_term.clone().unwrap_or_default()
            ),
            &hit.agent_fix,
            vec![
                format!("{}:{}", hit.path, hit.line.unwrap_or(1)),
                hit.problem.clone(),
            ],
            Some("HLT-001-DEAD-MARKER"),
            hit.line,
        );
    }
    if let Some(copy_code) = ctx.copy_code.as_ref() {
        for class in copy_code.classes.iter().filter(|class| class.hard_fail) {
            if let Some(instance) = class.instances.first() {
                b.add_with_rule(
                    "HLT-043-COPY-PASTE-BAD-BEHAVIOR",
                    &instance.path,
                    &format!("copy-code hard class `{}` detected", class.id),
                    &class.recommended_action,
                    vec![
                        format!("kind={:?}", class.kind),
                        format!("language={}", class.language),
                        format!("duplicate_lines={}", class.duplicate_lines),
                        format!("duplicate_tokens={}", class.duplicate_tokens),
                        format!("duplicate_bytes={}", class.duplicate_bytes),
                        format!(
                            "instances={}",
                            class
                                .instances
                                .iter()
                                .map(|instance| format!(
                                    "{}:{}-{}",
                                    instance.path, instance.start_line, instance.end_line
                                ))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    ],
                    Some(instance.start_line),
                    instance.unit_name.clone(),
                    Some(class.reason.clone()),
                );
            }
        }
    }
    if !scan::generated_zone_issues(ctx).is_empty() {
        let hit = scan::generated_zone_issues(ctx)[0].clone();
        b.add_with_rule("HLT-002-GENERATED-MUTATION", &hit.path, "generated zone is not protected strongly enough against hand edits", "add `agent/generated-zones.toml`, require generated/do-not-edit markers, and route repairs to the source contract", vec![hit.problem.clone()], hit.line, None, None);
    }
    for hit in scan::generated_zone_manifest_metadata_issues(ctx) {
        b.add(
            "high",
            "generated",
            &hit.path,
            "generated zone manifest lacks reproducibility metadata",
            "declare non-empty `path`, `source`, and `command` for every `[[zone]]` in `agent/generated-zones.toml`",
            vec![hit.problem.clone()],
            Some("HLT-002-GENERATED-MUTATION"),
            hit.line,
        );
    }
    if !scan::wrong_layer_db_hits(ctx).is_empty() {
        let hit = scan::wrong_layer_db_hits(ctx)[0].clone();
        b.add_with_rule("HLT-006-DIRECT-DB-WRONG-LAYER", &hit.path, "direct database access appears in a wrong layer", "move SQL and DB clients to `crates/adapters` or `db/`; expose typed application/domain APIs upward", vec![hit.problem.clone()], hit.line, None, None);
    }
    for cell in profile_structure
        .cells
        .iter()
        .filter(|cell| cell.applicable)
    {
        if cell.status == "noncanonical" {
            let detected_path = cell
                .detected_paths
                .first()
                .cloned()
                .unwrap_or_else(|| cell.canonical_path.clone());
            b.add_with_rule(
                "HLT-038-REFERENCE-PROFILE-STRUCTURE-GAP",
                &detected_path,
                &format!(
                    "reference-profile cell `{}` is detected at a noncanonical path",
                    cell.id
                ),
                &cell.agent_fix,
                vec![
                    format!("canonical_path={}", cell.canonical_path),
                    format!(
                        "detected_paths={}",
                        if cell.detected_paths.is_empty() {
                            "-".into()
                        } else {
                            cell.detected_paths.join(", ")
                        }
                    ),
                    format!(
                        "aliases={}",
                        if cell.aliases.is_empty() {
                            "-".into()
                        } else {
                            cell.aliases.join(", ")
                        }
                    ),
                    format!("guidance_status={}", cell.guidance_status),
                    format!("owner={}", cell.owner),
                    format!("proof_lane={}", cell.proof_lane),
                ],
                None,
                None,
                None,
            );
        }
        if cell.guidance_status == "missing" {
            b.add_with_rule(
                "HLT-038-REFERENCE-PROFILE-STRUCTURE-GAP",
                &cell.canonical_path,
                &format!(
                    "reference-profile cell `{}` lacks local AGENTS.md guidance",
                    cell.id
                ),
                &format!(
                    "add `{}` with owns / forbidden / proof lane guidance",
                    cell.canonical_path.trim_end_matches('/').to_string() + "/AGENTS.md"
                ),
                vec![
                    format!("canonical_path={}", cell.canonical_path),
                    format!(
                        "detected_paths={}",
                        if cell.detected_paths.is_empty() {
                            "-".into()
                        } else {
                            cell.detected_paths.join(", ")
                        }
                    ),
                    format!("guidance_status={}", cell.guidance_status),
                    format!("owner={}", cell.owner),
                    format!("proof_lane={}", cell.proof_lane),
                ],
                None,
                None,
                None,
            );
        }
    }

    // AST / Graph Pilot findings
    if !crate::audit::analyzers::ast::run_ast_pilot(ctx).is_empty() {
        let hit = crate::audit::analyzers::ast::run_ast_pilot(ctx)[0].clone();
        b.add(
            "high",
            "boundary",
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            vec![hit.text.clone()],
            Some("HLT-006-DIRECT-DB-WRONG-LAYER"), // Or maybe a more general boundary rule ID
            hit.line,
        );
    }
    if caps_applied.contains(&"missing-web-e2e-lane".into()) {
        b.add("high", "test", "apps/web", "web surface lacks a Playwright/Cypress e2e lane", "add Playwright e2e tests for critical user flows and wire them into the fast or CI proof map", vec!["web surface detected".into()], Some("HLT-013-RENDERED-UX-GAP"), None);
    }
    if caps_applied.contains(&"missing-rendered-ux-qa-lane".into()) {
        b.add("high", "ux-qa", "apps/web", "web surface lacks layered rendered UX QA evidence", "add Storybook state coverage, Playwright screenshots, visual review or `@jankurai/ux-qa`, accessibility scans, CLS checks, generated mocks, and design tokens", vec!["rendered UX QA lane missing".into()], Some("HLT-013-RENDERED-UX-GAP"), None);
    }
    if let Some(art) = ux_artifact {
        let missing_non_a11y_artifacts = art
            .missing_artifact_kinds
            .iter()
            .filter(|kind| kind.as_str() != "accessibility")
            .cloned()
            .collect::<Vec<_>>();
        if art.reports_missing_required_states > 0 || !missing_non_a11y_artifacts.is_empty() {
            let mut evidence = vec![format!(
                "{} validated reports; {} report(s) missing required states",
                art.report_count, art.reports_missing_required_states
            )];
            if !art.missing_state_names.is_empty() {
                evidence.push(format!(
                    "missing states: {}",
                    art.missing_state_names.join(", ")
                ));
            }
            if !missing_non_a11y_artifacts.is_empty() {
                evidence.push(format!(
                    "missing artifacts: {}",
                    missing_non_a11y_artifacts.join(", ")
                ));
            }
            b.add(
                "high",
                "ux-qa",
                &art.path,
                "validated UX QA evidence is missing required state or artifact coverage",
                "complete the configured route state matrix and emit required screenshot or ARIA artifacts before treating UX proof as complete",
                evidence,
                Some("HLT-013-RENDERED-UX-GAP"),
                None,
            );
        }
        if art.visual_baseline_review > 0 || art.visual_baseline_block > 0 {
            b.add(
                "high",
                "ux-qa",
                &art.path,
                "validated UX QA evidence has visual baseline gaps",
                "review or block the changed visual baseline evidence and regenerate the baseline artifacts before treating the surface as proven",
                vec![
                    format!(
                        "visual baseline missing/changed: {}/{}",
                        art.visual_baseline_missing, art.visual_baseline_changed
                    ),
                    format!(
                        "visual baseline review/block: {}/{}",
                        art.visual_baseline_review, art.visual_baseline_block
                    ),
                    format!(
                        "artifact fingerprints: {}",
                        art.artifact_fingerprint_count
                    ),
                ],
                Some("HLT-013-RENDERED-UX-GAP"),
                None,
            );
        }
        if art.accessibility_violation_total > 0
            || art.reports_missing_required_accessibility_artifact > 0
        {
            b.add(
                "high",
                "ux-qa",
                &art.path,
                "validated UX QA evidence has accessibility gaps",
                "fix axe accessibility violations and emit the required accessibility artifact for every configured report",
                vec![
                    format!("accessibility violations: {}", art.accessibility_violation_total),
                    format!("accessibility incomplete: {}", art.accessibility_incomplete_total),
                    format!(
                        "reports missing accessibility artifact: {}",
                        art.reports_missing_required_accessibility_artifact
                    ),
                ],
                Some("HLT-014-A11Y-GAP"),
                None,
            );
        }
    }
    if !scan::prompt_injection_hits(ctx).is_empty() {
        let hit = scan::prompt_injection_hits(ctx)[0].clone();
        b.add("high","security",&hit.path,"trusted agent/tool policy contains prompt-injection or policy-bypass language","isolate untrusted instructions from trusted policy, remove bypass wording, and validate tool calls against the repository standard", vec![hit.problem], Some("HLT-011-PROMPT-INJECTION"), hit.line);
    }
    if !scan::agency_hits(ctx).is_empty() {
        let hit = scan::agency_hits(ctx)[0].clone();
        b.add_with_rule("HLT-012-OVERBROAD-AGENCY", &hit.path, "agent/tool permissions appear broader than the requested proof lane", "replace broad terminal/browser/network/filesystem permissions with least-privilege lane profiles and explicit approval gates", vec![hit.problem], hit.line, None, None);
    }
    if !scan::secret_hits(ctx).is_empty() {
        let hit = scan::secret_hits(ctx)[0].clone();
        b.add_with_rule("HLT-010-SECRET-SPRAWL", &hit.path, "secret-like value or credential material appears in repository text", "remove and rotate the credential, add local and CI secret scanning, and scan transcripts/artifacts/MCP config for related exposure", vec![hit.problem], hit.line, None, None);
    }
    if !scan::false_green_hits(ctx).is_empty() {
        let hit = scan::false_green_hits(ctx)[0].clone();
        b.add_with_rule("HLT-008-FALSE-GREEN-RISK", &hit.path, "test code contains disabled, focused, tautological, or snapshot-only proof", "replace false-green tests with behavior assertions, red/green evidence, and mutation or fault checks for changed behavior", vec![hit.problem], hit.line, None, None);
    }
    if !scan::ci_hardening_hits(ctx).is_empty() {
        let hit = scan::ci_hardening_hits(ctx)[0].clone();
        b.add_with_rule(
            "HLT-020-CI-HARDENING-GAP",
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            vec!["CI hardening gap detected".into()],
            hit.line,
            None,
            None,
        );
    }
    if !scan::authz_isolation_hits(ctx).is_empty() {
        let hit = scan::authz_isolation_hits(ctx)[0].clone();
        b.add_with_rule(
            "HLT-022-AUTHZ-ISOLATION-GAP",
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            vec![hit.text],
            hit.line,
            hit.matched_term,
            Some("authz/data isolation requires negative proof evidence".into()),
        );
    }
    if !scan::input_boundary_hits(ctx).is_empty() {
        let hit = scan::input_boundary_hits(ctx)[0].clone();
        b.add_with_rule(
            "HLT-023-INPUT-BOUNDARY-GAP",
            &hit.path,
            "unsafe or unvalidated input boundary marker appears in product code",
            "replace unsafe sinks with typed schemas, parameterized APIs, allowlists, or sandboxed execution plus negative tests",
            vec![hit.problem],
            hit.line,
            hit.matched_term,
            Some("input handling risk needs deterministic negative tests".into()),
        );
    }
    for hit in scan::language_bad_behavior_hits(ctx) {
        b.add_with_rule(
            hit.rule_id,
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            hit.evidence,
            hit.line,
            Some(hit.matched_term.into()),
            Some(hit.reason),
        );
    }
    for hit in web_security::findings(ctx) {
        b.add_with_rule(
            hit.rule_id,
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            hit.evidence,
            hit.line,
            Some(hit.matched_term.into()),
            Some(hit.reason),
        );
    }
    for hit in repo_rot::findings(ctx) {
        b.add_with_rule(
            hit.rule_id,
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            hit.evidence,
            hit.line,
            Some(hit.matched_term.into()),
            Some(hit.reason),
        );
    }
    for hit in ci_local_parity::findings(ctx) {
        b.add_with_rule(
            hit.rule_id,
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            hit.evidence,
            hit.line,
            Some(hit.matched_term.into()),
            Some(hit.reason),
        );
    }
    for hit in zyal::findings(ctx) {
        b.add_with_rule(
            "HLT-024-AGENT-TOOL-SUPPLY-GAP",
            &hit.path,
            &hit.problem,
            &hit.fix,
            hit.evidence,
            hit.line,
            hit.matched_term,
            hit.reason,
        );
    }
    if !scan::agent_tool_supply_hits(ctx).is_empty() {
        let hit = scan::agent_tool_supply_hits(ctx)[0].clone();
        b.add_with_rule(
            "HLT-024-AGENT-TOOL-SUPPLY-GAP",
            &hit.path,
            "agent tool or configuration trust surface requires supply-chain evidence",
            "pin and review agent tools, MCP servers, hooks, and rule files; keep untrusted tool output separate from trusted policy",
            vec![hit.problem],
            hit.line,
            hit.matched_term,
            Some("agent tool supply-chain changes alter execution authority".into()),
        );
    }
    if !scan::release_readiness_hits(ctx).is_empty() {
        let hit = scan::release_readiness_hits(ctx)[0].clone();
        b.add_with_rule(
            "HLT-025-RELEASE-READINESS-GAP",
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            vec![hit.text],
            hit.line,
            hit.matched_term,
            Some("launch gates need artifact-backed release evidence".into()),
        );
    }
    if !scan::cost_budget_hits(ctx).is_empty() {
        let hit = scan::cost_budget_hits(ctx)[0].clone();
        b.add_with_rule(
            "HLT-026-COST-BUDGET-GAP",
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            vec![hit.text],
            hit.line,
            hit.matched_term,
            Some("unbounded paid work needs budgets and stop conditions".into()),
        );
    }
    if !scan::human_review_evidence_hits(ctx).is_empty() {
        let hit = scan::human_review_evidence_hits(ctx)[0].clone();
        b.add_with_rule(
            "HLT-027-HUMAN-REVIEW-EVIDENCE-GAP",
            &hit.path,
            "human review or proof claim lacks reproducible evidence",
            "attach raw CI logs, review receipts, and replayable commands instead of accepting claims or summaries",
            vec![hit.problem],
            hit.line,
            hit.matched_term,
            Some("proof and review claims need receipts".into()),
        );
    }
    if !destructive_sql_hits.is_empty() {
        let hit = destructive_sql_hits[0].clone();
        let fix = hit.agent_fix.as_str();
        b.add_with_rule(
            "HLT-021-DESTRUCTIVE-MIGRATION",
            &hit.path,
            "destructive migration lacks documented safety evidence",
            fix,
            vec![hit.problem.clone()],
            hit.line,
            None,
            None,
        );
    }
    if let Some(summary) = proofbind_artifact::load_summary(&ctx.root) {
        if summary.mode == "required" {
            for obligation in summary.missing_obligations.iter().take(10) {
                let rule_id = obligation
                    .rule_ids
                    .iter()
                    .find(|rule| rules::lookup(rule).is_some())
                    .map(String::as_str)
                    .unwrap_or("HLT-008-FALSE-GREEN-RISK");
                b.add(
                    "high",
                    "proof",
                    &summary.path,
                    &format!(
                        "proofbind obligation `{}` for `{}` is missing receipt evidence",
                        obligation.surface_type, obligation.path
                    ),
                    &obligation.repair_task,
                    vec![
                        format!("obligation_id={}", obligation.obligation_id),
                        format!("surface severity={}", obligation.severity),
                        format!("proofbind verdict={}", summary.verdict),
                    ],
                    Some(rule_id),
                    None,
                );
            }
        }
    }
    if caps_applied.contains(&"missing-rust-property-or-integration-tests".into()) {
        b.add_with_rule("HLT-008-FALSE-GREEN-RISK", "crates/", "Rust surface lacks required property and/or integration tests", "add `proptest` or equivalent invariant tests plus `tests/` integration coverage routed through `cargo nextest` or `cargo test`", vec!["Rust surface detected".into()], None, None, None);
    }
    if caps_applied.contains(&"no-agent-friendly-exception-pattern".into()) {
        let exception = helpers::audit_repair_exception();
        b.add(
            "high",
            "exceptions",
            "crates/domain",
            "no agent-friendly exception/error pattern was detected",
            exception.repair_hint,
            vec![
                exception.purpose.into(),
                exception.reason.into(),
                exception.common_fixes.join("; "),
                exception.docs_url.into(),
            ],
            Some("HLT-017-OPAQUE-OBSERVABILITY"),
            None,
        );
    }
    if caps_applied.contains(&"missing-agent-readable-docs".into()) {
        let missing = helpers::missing_core_docs(ctx);
        b.add("medium","docs","docs/","agent-readable documentation is incomplete","add concise docs for architecture, boundaries, tests, generated zones, and audit rules; route them from root `AGENTS.md`", missing, None, None);
    }
    if !scan::streaming_runtime_hits(ctx).is_empty() {
        let hit = scan::streaming_runtime_hits(ctx)[0].clone();
        b.add_with_rule(
            "HLT-019-STREAMING-RUNTIME-DRIFT",
            &hit.path,
            "queue or streaming runtime client appears outside the declared adapter boundary",
            "move Kafka/Tansu/Iggy/Fluvio/NATS/Redis-stream clients behind `crates/adapters/queues` or document a brownfield exception with owner, expiry, and migration path",
            vec![hit.problem.clone()],
            hit.line,
            None,
            None,
        );
    }
    // Phase 07 H1: contract source detection
    for hit in scan::contract_source_hits(ctx) {
        b.add(
            "high",
            "boundary",
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            vec![hit.text],
            Some("HLT-007-HANDWRITTEN-CONTRACT"),
            hit.line,
        );
    }
    // Phase 07 H2: generated zone existence + header
    for hit in scan::generated_zone_existence_hits(ctx) {
        b.add_with_rule(
            "HLT-002-GENERATED-MUTATION",
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            vec!["generated zone integrity violation".into()],
            hit.line,
            None,
            None,
        );
    }
    // Phase 07 H4: event contract path validation
    for hit in scan::event_contract_path_hits(ctx) {
        b.add(
            "high",
            "boundary",
            &hit.path,
            &hit.problem,
            &hit.agent_fix,
            vec![hit.text],
            Some("HLT-007-HANDWRITTEN-CONTRACT"),
            hit.line,
        );
    }

    for dimension in dimensions.iter().filter(|dimension| dimension.score < 85) {
        if dimension.name == "Jankurai tool adoption and CI replacement" {
            continue;
        }
        let (category, path, rule_id, fix) = dimension_soft_route(&dimension.name);
        let evidence = if dimension.evidence.is_empty() && dimension.notes.is_empty() {
            vec![format!("{} scored {}", dimension.name, dimension.score)]
        } else {
            dimension
                .evidence
                .iter()
                .chain(dimension.notes.iter())
                .take(4)
                .cloned()
                .collect()
        };
        b.add(
            "medium",
            category,
            path,
            &format!(
                "`{}` scored {} below the standard floor of 85",
                dimension.name, dimension.score
            ),
            fix,
            evidence,
            Some(rule_id),
            None,
        );
    }

    if final_score < minimum_score && !b.has_any_finding() {
        b.add(
            "medium",
            "audit",
            "agent/audit-policy.toml",
            "repository score is below the configured floor but no specific repair finding was emitted",
            "add or tune audit rules so every below-floor score maps to at least one actionable repair queue entry",
            vec![format!("score {final_score} is below minimum_score {minimum_score}")],
            Some("HLT-017-OPAQUE-OBSERVABILITY"),
            None,
        );
    }

    if let Some(ownership) = dim_by_name.get("Ownership and navigation surface") {
        if ownership.score < 55 && !b.has_context_finding() {
            b.add("medium","context",".","navigation surface is thin for agent work","add local routing docs and machine-readable owner/test maps where the repo needs them", ownership.evidence.iter().take(2).cloned().collect(), None, None);
        }
    }
    if let Some(shape) = dim_by_name.get("Code shape and semantic surface") {
        if shape.notes.iter().any(|n| n.contains("large code files")) {
            if let Some(max) = helpers::max_loc(&helpers::product_code_files(ctx)) {
                b.add(if max <= 1000 { "medium" } else { "high" }, "shape", ".", &format!("largest code file is {} LOC", max), "split the file along ownership or semantic boundaries before agents have to patch it again", vec![format!("largest authored code file: {} LOC", max)], None, None);
            }
        }
    }

    let mut findings = b.into_findings();
    findings.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.unwrap_or(0).cmp(&b.line.unwrap_or(0)))
            .then(a.problem.cmp(&b.problem))
    });
    findings
}

fn path_matches_scope(rel_path: &str, scopes: &[String]) -> bool {
    if scopes.is_empty() {
        return true;
    }
    scopes.iter().any(|scope| {
        rel_path == scope
            || rel_path.starts_with(&format!("{}/", scope))
            || scope.starts_with(&format!("{}/", rel_path))
    })
}

fn changed_fast_inventory_paths(scope_paths: &[String]) -> Vec<String> {
    let mut paths: BTreeSet<String> = scope_paths.iter().cloned().collect();
    for path in [
        "AGENTS.md",
        "CLAUDE.md",
        "GEMINI.md",
        "Justfile",
        "Cargo.toml",
        "Cargo.lock",
        "package.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "go.mod",
        "go.sum",
        "agent",
        ".github/workflows",
    ] {
        paths.insert(path.to_string());
    }
    paths.into_iter().collect()
}

fn normalize_changed_path(root: &Path, path: &Path) -> Option<String> {
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let rel = candidate
        .strip_prefix(root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    Some(rel)
}

pub fn changed_paths_from_git(root: &Path, base: &str) -> Result<Vec<PathBuf>> {
    let refspec = format!("{base}...HEAD");
    let output = Command::new("git")
        .args(["diff", "--name-only", refspec.as_str()])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Ok(vec![]);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| root.join(line.trim()))
        .collect())
}

fn load_proof_receipts(root: &Path, path: Option<&str>) -> Result<Vec<ProofReceipt>> {
    let Some(path) = path else {
        return Ok(vec![]);
    };
    let path = root.join(path);
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut entries = Vec::new();
    if path.is_dir() {
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            entries.push(entry_path);
        }
        entries.sort();
    } else {
        entries.push(path);
    }

    let mut receipts = Vec::new();
    for entry in entries {
        let text = std::fs::read_to_string(&entry)?;
        let value: serde_json::Value = serde_json::from_str(&text)?;
        crate::validation::validate_value(
            root,
            crate::validation::ArtifactSchema::ProofReceipt,
            &value,
        )?;
        let receipt: ProofReceipt = serde_json::from_value(value)?;
        receipts.push(receipt);
    }
    Ok(receipts)
}

pub fn release_proof_findings(
    root: &Path,
    proof_receipts: Option<&str>,
    proof_evidence: Option<&str>,
) -> Result<Vec<Finding>> {
    if let Some(path) = proof_evidence {
        return release_proof_evidence_findings(root, path);
    }
    if let Some(path) = proof_receipts {
        if load_proof_receipts(root, Some(path))?.is_empty() {
            return Ok(vec![release_proof_finding(
                "release mode requires proof evidence or proof receipts for the audited scope",
                vec!["no proof receipts were supplied".into()],
                "HLT-004-UNMAPPED-PROOF",
                "proof-receipts",
                "run `jankurai prove` and feed its evidence into `jankurai audit --proof-evidence`",
                "receipt",
                "proof receipts",
                "release mode cannot be verified without proof evidence",
            )]);
        }
        return Ok(vec![]);
    }
    Ok(vec![release_proof_finding(
        "release mode requires proof evidence for the audited scope",
        vec!["no proof evidence was supplied".into()],
        "HLT-004-UNMAPPED-PROOF",
        "proof-evidence",
        "run `jankurai prove` and feed its evidence index into `jankurai audit --proof-evidence`",
        "receipt",
        "proof evidence",
        "release mode cannot be verified without proof evidence",
    )])
}

fn release_proof_evidence_findings(root: &Path, path: &str) -> Result<Vec<Finding>> {
    let evidence_path = root.join(path);
    if !evidence_path.is_file() {
        return Ok(vec![release_proof_finding(
            "release mode requires proof evidence for the audited scope",
            vec![format!("missing proof evidence `{}`", display_rel(root, &evidence_path))],
            "HLT-004-UNMAPPED-PROOF",
            &display_rel(root, &evidence_path),
            "run `jankurai prove` and feed its evidence index into `jankurai audit --proof-evidence`",
            "receipt",
            "proof evidence",
            "release mode cannot be verified without proof evidence",
        )]);
    }

    let text = std::fs::read_to_string(&evidence_path)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    crate::validation::validate_value(
        root,
        crate::validation::ArtifactSchema::EvidenceIndex,
        &value,
    )?;

    let mut issues = Vec::new();
    let current_manifest = manifest_fingerprints(root);
    let evidence_manifest = value
        .get("manifest_fingerprints")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    compare_manifest_fingerprint(
        &mut issues,
        "owner_map",
        current_manifest.owner_map.as_deref(),
        evidence_manifest
            .get("owner_map")
            .and_then(serde_json::Value::as_str),
    );
    compare_manifest_fingerprint(
        &mut issues,
        "test_map",
        current_manifest.test_map.as_deref(),
        evidence_manifest
            .get("test_map")
            .and_then(serde_json::Value::as_str),
    );
    compare_manifest_fingerprint(
        &mut issues,
        "generated_zones",
        current_manifest.generated_zones.as_deref(),
        evidence_manifest
            .get("generated_zones")
            .and_then(serde_json::Value::as_str),
    );
    compare_manifest_fingerprint(
        &mut issues,
        "boundaries",
        current_manifest.boundaries.as_deref(),
        evidence_manifest
            .get("boundaries")
            .and_then(serde_json::Value::as_str),
    );
    compare_manifest_fingerprint(
        &mut issues,
        "proof_lanes",
        current_manifest.proof_lanes.as_deref(),
        evidence_manifest
            .get("proof_lanes")
            .and_then(serde_json::Value::as_str),
    );
    compare_manifest_fingerprint(
        &mut issues,
        "standard_version",
        current_manifest.standard_version.as_deref(),
        evidence_manifest
            .get("standard_version")
            .and_then(serde_json::Value::as_str),
    );

    let plan_path = value
        .get("plan_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if plan_path.is_empty() {
        issues.push("proof evidence missing plan_path".into());
    } else {
        let plan_abs = root.join(plan_path);
        if !plan_abs.is_file() {
            issues.push(format!("proof plan `{}` is missing", plan_path));
        } else {
            let expected_digest = file_fingerprint(&plan_abs).unwrap_or_else(missing_sha256);
            let actual_digest = value
                .get("plan_digest")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("missing");
            if actual_digest != expected_digest {
                issues.push(format!(
                    "plan digest mismatch for `{plan_path}`: evidence `{actual_digest}` vs current `{expected_digest}`"
                ));
            }
        }
    }

    let receipts = value
        .get("receipts")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let receipt_digest_entries = value
        .get("receipt_digests")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let command_digest_entries = value
        .get("command_digests")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let log_digest_entries = value
        .get("log_digests")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let artifact_digest_entries = value
        .get("artifact_digests")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let coverage_verdicts = value
        .get("coverage_verdicts")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Array(vec![]));

    if receipts.is_empty() {
        issues.push("proof evidence has no receipts".into());
    }
    if coverage_verdicts
        .as_array()
        .map(|items| items.is_empty())
        .unwrap_or(true)
    {
        issues.push("proof evidence has no coverage verdicts".into());
    }

    for receipt_rel_value in receipts {
        let Some(receipt_rel) = receipt_rel_value.as_str() else {
            continue;
        };
        let receipt_path = root.join(receipt_rel);
        if !receipt_path.is_file() {
            issues.push(format!("missing proof receipt `{receipt_rel}`"));
            continue;
        }
        let text = std::fs::read_to_string(&receipt_path)?;
        let receipt_json: serde_json::Value = serde_json::from_str(&text)?;
        crate::validation::validate_value(
            root,
            crate::validation::ArtifactSchema::ProofReceipt,
            &receipt_json,
        )?;
        let receipt: ProofReceipt = serde_json::from_value(receipt_json.clone())?;
        let receipt_digest = file_fingerprint(&receipt_path).unwrap_or_else(missing_sha256);
        if !receipt_digest_entries.iter().any(|entry| {
            entry.get("path").and_then(serde_json::Value::as_str) == Some(receipt_rel)
                && entry.get("sha256").and_then(serde_json::Value::as_str)
                    == Some(receipt_digest.as_str())
        }) {
            issues.push(format!("receipt digest mismatch for `{receipt_rel}`"));
        }
        if let Some(plan_digest) = receipt.plan_digest.as_deref() {
            let expected = value
                .get("plan_digest")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("missing");
            if plan_digest != expected {
                issues.push(format!("receipt `{receipt_rel}` plan digest mismatch"));
            }
        }
        if let Some(command_digest) = receipt.command_digest.as_deref() {
            let expected = sha256_string(&receipt.command);
            if command_digest != expected {
                issues.push(format!("receipt `{receipt_rel}` command digest mismatch"));
            }
            if !command_digest_entries.iter().any(|entry| {
                entry.get("path").and_then(serde_json::Value::as_str)
                    == Some(format!("{}::{}", receipt.lane, receipt.command).as_str())
                    && entry.get("sha256").and_then(serde_json::Value::as_str)
                        == Some(expected.as_str())
            }) {
                issues.push(format!(
                    "command digest index mismatch for receipt `{receipt_rel}`"
                ));
            }
        }
        if let Some(log_rel) = receipt.log_path.as_deref() {
            let log_path = root.join(log_rel);
            if !log_path.is_file() {
                issues.push(format!("missing proof log `{log_rel}`"));
            } else {
                let expected = file_fingerprint(&log_path).unwrap_or_else(missing_sha256);
                if receipt.log_sha256.as_deref() != Some(expected.as_str()) {
                    issues.push(format!("receipt `{receipt_rel}` log digest mismatch"));
                }
                if !log_digest_entries.iter().any(|entry| {
                    entry.get("path").and_then(serde_json::Value::as_str) == Some(log_rel)
                        && entry.get("sha256").and_then(serde_json::Value::as_str)
                            == Some(expected.as_str())
                }) {
                    issues.push(format!("log digest index mismatch for `{log_rel}`"));
                }
            }
        }
        if let Some(recorded) = receipt.artifact_digests.first() {
            if !artifact_digest_entries.iter().any(|entry| {
                entry.get("path").and_then(serde_json::Value::as_str)
                    == Some(recorded.path.as_str())
                    && entry.get("sha256").and_then(serde_json::Value::as_str)
                        == Some(recorded.sha256.as_str())
            }) {
                issues.push(format!(
                    "artifact digest index mismatch for receipt `{receipt_rel}`"
                ));
            }
        }
        if receipt.exit_code != 0 {
            issues.push(format!(
                "receipt `{receipt_rel}` exited with {}",
                receipt.exit_code
            ));
        }
    }

    if issues.is_empty() {
        return Ok(vec![]);
    }

    Ok(vec![release_proof_finding(
        "release proof evidence is stale or incomplete",
        issues,
        "HLT-008-FALSE-GREEN-RISK",
        &display_rel(root, &evidence_path),
        "regenerate proof evidence and re-run `jankurai audit --proof-evidence`",
        "receipt",
        "proof evidence",
        "release mode proof integrity checks failed",
    )])
}

fn compare_manifest_fingerprint(
    issues: &mut Vec<String>,
    name: &str,
    current: Option<&str>,
    evidence: Option<&str>,
) {
    if current != evidence {
        issues.push(format!(
            "manifest fingerprint mismatch for {name}: evidence `{}` vs current `{}`",
            evidence.unwrap_or("missing"),
            current.unwrap_or("missing")
        ));
    }
}

// Release proof findings preserve every receipt field for audit and SARIF output.
#[allow(clippy::too_many_arguments)]
fn release_proof_finding(
    problem: &str,
    evidence: Vec<String>,
    rule_id: &str,
    path: &str,
    agent_fix: &str,
    evidence_kind: &str,
    matched_term: &str,
    reason: &str,
) -> Finding {
    Finding {
        severity: "high".into(),
        category: "proof".into(),
        path: path.into(),
        problem: problem.into(),
        agent_fix: agent_fix.into(),
        evidence,
        check_id: "proof-evidence".into(),
        hardness: "hard".into(),
        confidence: 1.0,
        evidence_kind: evidence_kind.into(),
        rerun_command: "jankurai prove".into(),
        fingerprint: "sha256:pending".into(),
        rule_id: Some(rule_id.into()),
        tlr: Some("proof".into()),
        lane: Some("release".into()),
        docs_url: Some("agent/JANKURAI_STANDARD.md#proof-lanes".into()),
        owner: Some("agent".into()),
        line: None,
        matched_term: Some(matched_term.into()),
        reason: Some(reason.into()),
    }
}

pub fn docs_for_rule_id(rule: &str) -> Option<&'static str> {
    rules::docs_for_rule_id(rule)
}

pub fn rule_registry() -> &'static [rules::RuleSpec] {
    rules::all()
}
