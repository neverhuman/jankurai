use super::finding_builder::{
    confidence_for_severity, finding_fingerprint, hardness_for_severity, rerun_command_for_lane,
};
use super::rules;
use crate::model::{CoverageEvidenceSummary, Finding};
use crate::validation::{self, ArtifactSchema};
use anyhow::{bail, Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub const DEFAULT_CONFIG_PATH: &str = "agent/coverage-sources.toml";
pub const DEFAULT_JSON_PATH: &str = "target/jankurai/coverage/coverage-audit.json";
pub const DEFAULT_MD_PATH: &str = "target/jankurai/coverage/coverage-audit.md";
pub const DEFAULT_MAX_ARTIFACT_BYTES: u64 = 10_000_000;
pub const DEFAULT_MAX_FINDINGS: usize = 200;
const PER_SOURCE_FINDINGS_CAP: usize = 50;

pub type CoverageMetric = Value;

#[derive(Debug, Clone)]
pub struct CoverageAuditOptions {
    pub repo_root: PathBuf,
    pub config_path: PathBuf,
    pub changed_from: Option<String>,
    pub strict: bool,
    pub max_artifact_bytes: u64,
    pub max_findings: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoverageConfig {
    pub version: u32,
    #[serde(default, rename = "source")]
    pub sources: Vec<CoverageSource>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoverageSource {
    pub id: String,
    pub kind: CoverageKind,
    pub format: CoverageFormat,
    pub mode: CoverageMode,
    #[serde(default = "default_owner")]
    pub owner: String,
    #[serde(default = "default_lane")]
    pub lane: String,
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub rules: Vec<String>,
    pub hard_changed_line_coverage: Option<f64>,
    pub soft_total_line_coverage: Option<f64>,
    pub hard_survivors_on_changed_paths: Option<u64>,
    pub hard_critical_vulnerabilities: Option<u64>,
    pub hard_high_vulnerabilities: Option<u64>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageMode {
    Required,
    Advisory,
    Disabled,
    Auto,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageKind {
    LineCoverage,
    Mutation,
    PropertyFuzz,
    ApiContract,
    UiE2e,
    DbMigration,
    Container,
    SupplyChain,
    DeadCode,
    TypeCoverage,
    JankuraiArtifact,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum CoverageFormat {
    #[serde(rename = "lcov")]
    Lcov,
    #[serde(rename = "cargo-mutants-json")]
    CargoMutantsJson,
    #[serde(rename = "stryker-json")]
    StrykerJson,
    #[serde(rename = "trivy-json")]
    TrivyJson,
    #[serde(rename = "hadolint-json")]
    HadolintJson,
    #[serde(rename = "jankurai-json")]
    JankuraiJson,
    #[serde(rename = "generic-json-summary")]
    GenericJsonSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageAudit {
    pub schema_version: u32,
    pub generated_by: String,
    pub repo_root: String,
    pub config_path: String,
    pub strict: bool,
    pub changed_from: Option<String>,
    pub summary: CoverageSummary,
    pub sources: Vec<CoverageSourceResult>,
    pub findings: Vec<CoverageFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoverageSummary {
    pub status: String,
    pub sources_total: usize,
    pub sources_present: usize,
    pub sources_missing: usize,
    pub hard_findings: usize,
    pub soft_findings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSourceResult {
    pub id: String,
    pub kind: String,
    pub format: String,
    pub mode: String,
    pub status: String,
    pub artifact_paths: Vec<String>,
    pub matched_artifact: Option<String>,
    pub applies_to: Vec<String>,
    pub owner: String,
    pub lane: String,
    pub metrics: BTreeMap<String, CoverageMetric>,
    pub parser_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageFinding {
    pub rule_id: String,
    pub severity: String,
    pub confidence: f64,
    pub source_id: String,
    pub kind: String,
    pub artifact: String,
    pub path: String,
    pub line: Option<usize>,
    pub message: String,
    pub evidence: Vec<String>,
    pub repair: String,
    pub owner: String,
    pub lane: String,
}

#[derive(Debug, Clone, Default)]
pub struct LcovReport {
    pub files: BTreeMap<String, LcovFile>,
    pub total_lines: usize,
    pub covered_lines: usize,
    pub total_branches: usize,
    pub covered_branches: usize,
    pub parser_warnings: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LcovFile {
    pub lines: BTreeMap<usize, u64>,
}

#[derive(Debug, Clone, Default)]
pub struct MutationReport {
    pub total: usize,
    pub killed: usize,
    pub survived: usize,
    pub timeout: usize,
    pub unviable: usize,
    pub skipped: usize,
    pub mutants: Vec<MutationOutcome>,
}

#[derive(Debug, Clone)]
pub struct MutationOutcome {
    pub path: String,
    pub line: Option<usize>,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct SecurityReport {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub vulnerabilities: Vec<SecurityIssue>,
}

#[derive(Debug, Clone)]
pub struct SecurityIssue {
    pub target: String,
    pub vulnerability_id: String,
    pub package_name: String,
    pub severity: String,
    pub title: String,
}

#[derive(Debug, Clone, Default)]
pub struct ContainerLintReport {
    pub diagnostics: Vec<ContainerLintIssue>,
}

#[derive(Debug, Clone)]
pub struct ContainerLintIssue {
    pub file: String,
    pub line: Option<usize>,
    pub code: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct CoverageScoreIngest {
    pub summary: Option<CoverageEvidenceSummary>,
    pub findings: Vec<CoverageFinding>,
    pub config_present: bool,
    pub artifact_malformed: Option<String>,
}

fn default_owner() -> String {
    "agent".into()
}

fn default_lane() -> String {
    "coverage-audit".into()
}

impl CoverageMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Advisory => "advisory",
            Self::Disabled => "disabled",
            Self::Auto => "auto",
        }
    }
}

impl CoverageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LineCoverage => "line_coverage",
            Self::Mutation => "mutation",
            Self::PropertyFuzz => "property_fuzz",
            Self::ApiContract => "api_contract",
            Self::UiE2e => "ui_e2e",
            Self::DbMigration => "db_migration",
            Self::Container => "container",
            Self::SupplyChain => "supply_chain",
            Self::DeadCode => "dead_code",
            Self::TypeCoverage => "type_coverage",
            Self::JankuraiArtifact => "jankurai_artifact",
        }
    }
}

impl CoverageFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lcov => "lcov",
            Self::CargoMutantsJson => "cargo-mutants-json",
            Self::StrykerJson => "stryker-json",
            Self::TrivyJson => "trivy-json",
            Self::HadolintJson => "hadolint-json",
            Self::JankuraiJson => "jankurai-json",
            Self::GenericJsonSummary => "generic-json-summary",
        }
    }
}

pub fn load_coverage_config(repo_root: &Path, config_path: &Path) -> Result<CoverageConfig> {
    let path = resolve_existing_or_relative(repo_root, config_path);
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    validation::validate_coverage_sources_toml_text(repo_root, &text)?;
    let config: CoverageConfig =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if config.version != 1 {
        bail!("unsupported coverage sources version {}", config.version);
    }
    let mut ids = BTreeSet::new();
    for source in &config.sources {
        if source.id.trim().is_empty() {
            bail!("coverage source id may not be empty");
        }
        if !ids.insert(source.id.clone()) {
            bail!("duplicate coverage source id `{}`", source.id);
        }
    }
    Ok(config)
}

pub fn run_coverage_audit(opts: CoverageAuditOptions) -> Result<CoverageAudit> {
    let repo_root = opts
        .repo_root
        .canonicalize()
        .with_context(|| format!("resolve repo root {}", opts.repo_root.display()))?;
    let config_path = resolve_existing_or_relative(&repo_root, &opts.config_path);
    let config = load_coverage_config(&repo_root, &config_path)?;
    let changed_lines = if let Some(base) = opts.changed_from.as_deref() {
        changed_lines_from_git(&repo_root, base)?
    } else {
        BTreeMap::new()
    };
    let changed_paths = changed_lines.keys().cloned().collect::<BTreeSet<_>>();

    let mut source_results = Vec::new();
    let mut findings = Vec::new();

    for source in &config.sources {
        let mut result = base_source_result(&repo_root, source)?;
        if source.mode == CoverageMode::Disabled
            || (source.mode == CoverageMode::Auto && !auto_source_enabled(&repo_root, source)?)
        {
            result.status = "disabled".into();
            source_results.push(result);
            continue;
        }

        let matched_artifact = first_existing_artifact(&repo_root, source)?;
        let Some((artifact_abs, artifact_rel)) = matched_artifact else {
            result.status = "missing".into();
            result.metrics.insert("present".into(), json!(false));
            let severity = missing_artifact_severity(source, &changed_paths, opts.strict)?;
            if let Some(severity) = severity {
                findings.push(missing_artifact_finding(
                    source,
                    &artifact_label(source),
                    severity,
                ));
            } else if source.mode != CoverageMode::Auto {
                findings.push(missing_artifact_finding(
                    source,
                    &artifact_label(source),
                    "info",
                ));
            }
            source_results.push(result);
            continue;
        };

        result.matched_artifact = Some(artifact_rel.clone());
        result.metrics.insert("present".into(), json!(true));
        let mut source_findings = match source.format {
            CoverageFormat::Lcov => match parse_lcov(&artifact_abs, opts.max_artifact_bytes) {
                Ok(report) => {
                    result.parser_warnings = report.parser_warnings.clone();
                    analyze_lcov(
                        source,
                        &artifact_rel,
                        &report,
                        &changed_lines,
                        opts.strict,
                        &mut result.metrics,
                    )?
                }
                Err(err) => {
                    parser_error_findings(source, &artifact_rel, err.to_string(), opts.strict)
                }
            },
            CoverageFormat::CargoMutantsJson => {
                match parse_cargo_mutants_json(&artifact_abs, opts.max_artifact_bytes) {
                    Ok(report) => analyze_mutation(
                        source,
                        &artifact_rel,
                        &report,
                        &changed_paths,
                        opts.strict,
                    ),
                    Err(err) => {
                        parser_error_findings(source, &artifact_rel, err.to_string(), opts.strict)
                    }
                }
            }
            CoverageFormat::StrykerJson => {
                match parse_stryker_json(&artifact_abs, opts.max_artifact_bytes) {
                    Ok(report) => analyze_mutation(
                        source,
                        &artifact_rel,
                        &report,
                        &changed_paths,
                        opts.strict,
                    ),
                    Err(err) => {
                        parser_error_findings(source, &artifact_rel, err.to_string(), opts.strict)
                    }
                }
            }
            CoverageFormat::TrivyJson => {
                match parse_trivy_json(&artifact_abs, opts.max_artifact_bytes) {
                    Ok(report) => analyze_security(
                        source,
                        &artifact_rel,
                        &report,
                        opts.strict,
                        &mut result.metrics,
                    ),
                    Err(err) => {
                        parser_error_findings(source, &artifact_rel, err.to_string(), opts.strict)
                    }
                }
            }
            CoverageFormat::HadolintJson => {
                match parse_hadolint_json(&artifact_abs, opts.max_artifact_bytes) {
                    Ok(report) => analyze_hadolint(
                        source,
                        &artifact_rel,
                        &report,
                        opts.strict,
                        &mut result.metrics,
                    ),
                    Err(err) => {
                        parser_error_findings(source, &artifact_rel, err.to_string(), opts.strict)
                    }
                }
            }
            CoverageFormat::GenericJsonSummary | CoverageFormat::JankuraiJson => {
                match parse_generic_json_summary(&artifact_abs, opts.max_artifact_bytes) {
                    Ok((metrics, imported)) => {
                        result.metrics.extend(metrics);
                        normalize_imported_findings(source, &artifact_rel, imported, opts.strict)
                    }
                    Err(err) => {
                        parser_error_findings(source, &artifact_rel, err.to_string(), opts.strict)
                    }
                }
            }
        };

        cap_source_findings(source, &mut source_findings);
        result.status = status_for_findings(&source_findings);
        findings.extend(source_findings);
        source_results.push(result);
    }

    dedup_findings(&mut findings);
    sort_findings(&mut findings);
    cap_global_findings(&mut findings, opts.max_findings);

    let sources_present = source_results
        .iter()
        .filter(|source| source.matched_artifact.is_some())
        .count();
    let sources_missing = source_results
        .iter()
        .filter(|source| source.status == "missing")
        .count();
    let hard_findings = findings
        .iter()
        .filter(|finding| is_hard(&finding.severity))
        .count();
    let soft_findings = findings.len().saturating_sub(hard_findings);
    let summary_status = if hard_findings > 0 {
        "fail"
    } else if sources_present == 0 && sources_missing > 0 {
        "missing"
    } else if soft_findings > 0 || sources_missing > 0 {
        "warn"
    } else {
        "pass"
    };

    Ok(CoverageAudit {
        schema_version: 1,
        generated_by: "jankurai coverage audit".into(),
        repo_root: ".".into(),
        config_path: display_rel(&repo_root, &config_path),
        strict: opts.strict,
        changed_from: opts.changed_from,
        summary: CoverageSummary {
            status: summary_status.into(),
            sources_total: source_results
                .iter()
                .filter(|source| source.status != "disabled")
                .count(),
            sources_present,
            sources_missing,
            hard_findings,
            soft_findings,
        },
        sources: source_results,
        findings,
    })
}

pub fn parse_lcov(path: &Path, max_bytes: u64) -> Result<LcovReport> {
    let text = read_bounded_text(path, max_bytes)?;
    let repo_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut report = LcovReport::default();
    let mut current_path: Option<String> = None;
    let mut current_file = LcovFile::default();

    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("TN:") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("SF:") {
            if let Some(path) = current_path.take() {
                flush_lcov_file(&mut report, path, std::mem::take(&mut current_file));
            }
            if rest.trim().is_empty() {
                bail!("malformed LCOV at line {line_no}: empty SF record");
            }
            current_path = Some(normalize_source_path(&repo_root, rest.trim()));
            continue;
        }
        if let Some(rest) = line.strip_prefix("DA:") {
            let Some(_) = current_path.as_ref() else {
                bail!("malformed LCOV at line {line_no}: DA record before SF");
            };
            let mut parts = rest.split(',');
            let Some(line_part) = parts.next() else {
                bail!("malformed LCOV at line {line_no}: missing DA line");
            };
            let Some(count_part) = parts.next() else {
                bail!("malformed LCOV at line {line_no}: missing DA count");
            };
            let source_line: usize = line_part
                .parse()
                .with_context(|| format!("malformed LCOV at line {line_no}: invalid DA line"))?;
            let count: u64 = count_part
                .parse()
                .with_context(|| format!("malformed LCOV at line {line_no}: invalid DA count"))?;
            current_file.lines.insert(source_line, count);
            continue;
        }
        if let Some(rest) = line.strip_prefix("BRDA:") {
            let fields = rest.split(',').collect::<Vec<_>>();
            if fields.len() != 4 {
                bail!("malformed LCOV at line {line_no}: invalid BRDA record");
            }
            report.total_branches += 1;
            if fields[3] != "-" && fields[3].parse::<u64>().unwrap_or(0) > 0 {
                report.covered_branches += 1;
            }
            continue;
        }
        if line == "end_of_record" {
            let Some(path) = current_path.take() else {
                bail!("malformed LCOV at line {line_no}: end_of_record before SF");
            };
            flush_lcov_file(&mut report, path, std::mem::take(&mut current_file));
            continue;
        }
    }

    if let Some(path) = current_path.take() {
        report
            .parser_warnings
            .push("LCOV file ended without end_of_record; final record was accepted".into());
        flush_lcov_file(&mut report, path, current_file);
    }

    if report.files.is_empty() {
        bail!("LCOV report contains no source records");
    }
    Ok(report)
}

pub fn parse_cargo_mutants_json(path: &Path, max_bytes: u64) -> Result<MutationReport> {
    let text = read_bounded_text(path, max_bytes)?;
    let value: Value = serde_json::from_str(&text).context("parse cargo-mutants JSON")?;
    let mut outcomes = Vec::new();
    collect_mutation_outcomes(&value, None, &mut outcomes);
    Ok(build_mutation_report(outcomes))
}

pub fn parse_stryker_json(path: &Path, max_bytes: u64) -> Result<MutationReport> {
    let text = read_bounded_text(path, max_bytes)?;
    let value: Value = serde_json::from_str(&text).context("parse Stryker JSON")?;
    let mut outcomes = Vec::new();
    if let Some(files) = value.get("files").and_then(Value::as_object) {
        for (file_path, file_value) in files {
            if let Some(mutants) = file_value.get("mutants").and_then(Value::as_array) {
                for mutant in mutants {
                    if let Some(outcome) = mutation_outcome_from_value(mutant, Some(file_path)) {
                        outcomes.push(outcome);
                    }
                }
            }
        }
    }
    if outcomes.is_empty() {
        collect_mutation_outcomes(&value, None, &mut outcomes);
    }
    Ok(build_mutation_report(outcomes))
}

pub fn parse_trivy_json(path: &Path, max_bytes: u64) -> Result<SecurityReport> {
    let text = read_bounded_text(path, max_bytes)?;
    let value: Value = serde_json::from_str(&text).context("parse Trivy JSON")?;
    let mut report = SecurityReport::default();
    for result in value
        .get("Results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let target = result
            .get("Target")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        for vuln in result
            .get("Vulnerabilities")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let severity = vuln
                .get("Severity")
                .and_then(Value::as_str)
                .unwrap_or("UNKNOWN")
                .to_ascii_uppercase();
            match severity.as_str() {
                "CRITICAL" => report.critical += 1,
                "HIGH" => report.high += 1,
                "MEDIUM" => report.medium += 1,
                "LOW" => report.low += 1,
                _ => {}
            }
            report.vulnerabilities.push(SecurityIssue {
                target: target.clone(),
                vulnerability_id: vuln
                    .get("VulnerabilityID")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                package_name: vuln
                    .get("PkgName")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                severity,
                title: vuln
                    .get("Title")
                    .or_else(|| vuln.get("Description"))
                    .and_then(Value::as_str)
                    .unwrap_or("vulnerability reported by Trivy")
                    .to_string(),
            });
        }
    }
    Ok(report)
}

pub fn parse_hadolint_json(path: &Path, max_bytes: u64) -> Result<ContainerLintReport> {
    let text = read_bounded_text(path, max_bytes)?;
    let value: Value = serde_json::from_str(&text).context("parse Hadolint JSON")?;
    let items = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Hadolint JSON must be an array"))?;
    let mut report = ContainerLintReport::default();
    for item in items {
        report.diagnostics.push(ContainerLintIssue {
            file: item
                .get("file")
                .and_then(Value::as_str)
                .unwrap_or("Dockerfile")
                .to_string(),
            line: item
                .get("line")
                .and_then(Value::as_u64)
                .map(|line| line as usize),
            code: item
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("hadolint")
                .to_string(),
            level: item
                .get("level")
                .and_then(Value::as_str)
                .unwrap_or("info")
                .to_ascii_lowercase(),
            message: item
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Hadolint diagnostic")
                .to_string(),
        });
    }
    Ok(report)
}

pub fn write_coverage_json(path: &Path, audit: &CoverageAudit) -> Result<()> {
    validation::write_json(
        Path::new("."),
        ArtifactSchema::CoverageAudit,
        &path.display().to_string(),
        audit,
    )
}

pub fn write_coverage_markdown(path: &Path, audit: &CoverageAudit) -> Result<()> {
    crate::render::write_markdown(
        &path.display().to_string(),
        &render_coverage_markdown(audit),
    )
}

pub fn load_score_ingest(root: &Path) -> CoverageScoreIngest {
    let config_present = root.join(DEFAULT_CONFIG_PATH).is_file();
    let artifact_path = root.join(DEFAULT_JSON_PATH);
    if !artifact_path.is_file() {
        if config_present {
            return CoverageScoreIngest {
                summary: Some(CoverageEvidenceSummary {
                    artifact: DEFAULT_JSON_PATH.into(),
                    status: "missing".into(),
                    sources_total: 0,
                    sources_present: 0,
                    hard_findings: 0,
                    soft_findings: 1,
                }),
                findings: vec![CoverageFinding {
                    rule_id: "HLT-008-FALSE-GREEN-RISK".into(),
                    severity: "medium".into(),
                    confidence: 0.76,
                    source_id: "coverage-evidence".into(),
                    kind: "jankurai_artifact".into(),
                    artifact: DEFAULT_JSON_PATH.into(),
                    path: DEFAULT_CONFIG_PATH.into(),
                    line: None,
                    message: "coverage evidence lane is configured but has not been run".into(),
                    evidence: vec![format!("{DEFAULT_CONFIG_PATH} exists")],
                    repair: "run `cargo run -p jankurai -- coverage audit . --config agent/coverage-sources.toml --json target/jankurai/coverage/coverage-audit.json --md target/jankurai/coverage/coverage-audit.md`".into(),
                    owner: "agent".into(),
                    lane: "coverage-audit".into(),
                }],
                config_present,
                artifact_malformed: None,
            };
        }
        return CoverageScoreIngest {
            config_present,
            ..CoverageScoreIngest::default()
        };
    }

    let text = match fs::read_to_string(&artifact_path) {
        Ok(text) => text,
        Err(err) => {
            return malformed_score_ingest(config_present, format!("read coverage audit: {err}"));
        }
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(err) => {
            return malformed_score_ingest(config_present, format!("parse coverage audit: {err}"));
        }
    };
    if let Err(err) = validation::validate_value(root, ArtifactSchema::CoverageAudit, &value) {
        return malformed_score_ingest(config_present, format!("validate coverage audit: {err}"));
    }
    let audit: CoverageAudit = match serde_json::from_value(value) {
        Ok(audit) => audit,
        Err(err) => {
            return malformed_score_ingest(config_present, format!("decode coverage audit: {err}"));
        }
    };
    CoverageScoreIngest {
        summary: Some(CoverageEvidenceSummary {
            artifact: DEFAULT_JSON_PATH.into(),
            status: audit.summary.status.clone(),
            sources_total: audit.summary.sources_total,
            sources_present: audit.summary.sources_present,
            hard_findings: audit.summary.hard_findings,
            soft_findings: audit.summary.soft_findings,
        }),
        findings: audit.findings,
        config_present,
        artifact_malformed: None,
    }
}

pub fn apply_coverage_caps(caps: &mut Vec<String>, ingest: &CoverageScoreIngest) {
    for finding in ingest
        .findings
        .iter()
        .filter(|finding| is_hard(&finding.severity))
    {
        let cap = match normalize_rule_id(&finding.rule_id).as_str() {
            "HLT-008-FALSE-GREEN-RISK" => Some("false-green-test-risk"),
            "HLT-016-SUPPLY-CHAIN-DRIFT" => Some("no-secret-or-dependency-scanning-in-ci"),
            "HLT-021-DESTRUCTIVE-MIGRATION" => Some("destructive-migration-risk"),
            "HLT-022-AUTHZ-ISOLATION-GAP" => Some("authz-or-data-isolation-gap"),
            "HLT-023-INPUT-BOUNDARY-GAP" => Some("input-boundary-gap"),
            "HLT-032-DOCKER-BAD-BEHAVIOR" => Some("docker-bad-behavior"),
            "HLT-013-RENDERED-UX-GAP" => Some("missing-rendered-ux-qa-lane"),
            _ => None,
        };
        if let Some(cap) = cap {
            if !caps.iter().any(|existing| existing == cap) {
                caps.push(cap.into());
            }
        }
    }
}

pub fn score_findings(ingest: &CoverageScoreIngest) -> Vec<Finding> {
    ingest
        .findings
        .iter()
        .map(coverage_finding_to_score_finding)
        .collect()
}

fn base_source_result(repo_root: &Path, source: &CoverageSource) -> Result<CoverageSourceResult> {
    let mut artifact_paths = Vec::new();
    for artifact in &source.artifacts {
        let abs = resolve_artifact_path(repo_root, artifact)?;
        artifact_paths.push(display_rel(repo_root, &abs));
    }
    Ok(CoverageSourceResult {
        id: source.id.clone(),
        kind: source.kind.as_str().into(),
        format: source.format.as_str().into(),
        mode: source.mode.as_str().into(),
        status: "pending".into(),
        artifact_paths,
        matched_artifact: None,
        applies_to: source.applies_to.clone(),
        owner: source.owner.clone(),
        lane: source.lane.clone(),
        metrics: BTreeMap::new(),
        parser_warnings: Vec::new(),
    })
}

fn first_existing_artifact(
    repo_root: &Path,
    source: &CoverageSource,
) -> Result<Option<(PathBuf, String)>> {
    for artifact in &source.artifacts {
        let abs = resolve_artifact_path(repo_root, artifact)?;
        if abs.is_file() {
            let rel = display_rel(repo_root, &abs);
            return Ok(Some((abs, rel)));
        }
    }
    Ok(None)
}

fn artifact_label(source: &CoverageSource) -> String {
    source
        .artifacts
        .first()
        .cloned()
        .unwrap_or_else(|| "coverage artifact".into())
}

fn missing_artifact_severity(
    source: &CoverageSource,
    changed_paths: &BTreeSet<String>,
    strict: bool,
) -> Result<Option<&'static str>> {
    if source.mode != CoverageMode::Required {
        return Ok(None);
    }
    if strict
        || changed_paths
            .iter()
            .any(|path| source_matches_path(source, path).unwrap_or(false))
    {
        Ok(Some("high"))
    } else {
        Ok(Some("medium"))
    }
}

fn missing_artifact_finding(
    source: &CoverageSource,
    artifact: &str,
    severity: &str,
) -> CoverageFinding {
    CoverageFinding {
        rule_id: primary_rule(source, "HLT-008-FALSE-GREEN-RISK"),
        severity: severity.into(),
        confidence: confidence_for_severity(severity),
        source_id: source.id.clone(),
        kind: source.kind.as_str().into(),
        artifact: artifact.into(),
        path: source
            .applies_to
            .first()
            .cloned()
            .unwrap_or_else(|| artifact.into()),
        line: None,
        message: "configured coverage/proof evidence artifact is missing".into(),
        evidence: vec![format!("missing artifact candidate `{artifact}`")],
        repair: format!(
            "run the `{}` producer lane, write `{artifact}`, then rerun `jankurai coverage audit`",
            source.id
        ),
        owner: source.owner.clone(),
        lane: source.lane.clone(),
    }
}

fn parser_error_findings(
    source: &CoverageSource,
    artifact: &str,
    error: String,
    strict: bool,
) -> Vec<CoverageFinding> {
    let severity = if strict || source.mode == CoverageMode::Required {
        "high"
    } else {
        "medium"
    };
    vec![CoverageFinding {
        rule_id: primary_rule(source, "HLT-008-FALSE-GREEN-RISK"),
        severity: severity.into(),
        confidence: confidence_for_severity(severity),
        source_id: source.id.clone(),
        kind: source.kind.as_str().into(),
        artifact: artifact.into(),
        path: artifact.into(),
        line: None,
        message: "coverage/proof artifact could not be parsed".into(),
        evidence: vec![error],
        repair: "regenerate the artifact with the configured producer and rerun `jankurai coverage audit`".into(),
        owner: source.owner.clone(),
        lane: source.lane.clone(),
    }]
}

fn analyze_lcov(
    source: &CoverageSource,
    artifact: &str,
    report: &LcovReport,
    changed_lines: &BTreeMap<String, BTreeSet<usize>>,
    strict: bool,
    metrics: &mut BTreeMap<String, CoverageMetric>,
) -> Result<Vec<CoverageFinding>> {
    let total_coverage = ratio(report.covered_lines, report.total_lines);
    metrics.insert("total_lines".into(), json!(report.total_lines));
    metrics.insert("covered_lines".into(), json!(report.covered_lines));
    metrics.insert("total_line_coverage".into(), json!(total_coverage));
    metrics.insert("total_branches".into(), json!(report.total_branches));
    metrics.insert("covered_branches".into(), json!(report.covered_branches));

    let mut findings = Vec::new();
    let mut changed_total = 0usize;
    let mut changed_covered = 0usize;
    for (path, lines) in changed_lines {
        if !source_matches_path(source, path)? {
            continue;
        }
        let file = report.files.get(path);
        for line in lines {
            changed_total += 1;
            let count = file
                .and_then(|file| file.lines.get(line))
                .copied()
                .unwrap_or(0);
            if count > 0 {
                changed_covered += 1;
            } else {
                let severity = if source.mode == CoverageMode::Required || strict {
                    "high"
                } else {
                    "medium"
                };
                findings.push(CoverageFinding {
                    rule_id: primary_rule(source, "HLT-008-FALSE-GREEN-RISK"),
                    severity: severity.into(),
                    confidence: 0.88,
                    source_id: source.id.clone(),
                    kind: source.kind.as_str().into(),
                    artifact: artifact.into(),
                    path: path.clone(),
                    line: Some(*line),
                    message: "uncovered changed line is reachable but not proven".into(),
                    evidence: vec![
                        format!("changed line `{path}:{line}` has LCOV count `{count}`"),
                        "line coverage is reachability evidence, not behavioral proof".into(),
                    ],
                    repair: "add or strengthen behavior tests for this changed line, rerun the producer lane, then rerun `jankurai coverage audit`".into(),
                    owner: source.owner.clone(),
                    lane: source.lane.clone(),
                });
            }
        }
    }
    if changed_total > 0 {
        let changed_coverage = ratio(changed_covered, changed_total);
        metrics.insert("changed_lines".into(), json!(changed_total));
        metrics.insert("covered_changed_lines".into(), json!(changed_covered));
        metrics.insert("changed_line_coverage".into(), json!(changed_coverage));
        if let Some(threshold) = source.hard_changed_line_coverage {
            if changed_coverage < threshold && findings.is_empty() {
                let severity = if source.mode == CoverageMode::Required || strict {
                    "high"
                } else {
                    "medium"
                };
                findings.push(CoverageFinding {
                    rule_id: primary_rule(source, "HLT-008-FALSE-GREEN-RISK"),
                    severity: severity.into(),
                    confidence: 0.88,
                    source_id: source.id.clone(),
                    kind: source.kind.as_str().into(),
                    artifact: artifact.into(),
                    path: artifact.into(),
                    line: None,
                    message: "changed-line coverage is below configured threshold".into(),
                    evidence: vec![format!(
                        "changed_line_coverage={changed_coverage:.3} threshold={threshold:.3}"
                    )],
                    repair: "add behavior tests for uncovered changed paths and rerun the coverage producer lane".into(),
                    owner: source.owner.clone(),
                    lane: source.lane.clone(),
                });
            }
        }
    }

    if let Some(threshold) = source.soft_total_line_coverage {
        if total_coverage < threshold {
            findings.push(CoverageFinding {
                rule_id: primary_rule(source, "HLT-008-FALSE-GREEN-RISK"),
                severity: "medium".into(),
                confidence: 0.62,
                source_id: source.id.clone(),
                kind: source.kind.as_str().into(),
                artifact: artifact.into(),
                path: artifact.into(),
                line: None,
                message: "total line coverage is below advisory threshold".into(),
                evidence: vec![format!(
                    "total_line_coverage={total_coverage:.3} threshold={threshold:.3}"
                )],
                repair: "review untested source areas and add targeted behavior tests where the changed surface depends on them".into(),
                owner: source.owner.clone(),
                lane: source.lane.clone(),
            });
        }
    }
    Ok(findings)
}

fn analyze_mutation(
    source: &CoverageSource,
    artifact: &str,
    report: &MutationReport,
    changed_paths: &BTreeSet<String>,
    strict: bool,
) -> Vec<CoverageFinding> {
    let threshold = source.hard_survivors_on_changed_paths.unwrap_or(1) as usize;
    let relevant_survivors = report
        .mutants
        .iter()
        .filter(|mutant| mutant.status == "survived")
        .filter(|mutant| {
            changed_paths.is_empty()
                || changed_paths.contains(&mutant.path)
                || source_matches_path(source, &mutant.path).unwrap_or(false)
        })
        .collect::<Vec<_>>();
    if relevant_survivors.len() < threshold || relevant_survivors.is_empty() {
        return Vec::new();
    }
    let severity = if source.mode == CoverageMode::Required || strict {
        "high"
    } else {
        "medium"
    };
    relevant_survivors
        .into_iter()
        .take(PER_SOURCE_FINDINGS_CAP)
        .map(|mutant| CoverageFinding {
            rule_id: primary_rule(source, "HLT-008-FALSE-GREEN-RISK"),
            severity: severity.into(),
            confidence: 0.95,
            source_id: source.id.clone(),
            kind: source.kind.as_str().into(),
            artifact: artifact.into(),
            path: if mutant.path.is_empty() {
                artifact.into()
            } else {
                mutant.path.clone()
            },
            line: mutant.line,
            message: "mutation survivor indicates weak or missing behavioral assertions".into(),
            evidence: vec![
                format!("status={}", mutant.status),
                mutant.message.clone(),
                format!(
                    "mutation totals killed={} survived={} timeout={} unviable={} skipped={} total={}",
                    report.killed, report.survived, report.timeout, report.unviable, report.skipped, report.total
                ),
            ],
            repair: "add or strengthen tests around the changed path, confirm the mutant is killed, and rerun the mutation lane".into(),
            owner: source.owner.clone(),
            lane: source.lane.clone(),
        })
        .collect()
}

fn analyze_security(
    source: &CoverageSource,
    artifact: &str,
    report: &SecurityReport,
    strict: bool,
    metrics: &mut BTreeMap<String, CoverageMetric>,
) -> Vec<CoverageFinding> {
    metrics.insert("critical".into(), json!(report.critical));
    metrics.insert("high".into(), json!(report.high));
    metrics.insert("medium".into(), json!(report.medium));
    metrics.insert("low".into(), json!(report.low));
    metrics.insert(
        "vulnerabilities".into(),
        json!(report.vulnerabilities.len()),
    );
    let critical_threshold = source.hard_critical_vulnerabilities.unwrap_or(1) as usize;
    let high_threshold = source
        .hard_high_vulnerabilities
        .unwrap_or(usize::MAX as u64) as usize;
    report
        .vulnerabilities
        .iter()
        .filter(|issue| {
            (issue.severity == "CRITICAL" && report.critical >= critical_threshold)
                || (issue.severity == "HIGH" && report.high >= high_threshold)
        })
        .take(PER_SOURCE_FINDINGS_CAP)
        .map(|issue| {
            let severity = if issue.severity == "CRITICAL" || source.mode == CoverageMode::Required || strict {
                "high"
            } else {
                "medium"
            };
            CoverageFinding {
                rule_id: security_rule_for(source, &issue.target),
                severity: severity.into(),
                confidence: 0.95,
                source_id: source.id.clone(),
                kind: source.kind.as_str().into(),
                artifact: artifact.into(),
                path: issue.target.clone(),
                line: None,
                message: "coverage-adjacent supply-chain scanner reported a blocking vulnerability".into(),
                evidence: vec![
                    format!("{} {} severity={}", issue.vulnerability_id, issue.package_name, issue.severity),
                    issue.title.clone(),
                ],
                repair: "upgrade, remove, or explicitly review the affected dependency or image layer, then rerun the scanner and `jankurai coverage audit`".into(),
                owner: source.owner.clone(),
                lane: source.lane.clone(),
            }
        })
        .collect()
}

fn analyze_hadolint(
    source: &CoverageSource,
    artifact: &str,
    report: &ContainerLintReport,
    strict: bool,
    metrics: &mut BTreeMap<String, CoverageMetric>,
) -> Vec<CoverageFinding> {
    metrics.insert("diagnostics".into(), json!(report.diagnostics.len()));
    report
        .diagnostics
        .iter()
        .take(PER_SOURCE_FINDINGS_CAP)
        .map(|issue| {
            let severity = match issue.level.as_str() {
                "error" if source.mode == CoverageMode::Required || strict => "high",
                "error" | "warning" => "medium",
                "info" | "style" => "info",
                _ => "low",
            };
            CoverageFinding {
                rule_id: primary_rule(source, "HLT-032-DOCKER-BAD-BEHAVIOR"),
                severity: severity.into(),
                confidence: confidence_for_severity(severity),
                source_id: source.id.clone(),
                kind: source.kind.as_str().into(),
                artifact: artifact.into(),
                path: issue.file.clone(),
                line: issue.line,
                message: "Hadolint reported Dockerfile behavior that needs review".into(),
                evidence: vec![format!("{} {}: {}", issue.level, issue.code, issue.message)],
                repair: "fix the Dockerfile lint finding or document a reviewed container-build exception, then rerun Hadolint and `jankurai coverage audit`".into(),
                owner: source.owner.clone(),
                lane: source.lane.clone(),
            }
        })
        .collect()
}

fn parse_generic_json_summary(
    path: &Path,
    max_bytes: u64,
) -> Result<(BTreeMap<String, CoverageMetric>, Vec<Value>)> {
    let text = read_bounded_text(path, max_bytes)?;
    let value: Value = serde_json::from_str(&text).context("parse generic JSON summary")?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("generic-json-summary must be an object"))?;
    if !object.contains_key("status")
        || !object.contains_key("metrics")
        || !object.contains_key("findings")
    {
        bail!("generic-json-summary requires status, metrics, and findings");
    }
    let metrics = object
        .get("metrics")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("generic-json-summary metrics must be an object"))?
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let findings = object
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("generic-json-summary findings must be an array"))?
        .clone();
    Ok((metrics, findings))
}

fn normalize_imported_findings(
    source: &CoverageSource,
    artifact: &str,
    imported: Vec<Value>,
    strict: bool,
) -> Vec<CoverageFinding> {
    imported
        .into_iter()
        .filter_map(|value| {
            let object = value.as_object()?;
            let repair = object
                .get("repair")
                .or_else(|| object.get("fix"))
                .and_then(Value::as_str)?;
            if repair.trim().is_empty() {
                return None;
            }
            let mut severity = object
                .get("severity")
                .and_then(Value::as_str)
                .unwrap_or("medium")
                .to_ascii_lowercase();
            if source.mode != CoverageMode::Required && !strict && is_hard(&severity) {
                severity = "medium".into();
            }
            Some(CoverageFinding {
                rule_id: normalize_rule_id(
                    object
                        .get("rule_id")
                        .and_then(Value::as_str)
                        .unwrap_or(&primary_rule(source, "HLT-008-FALSE-GREEN-RISK")),
                ),
                severity: severity.clone(),
                confidence: object
                    .get("confidence")
                    .and_then(Value::as_f64)
                    .unwrap_or_else(|| confidence_for_severity(&severity)),
                source_id: source.id.clone(),
                kind: source.kind.as_str().into(),
                artifact: artifact.into(),
                path: object
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or(artifact)
                    .to_string(),
                line: object
                    .get("line")
                    .and_then(Value::as_u64)
                    .map(|line| line as usize),
                message: object
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("generic coverage/proof finding")
                    .to_string(),
                evidence: object
                    .get("evidence")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(ToString::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                repair: repair.to_string(),
                owner: source.owner.clone(),
                lane: source.lane.clone(),
            })
        })
        .collect()
}

fn render_coverage_markdown(audit: &CoverageAudit) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Coverage Audit");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Status: `{}`", audit.summary.status);
    let _ = writeln!(
        out,
        "- Sources: `{}` present=`{}` missing=`{}`",
        audit.summary.sources_total, audit.summary.sources_present, audit.summary.sources_missing
    );
    let _ = writeln!(
        out,
        "- Findings: hard=`{}` soft=`{}`",
        audit.summary.hard_findings, audit.summary.soft_findings
    );
    let _ = writeln!(out, "- Config: `{}`", audit.config_path);
    if let Some(base) = &audit.changed_from {
        let _ = writeln!(out, "- Changed from: `{}`", base);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Sources");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Source | Kind | Format | Mode | Status | Artifact |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
    for source in &audit.sources {
        let artifact = source.matched_artifact.as_deref().unwrap_or("missing");
        let _ = writeln!(
            out,
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |",
            source.id, source.kind, source.format, source.mode, source.status, artifact
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Findings");
    let _ = writeln!(out);
    if audit.findings.is_empty() {
        let _ = writeln!(out, "No coverage evidence findings.");
    } else {
        for (idx, finding) in audit.findings.iter().enumerate() {
            let loc = finding
                .line
                .map(|line| format!("{}:{line}", finding.path))
                .unwrap_or_else(|| finding.path.clone());
            let _ = writeln!(
                out,
                "{}. `{}` `{}` `{}` - {}",
                idx + 1,
                finding.severity,
                finding.rule_id,
                loc,
                finding.message
            );
            let _ = writeln!(out, "   Artifact: `{}`", finding.artifact);
            let _ = writeln!(out, "   Repair: {}", finding.repair);
            if !finding.evidence.is_empty() {
                let _ = writeln!(out, "   Evidence: {}", finding.evidence.join("; "));
            }
        }
    }
    out
}

fn flush_lcov_file(report: &mut LcovReport, path: String, file: LcovFile) {
    report.total_lines += file.lines.len();
    report.covered_lines += file.lines.values().filter(|count| **count > 0).count();
    report.files.insert(path, file);
}

fn collect_mutation_outcomes(
    value: &Value,
    parent_path: Option<&str>,
    outcomes: &mut Vec<MutationOutcome>,
) {
    if let Some(outcome) = mutation_outcome_from_value(value, parent_path) {
        outcomes.push(outcome);
    }
    match value {
        Value::Array(items) => {
            for item in items {
                collect_mutation_outcomes(item, parent_path, outcomes);
            }
        }
        Value::Object(object) => {
            for (key, child) in object {
                let next_parent = if key.ends_with(".rs")
                    || key.ends_with(".ts")
                    || key.ends_with(".tsx")
                    || key.ends_with(".js")
                {
                    Some(key.as_str())
                } else {
                    parent_path
                };
                collect_mutation_outcomes(child, next_parent, outcomes);
            }
        }
        _ => {}
    }
}

fn mutation_outcome_from_value(
    value: &Value,
    parent_path: Option<&str>,
) -> Option<MutationOutcome> {
    let object = value.as_object()?;
    let raw_status = object
        .get("status")
        .or_else(|| object.get("outcome"))
        .or_else(|| object.get("result"))
        .and_then(Value::as_str)?;
    let status = normalize_mutation_status(raw_status)?;
    let path = string_field(object, &["path", "file", "filename", "source_file"])
        .or_else(|| {
            object
                .get("mutant")
                .and_then(Value::as_object)
                .and_then(|object| {
                    string_field(object, &["path", "file", "filename", "source_file"])
                })
        })
        .or_else(|| parent_path.map(ToString::to_string))
        .unwrap_or_default();
    let line = object
        .get("line")
        .or_else(|| object.get("start_line"))
        .and_then(Value::as_u64)
        .map(|line| line as usize)
        .or_else(|| {
            object
                .get("location")
                .and_then(|location| location.get("start"))
                .and_then(|start| start.get("line"))
                .and_then(Value::as_u64)
                .map(|line| line as usize)
        });
    let message = string_field(object, &["name", "mutatorName", "description", "id"])
        .unwrap_or_else(|| raw_status.to_string());
    Some(MutationOutcome {
        path: normalize_rel_string(&path),
        line,
        status,
        message,
    })
}

fn normalize_mutation_status(status: &str) -> Option<String> {
    let lower = status.to_ascii_lowercase().replace(['-', '_', ' '], "");
    let normalized = match lower.as_str() {
        "killed" | "caught" | "success" => "killed",
        "survived" | "missed" | "nocoverage" => "survived",
        "timeout" | "timedout" => "timeout",
        "unviable" | "compileerror" | "error" | "runtimeerror" => "unviable",
        "skipped" | "ignored" => "skipped",
        _ => return None,
    };
    Some(normalized.into())
}

fn build_mutation_report(mutants: Vec<MutationOutcome>) -> MutationReport {
    let mut report = MutationReport {
        total: mutants.len(),
        mutants,
        ..MutationReport::default()
    };
    for mutant in &report.mutants {
        match mutant.status.as_str() {
            "killed" => report.killed += 1,
            "survived" => report.survived += 1,
            "timeout" => report.timeout += 1,
            "unviable" => report.unviable += 1,
            "skipped" => report.skipped += 1,
            _ => {}
        }
    }
    report
}

fn string_field(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(ToString::to_string)
}

fn status_for_findings(findings: &[CoverageFinding]) -> String {
    if findings.iter().any(|finding| is_hard(&finding.severity)) {
        "fail".into()
    } else if findings.is_empty() {
        "pass".into()
    } else {
        "warn".into()
    }
}

fn cap_source_findings(source: &CoverageSource, findings: &mut Vec<CoverageFinding>) {
    if findings.len() <= PER_SOURCE_FINDINGS_CAP {
        return;
    }
    sort_findings(findings);
    let omitted = findings.len() - PER_SOURCE_FINDINGS_CAP;
    findings.truncate(PER_SOURCE_FINDINGS_CAP);
    findings.push(CoverageFinding {
        rule_id: primary_rule(source, "HLT-008-FALSE-GREEN-RISK"),
        severity: "info".into(),
        confidence: 0.62,
        source_id: source.id.clone(),
        kind: source.kind.as_str().into(),
        artifact: artifact_label(source),
        path: ".".into(),
        line: None,
        message: "additional coverage findings omitted by per-source cap".into(),
        evidence: vec![format!("omitted_findings={omitted}")],
        repair: "narrow the producer artifact or fix the highest-severity findings first, then rerun `jankurai coverage audit`".into(),
        owner: source.owner.clone(),
        lane: source.lane.clone(),
    });
}

fn cap_global_findings(findings: &mut Vec<CoverageFinding>, max_findings: usize) {
    if findings.len() <= max_findings {
        return;
    }
    sort_findings(findings);
    let omitted = findings.len() - max_findings;
    findings.truncate(max_findings);
    findings.push(CoverageFinding {
        rule_id: "HLT-008-FALSE-GREEN-RISK".into(),
        severity: "info".into(),
        confidence: 0.62,
        source_id: "coverage-audit".into(),
        kind: "jankurai_artifact".into(),
        artifact: DEFAULT_JSON_PATH.into(),
        path: ".".into(),
        line: None,
        message: "additional coverage findings omitted by global cap".into(),
        evidence: vec![format!("omitted_findings={omitted}")],
        repair:
            "raise `--max-findings` for investigation or fix the highest-severity findings first"
                .into(),
        owner: "agent".into(),
        lane: "coverage-audit".into(),
    });
}

fn dedup_findings(findings: &mut Vec<CoverageFinding>) {
    let mut seen = BTreeSet::new();
    findings.retain(|finding| {
        seen.insert(format!(
            "{}\0{}\0{}\0{}\0{:?}\0{}",
            finding.rule_id,
            finding.source_id,
            finding.artifact,
            finding.path,
            finding.line,
            finding.message
        ))
    });
}

fn sort_findings(findings: &mut [CoverageFinding]) {
    findings.sort_by(|a, b| {
        severity_rank(&b.severity)
            .cmp(&severity_rank(&a.severity))
            .then_with(|| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.source_id.cmp(&b.source_id))
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.line.cmp(&b.line))
    });
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 5,
        "high" => 4,
        "medium" => 3,
        "low" => 2,
        "info" => 1,
        _ => 0,
    }
}

fn is_hard(severity: &str) -> bool {
    matches!(severity, "critical" | "high")
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        1.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn primary_rule(source: &CoverageSource, fallback: &str) -> String {
    source
        .rules
        .iter()
        .find_map(|rule| {
            let normalized = normalize_rule_id(rule);
            rules::lookup(&normalized).map(|_| normalized)
        })
        .unwrap_or_else(|| fallback.into())
}

fn security_rule_for(source: &CoverageSource, target: &str) -> String {
    let target_lower = target.to_ascii_lowercase();
    if (target_lower.contains("docker") || target_lower.contains("container"))
        && source
            .rules
            .iter()
            .any(|rule| normalize_rule_id(rule) == "HLT-032-DOCKER-BAD-BEHAVIOR")
    {
        return "HLT-032-DOCKER-BAD-BEHAVIOR".into();
    }
    if source
        .rules
        .iter()
        .any(|rule| normalize_rule_id(rule) == "HLT-016-SUPPLY-CHAIN-DRIFT")
    {
        "HLT-016-SUPPLY-CHAIN-DRIFT".into()
    } else {
        primary_rule(source, "HLT-016-SUPPLY-CHAIN-DRIFT")
    }
}

fn normalize_rule_id(rule: &str) -> String {
    match rule {
        "HLT-016-SUPPLY-CHAIN-RISK" => "HLT-016-SUPPLY-CHAIN-DRIFT".into(),
        _ if rules::lookup(rule).is_some() => rule.into(),
        _ => "HLT-008-FALSE-GREEN-RISK".into(),
    }
}

fn auto_source_enabled(repo_root: &Path, source: &CoverageSource) -> Result<bool> {
    if first_existing_artifact(repo_root, source)?.is_some() {
        return Ok(true);
    }
    if source.applies_to.is_empty() {
        return Ok(true);
    }
    let matcher = globset_for(&source.applies_to)?;
    for entry in walkdir::WalkDir::new(repo_root)
        .into_iter()
        .filter_entry(|entry| entry.file_name() != ".git" && entry.file_name() != "target")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let rel = display_rel(repo_root, entry.path());
        if matcher.is_match(rel.as_str()) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn source_matches_path(source: &CoverageSource, path: &str) -> Result<bool> {
    if source.applies_to.is_empty() {
        return Ok(true);
    }
    Ok(globset_for(&source.applies_to)?.is_match(path))
}

fn globset_for(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern).with_context(|| format!("compile glob `{pattern}`"))?);
    }
    builder.build().context("build coverage source globset")
}

fn changed_lines_from_git(
    repo_root: &Path,
    base: &str,
) -> Result<BTreeMap<String, BTreeSet<usize>>> {
    let output = Command::new("git")
        .args(["diff", "--unified=0", base, "--"])
        .current_dir(repo_root)
        .output()
        .with_context(|| format!("run git diff from {base}"))?;
    if !output.status.success() {
        bail!(
            "git diff from {base} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(parse_changed_lines_diff(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_changed_lines_diff(diff: &str) -> BTreeMap<String, BTreeSet<usize>> {
    let mut out = BTreeMap::<String, BTreeSet<usize>>::new();
    let mut current_file: Option<String> = None;
    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_file = Some(normalize_rel_string(path));
            continue;
        }
        if line.starts_with("+++ /dev/null") {
            current_file = None;
            continue;
        }
        let Some(hunk) = line.strip_prefix("@@ ") else {
            continue;
        };
        let Some(file) = current_file.as_ref() else {
            continue;
        };
        let Some(plus_idx) = hunk.find('+') else {
            continue;
        };
        let rest = &hunk[plus_idx + 1..];
        let end = rest.find([' ', '@']).unwrap_or(rest.len());
        let range = &rest[..end];
        let mut parts = range.split(',');
        let start = parts
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let count = parts
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1);
        if start == 0 || count == 0 {
            continue;
        }
        let lines = out.entry(file.clone()).or_default();
        for changed_line in start..start + count {
            lines.insert(changed_line);
        }
    }
    out
}

fn read_bounded_text(path: &Path, max_bytes: u64) -> Result<String> {
    let metadata = fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if metadata.len() > max_bytes {
        bail!(
            "artifact {} is {} bytes, above max_artifact_bytes {}",
            path.display(),
            metadata.len(),
            max_bytes
        );
    }
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    if bytes.len() as u64 > max_bytes {
        bail!(
            "artifact {} exceeded max_artifact_bytes {} while reading",
            path.display(),
            max_bytes
        );
    }
    String::from_utf8(bytes).with_context(|| format!("{} is not valid UTF-8", path.display()))
}

fn resolve_existing_or_relative(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn resolve_artifact_path(repo_root: &Path, artifact: &str) -> Result<PathBuf> {
    let raw = Path::new(artifact);
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        repo_root.join(raw)
    };
    let normalized = normalize_path(&joined)?;
    if raw.is_absolute() {
        let repo = repo_root
            .canonicalize()
            .unwrap_or_else(|_| repo_root.to_path_buf());
        if !normalized.starts_with(&repo) {
            bail!(
                "coverage artifact path `{artifact}` escapes repo root `{}`",
                repo.display()
            );
        }
    } else if artifact.split('/').any(|part| part == "..") {
        bail!("coverage artifact path `{artifact}` escapes repo root");
    }
    Ok(normalized)
}

fn normalize_path(path: &Path) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(part) => out.push(part),
            Component::ParentDir => {
                if !out.pop() {
                    bail!("path escapes root: {}", path.display());
                }
            }
        }
    }
    Ok(out)
}

fn normalize_source_path(repo_root: &Path, path: &str) -> String {
    let raw = Path::new(path);
    if raw.is_absolute() {
        raw.strip_prefix(repo_root)
            .map(normalize_rel_path)
            .unwrap_or_else(|_| normalize_rel_path(raw))
    } else {
        normalize_rel_path(raw)
    }
}

fn normalize_rel_path(path: &Path) -> String {
    normalize_rel_string(&path.to_string_lossy())
}

fn normalize_rel_string(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn display_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(normalize_rel_path)
        .unwrap_or_else(|_| normalize_rel_path(path))
}

fn malformed_score_ingest(config_present: bool, error: String) -> CoverageScoreIngest {
    CoverageScoreIngest {
        summary: Some(CoverageEvidenceSummary {
            artifact: DEFAULT_JSON_PATH.into(),
            status: "fail".into(),
            sources_total: 0,
            sources_present: 0,
            hard_findings: 1,
            soft_findings: 0,
        }),
        findings: vec![CoverageFinding {
            rule_id: "HLT-027-HUMAN-REVIEW-EVIDENCE-GAP".into(),
            severity: "high".into(),
            confidence: 0.88,
            source_id: "coverage-evidence".into(),
            kind: "jankurai_artifact".into(),
            artifact: DEFAULT_JSON_PATH.into(),
            path: DEFAULT_JSON_PATH.into(),
            line: None,
            message: "coverage audit artifact is malformed or unreadable".into(),
            evidence: vec![error.clone()],
            repair: "regenerate `target/jankurai/coverage/coverage-audit.json` with `jankurai coverage audit`".into(),
            owner: "agent".into(),
            lane: "coverage-audit".into(),
        }],
        config_present,
        artifact_malformed: Some(error),
    }
}

fn coverage_finding_to_score_finding(finding: &CoverageFinding) -> Finding {
    let rule_id = normalize_rule_id(&finding.rule_id);
    let rule = rules::lookup(&rule_id);
    let category = rule.map(|rule| rule.category).unwrap_or("proof");
    let severity = if finding.severity == "info" {
        "low"
    } else {
        finding.severity.as_str()
    };
    let lane = if finding.lane.is_empty() {
        rule.map(|rule| rule.lane).unwrap_or("coverage-audit")
    } else {
        finding.lane.as_str()
    };
    let evidence_kind = rule
        .map(|rule| rule.evidence_kind)
        .unwrap_or("coverage-evidence");
    let tlr = rule.map(|rule| rule.tlr).unwrap_or("Verification");
    let docs_url = rule
        .map(|rule| rule.docs_url)
        .unwrap_or("docs/COVERAGE_MASTER.md");
    let fingerprint = finding_fingerprint(
        &rule_id,
        category,
        &finding.path,
        &finding.message,
        &finding.evidence,
    );
    Finding {
        severity: severity.into(),
        category: category.into(),
        path: finding.path.clone(),
        problem: finding.message.clone(),
        agent_fix: finding.repair.clone(),
        evidence: finding.evidence.clone(),
        check_id: format!("{rule_id}:coverage-evidence"),
        hardness: hardness_for_severity(severity).into(),
        confidence: finding.confidence.max(confidence_for_severity(severity)),
        evidence_kind: evidence_kind.into(),
        rerun_command: if lane == "coverage-audit" {
            "cargo run -p jankurai -- coverage audit . --config agent/coverage-sources.toml --json target/jankurai/coverage/coverage-audit.json --md target/jankurai/coverage/coverage-audit.md".into()
        } else {
            rerun_command_for_lane(Some(lane)).into()
        },
        fingerprint,
        rule_id: Some(rule_id),
        tlr: Some(tlr.into()),
        lane: Some(lane.into()),
        docs_url: Some(docs_url.into()),
        owner: Some(if finding.owner.is_empty() {
            rule.map(|rule| rule.owner_hint).unwrap_or("agent").into()
        } else {
            finding.owner.clone()
        }),
        line: finding.line,
        matched_term: Some(finding.source_id.clone()),
        reason: Some(format!("coverage evidence artifact `{}`", finding.artifact)),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_changed_lines_diff;

    #[test]
    fn parses_unified_zero_changed_lines() {
        let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,0 +2,2 @@
+a
+b
";
        let lines = parse_changed_lines_diff(diff);
        assert!(lines["src/lib.rs"].contains(&2));
        assert!(lines["src/lib.rs"].contains(&3));
    }
}
