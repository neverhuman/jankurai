use crate::commands::bench;
use crate::commands::exceptions::repo_relative_path;
use crate::commands::release_data::load_release_data;
use crate::commands::repair::now_string;
use crate::validation::{self, ArtifactSchema};
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct OptimizeArgs {
    pub repo: PathBuf,
    pub mode: String,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimizationReport {
    pub schema_version: String,
    pub repo: String,
    pub generated_at: String,
    pub status: String,
    pub mode: String,
    pub target_stack_id: String,
    pub context_size_before_bytes: usize,
    pub context_size_after_bytes: usize,
    pub estimated_tokens_before: usize,
    pub estimated_tokens_after: usize,
    pub context_files: Vec<ContextFileSummary>,
    pub benchmark_summary: bench::BenchmarkSummary,
    pub findings: Vec<OptimizationFinding>,
    pub proof_requirements: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextFileSummary {
    pub path: String,
    pub bytes: usize,
    pub duplicate_bytes: usize,
    pub duplicate_lines: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimizationFinding {
    pub kind: String,
    pub subject: String,
    pub path: String,
    pub signal: String,
    pub recommended_action: String,
    pub proof_requirement: String,
    pub risk_level: String,
}

#[derive(Debug, Clone)]
struct DuplicateBucket {
    line: String,
    occurrences: Vec<String>,
}

#[derive(Debug, Clone)]
struct SourceFile {
    path: String,
    text: String,
}

pub fn run(args: OptimizeArgs) -> Result<()> {
    let report = build_report(&args.repo, &args.mode)?;
    if let Some(path) = args.out.as_deref() {
        validation::write_json(
            &args.repo,
            ArtifactSchema::OptimizationReport,
            path,
            &report,
        )?;
    } else {
        validation::validate_serializable(&args.repo, ArtifactSchema::OptimizationReport, &report)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&report))?;
    }
    Ok(())
}

pub fn build_report(repo: &Path, mode: &str) -> Result<OptimizationReport> {
    let release = load_release_data(repo)?;
    let (context_files, context_before, context_after, token_findings) =
        analyze_context_files(repo)?;
    let benchmark_report =
        bench::build_benchmark_report(repo, &bench::build_benchmark_suite(repo)?)?;
    let benchmark_summary = benchmark_report.summary.clone();
    let mut findings = Vec::new();
    findings.extend(token_findings);
    findings.extend(performance_findings(repo, &benchmark_summary)?);
    findings.extend(dependency_findings(repo)?);
    findings.extend(dead_code_findings(repo)?);
    let findings = filter_findings(findings, mode);
    let proof_requirements = collect_proof_requirements(&findings, &benchmark_summary);
    let mut notes = vec![
        "optimization is advisory; this command does not mutate the repository".to_string(),
        "token reduction is estimated from duplicate line removal across the loaded context set"
            .to_string(),
    ];
    if benchmark_summary.failed > 0 || benchmark_summary.inconclusive > 0 {
        notes.push("benchmark proof still shows unresolved or inconclusive coverage".to_string());
    } else {
        notes.push("benchmark proof is currently clean for the bundled smoke suite".to_string());
    }
    if findings.is_empty() {
        notes
            .push("no optimization candidates were detected by the current heuristics".to_string());
    }
    Ok(OptimizationReport {
        schema_version: release.schema_version,
        repo: repo.display().to_string(),
        generated_at: now_string(),
        status: "complete".to_string(),
        mode: mode.to_string(),
        target_stack_id: release.target_stack_id,
        context_size_before_bytes: context_before,
        context_size_after_bytes: context_after,
        estimated_tokens_before: estimate_tokens(context_before),
        estimated_tokens_after: estimate_tokens(context_after),
        context_files,
        benchmark_summary,
        findings,
        proof_requirements,
        notes,
    })
}

fn analyze_context_files(
    repo: &Path,
) -> Result<(
    Vec<ContextFileSummary>,
    usize,
    usize,
    Vec<OptimizationFinding>,
)> {
    let mut files = Vec::new();
    for rel in [
        "AGENTS.md",
        "CLAUDE.md",
        "GEMINI.md",
        "agent/JANKURAI_STANDARD.md",
        "agent/MASTER_PLAN.md",
        "docs/agent-native-standard.md",
        "tips/phases/00-phase-index.md",
    ] {
        let path = repo.join(rel);
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("read context file {}", path.display()))?;
        files.push(SourceFile {
            path: rel.to_string(),
            text,
        });
    }

    let mut buckets: HashMap<String, DuplicateBucket> = HashMap::new();
    for file in &files {
        for line in file.text.lines() {
            let normalized = normalize_line(line);
            if normalized.len() < 24 {
                continue;
            }
            let entry = buckets
                .entry(normalized.clone())
                .or_insert_with(|| DuplicateBucket {
                    line: normalized.clone(),
                    occurrences: Vec::new(),
                });
            entry.occurrences.push(file.path.clone());
        }
    }

    let mut duplicate_entries = Vec::new();
    let mut total_before = 0usize;
    let mut duplicate_bytes = 0usize;
    let mut per_file: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for file in &files {
        total_before += file.text.len();
        per_file.entry(file.path.clone()).or_insert((0, 0)).1 = file.text.lines().count();
    }

    for bucket in buckets.values() {
        if bucket.occurrences.len() < 2 {
            continue;
        }
        let mut paths = bucket.occurrences.clone();
        paths.sort();
        let canonical = paths[0].clone();
        let repeated_bytes = bucket
            .occurrences
            .iter()
            .skip(1)
            .map(|path| {
                let bytes = line_bytes_for_path(&files, path, &bucket.line);
                let entry = per_file.entry(path.clone()).or_insert((0, 0));
                entry.0 += bytes;
                bytes
            })
            .sum::<usize>();
        duplicate_bytes += repeated_bytes;
        duplicate_entries.push(OptimizationFinding {
            kind: "token".to_string(),
            subject: format!(
                "duplicate context line across {} files",
                bucket.occurrences.len()
            ),
            path: canonical,
            signal: truncate_for_signal(&bucket.line),
            recommended_action:
                "keep the canonical copy and trim duplicate instruction text from the other files"
                    .to_string(),
            proof_requirement: "rerun just fast after the documentation split".to_string(),
            risk_level: "low".to_string(),
        });
    }

    duplicate_entries.sort_by(|a, b| a.signal.cmp(&b.signal));
    duplicate_entries.truncate(8);

    let context_files = files
        .into_iter()
        .map(|file| {
            let duplicate_bytes = per_file
                .get(&file.path)
                .map(|entry| entry.0)
                .unwrap_or_default();
            let duplicate_lines = per_file
                .get(&file.path)
                .map(|entry| entry.1)
                .unwrap_or_default();
            ContextFileSummary {
                path: file.path,
                bytes: file.text.len(),
                duplicate_bytes,
                duplicate_lines,
            }
        })
        .collect::<Vec<_>>();
    let total_after = total_before.saturating_sub(duplicate_bytes);
    Ok((context_files, total_before, total_after, duplicate_entries))
}

fn performance_findings(
    repo: &Path,
    benchmark_summary: &bench::BenchmarkSummary,
) -> Result<Vec<OptimizationFinding>> {
    let mut out = Vec::new();
    if let Some(report) = read_repo_score_findings(repo)? {
        for finding in report {
            let text = finding_signal_text(&finding);
            if contains_any(
                &text,
                &["speed", "perf", "performance", "benchmark", "regression"],
            ) {
                out.push(OptimizationFinding {
                    kind: "performance".to_string(),
                    subject: finding_subject(&finding),
                    path: finding_path(&finding),
                    signal: text.clone(),
                    recommended_action: finding_action(
                        &finding,
                        "measure the hot path, narrow the regression, and rerun the benchmark lane",
                    ),
                    proof_requirement: "run just bench and attach the benchmark report".to_string(),
                    risk_level: finding_risk_level(&finding, "medium"),
                });
            }
        }
    }
    if benchmark_summary.failed > 0 || benchmark_summary.inconclusive > 0 {
        out.push(OptimizationFinding {
            kind: "performance".to_string(),
            subject: "benchmark smoke suite".to_string(),
            path: "target/jankurai/benchmark-report.json".to_string(),
            signal: format!(
                "benchmark summary has passed={}, failed={}, inconclusive={}",
                benchmark_summary.passed, benchmark_summary.failed, benchmark_summary.inconclusive
            ),
            recommended_action:
                "re-run the benchmark suite and fix the fixture or task that remains unresolved"
                    .to_string(),
            proof_requirement: "run just bench before treating the regression as closed"
                .to_string(),
            risk_level: "medium".to_string(),
        });
    }
    Ok(out)
}

fn dependency_findings(repo: &Path) -> Result<Vec<OptimizationFinding>> {
    let mut out = Vec::new();
    for manifest in collect_manifests(repo, "Cargo.toml")? {
        let text = fs::read_to_string(&manifest)
            .with_context(|| format!("read manifest {}", manifest.display()))?;
        let value: toml::Value = toml::from_str(&text)
            .with_context(|| format!("parse manifest {}", manifest.display()))?;
        let package_name = value
            .get("package")
            .and_then(|value| value.get("name"))
            .and_then(|value| value.as_str())
            .unwrap_or("workspace")
            .to_string();
        let Some(dependencies) = dependency_tables(&value) else {
            continue;
        };
        for dep_name in dependencies {
            if dependency_is_used(
                repo,
                &manifest,
                manifest.parent().unwrap_or(repo),
                &dep_name,
            )? {
                continue;
            }
            out.push(OptimizationFinding {
                kind: "dependency".to_string(),
                subject: format!("{package_name}:{dep_name}"),
                path: repo_relative_path(repo, &manifest),
                signal: "no source references found for the declared dependency".to_string(),
                recommended_action:
                    "remove the dependency or prove the build/test path still requires it"
                        .to_string(),
                proof_requirement: format!(
                    "run cargo check -p {package_name} and cargo test -p {package_name}"
                ),
                risk_level: "low".to_string(),
            });
        }
    }
    Ok(out)
}

fn dead_code_findings(repo: &Path) -> Result<Vec<OptimizationFinding>> {
    let mut out = Vec::new();
    let score_path = repo.join("agent/repo-score.json");
    if !score_path.exists() {
        return Ok(out);
    }
    let text = fs::read_to_string(&score_path)
        .with_context(|| format!("read {}", score_path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", score_path.display()))?;
    let Some(findings) = value.get("findings").and_then(|value| value.as_array()) else {
        return Ok(out);
    };
    for finding in findings {
        let text = finding_signal_text(finding);
        if contains_any(
            &text,
            &[
                "placeholder",
                "dead code",
                "dead-marker",
                "unimplemented",
                "stub",
                "orphan",
            ],
        ) || finding_rule_id(finding) == "HLT-001-DEAD-MARKER"
        {
            out.push(OptimizationFinding {
                kind: "dead-code".to_string(),
                subject: finding_subject(finding),
                path: finding_path(finding),
                signal: text,
                recommended_action:
                    "replace placeholders with implemented behavior or move the code behind a tighter boundary"
                        .to_string(),
                proof_requirement: finding
                    .get("rerun_command")
                    .and_then(|value| value.as_str())
                    .unwrap_or("run just fast")
                    .to_string(),
                risk_level: finding_risk_level(finding, "high"),
            });
        }
    }
    Ok(out)
}

fn collect_proof_requirements(
    findings: &[OptimizationFinding],
    benchmark_summary: &bench::BenchmarkSummary,
) -> Vec<String> {
    let mut out = BTreeSet::new();
    out.insert("just fast".to_string());
    out.insert("cargo test -p jankurai".to_string());
    out.insert("just bench".to_string());
    if benchmark_summary.failed > 0 || benchmark_summary.inconclusive > 0 {
        out.insert("benchmark proof required before removing performance guards".to_string());
    }
    for finding in findings {
        out.insert(finding.proof_requirement.clone());
    }
    out.into_iter().collect()
}

fn dependency_tables(value: &toml::Value) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for table_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = value.get(table_name).and_then(|value| value.as_table()) {
            for (dep_name, dep_value) in table {
                if dep_value
                    .as_table()
                    .map(|table| table.contains_key("path") || table.contains_key("workspace"))
                    .unwrap_or(false)
                {
                    continue;
                }
                out.push(dep_name.to_string());
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        out.sort();
        out.dedup();
        Some(out)
    }
}

fn dependency_is_used(
    repo: &Path,
    manifest: &Path,
    source_root: &Path,
    dep_name: &str,
) -> Result<bool> {
    for path in walk_source_files(source_root)? {
        if path == manifest {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("lock") {
            continue;
        }
        let rel = repo_relative_path(repo, &path);
        if rel.contains("target/") || rel.starts_with("reference/") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("read candidate source {}", path.display()))?;
        if text.contains(dep_name) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn walk_source_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in WalkBuilder::new(root).hidden(true).build() {
        let entry = entry?;
        let path = entry.into_path();
        if should_skip_path(&path) {
            continue;
        }
        if path.is_file() {
            out.push(path);
        }
    }
    Ok(out)
}

fn collect_manifests(repo: &Path, name: &str) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for path in walk_source_files(repo)? {
        if path.file_name().and_then(|value| value.to_str()) == Some(name) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn should_skip_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some("target" | "reference" | "paper" | "node_modules" | ".git")
        )
    })
}

fn filter_findings(findings: Vec<OptimizationFinding>, mode: &str) -> Vec<OptimizationFinding> {
    if mode == "all" {
        return findings;
    }
    findings
        .into_iter()
        .filter(|finding| finding.kind == mode)
        .collect()
}

fn read_repo_score_findings(repo: &Path) -> Result<Option<Vec<serde_json::Value>>> {
    let path = repo.join("agent/repo-score.json");
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(value
        .get("findings")
        .and_then(|value| value.as_array())
        .cloned())
}

fn finding_signal_text(value: &serde_json::Value) -> String {
    value
        .get("problem")
        .and_then(|value| value.as_str())
        .or_else(|| value.get("reason").and_then(|value| value.as_str()))
        .or_else(|| value.get("agent_fix").and_then(|value| value.as_str()))
        .unwrap_or("optimization candidate")
        .to_string()
}

fn finding_subject(value: &serde_json::Value) -> String {
    value
        .get("path")
        .and_then(|value| value.as_str())
        .or_else(|| value.get("check_id").and_then(|value| value.as_str()))
        .or_else(|| value.get("rule_id").and_then(|value| value.as_str()))
        .unwrap_or("optimization candidate")
        .to_string()
}

fn finding_path(value: &serde_json::Value) -> String {
    value
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or("agent/repo-score.json")
        .to_string()
}

fn finding_rule_id(value: &serde_json::Value) -> String {
    value
        .get("rule_id")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

fn finding_risk_level(value: &serde_json::Value, fallback: &str) -> String {
    value
        .get("severity")
        .and_then(|value| value.as_str())
        .unwrap_or(fallback)
        .to_string()
}

fn finding_action(value: &serde_json::Value, fallback: &str) -> String {
    value
        .get("agent_fix")
        .and_then(|value| value.as_str())
        .unwrap_or(fallback)
        .to_string()
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn truncate_for_signal(value: &str) -> String {
    value.chars().take(120).collect()
}

fn normalize_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn line_bytes_for_path(files: &[SourceFile], path: &str, line: &str) -> usize {
    files
        .iter()
        .find(|file| {
            file.path == path
                && file
                    .text
                    .lines()
                    .any(|candidate| normalize_line(candidate) == line)
        })
        .and_then(|file| {
            file.text
                .lines()
                .find(|candidate| normalize_line(candidate) == line)
                .map(|candidate| candidate.len() + 1)
        })
        .unwrap_or_else(|| line.len() + 1)
}

fn estimate_tokens(bytes: usize) -> usize {
    bytes.div_ceil(4)
}

fn render_markdown(report: &OptimizationReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Optimization Report");
    let _ = writeln!(out);
    let _ = writeln!(out, "- repo: `{}`", report.repo);
    let _ = writeln!(out, "- mode: `{}`", report.mode);
    let _ = writeln!(out, "- status: `{}`", report.status);
    let _ = writeln!(out, "- target stack: `{}`", report.target_stack_id);
    let _ = writeln!(
        out,
        "- context size: `{}` bytes -> `{}` bytes",
        report.context_size_before_bytes, report.context_size_after_bytes
    );
    let _ = writeln!(
        out,
        "- estimated tokens: `{}` -> `{}`",
        report.estimated_tokens_before, report.estimated_tokens_after
    );
    let _ = writeln!(
        out,
        "- benchmark summary: passed=`{}` failed=`{}` inconclusive=`{}`",
        report.benchmark_summary.passed,
        report.benchmark_summary.failed,
        report.benchmark_summary.inconclusive
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "## Findings");
    let _ = writeln!(out);
    for finding in &report.findings {
        let _ = writeln!(
            out,
            "- `[{}]` {} -> {}",
            finding.kind, finding.path, finding.recommended_action
        );
        let _ = writeln!(out, "  signal: `{}`", finding.signal);
        let _ = writeln!(out, "  proof: `{}`", finding.proof_requirement);
    }
    if !report.proof_requirements.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Proof");
        let _ = writeln!(out);
        for proof in &report.proof_requirements {
            let _ = writeln!(out, "- {}", proof);
        }
    }
    out
}
