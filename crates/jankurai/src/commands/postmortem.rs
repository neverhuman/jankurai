use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

use crate::validation::{self, ArtifactSchema};

#[derive(Debug, Clone)]
pub struct PostmortemRecordArgs {
    pub repo: PathBuf,
    pub input: String,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PostmortemListArgs {
    pub repo: PathBuf,
    pub root: String,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PostmortemShowArgs {
    pub repo: PathBuf,
    pub root: String,
    pub postmortem_id: String,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PostmortemReadArgs {
    pub repo: PathBuf,
    pub path: String,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailureMode {
    AspirationalSpec,
    EnvPrerequisite,
    InteropRuntime,
    EquivalenceGap,
    CutoverRollback,
    PerfRegression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmortemEntry {
    pub schema_version: String,
    pub postmortem_id: String,
    pub title: String,
    pub owner: String,
    pub failure_mode: FailureMode,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocker_type: Option<FailureMode>,
    pub summary: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PostmortemEntrySummary {
    postmortem_id: String,
    title: String,
    owner: String,
    failure_mode: FailureMode,
    severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    blocker_type: Option<FailureMode>,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PostmortemRecordReport {
    schema_version: String,
    command: String,
    status: String,
    repo: String,
    root: String,
    record_path: String,
    record: PostmortemEntry,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PostmortemListReport {
    schema_version: String,
    command: String,
    status: String,
    repo: String,
    root: String,
    records_total: usize,
    records: Vec<PostmortemEntrySummary>,
    notes: Vec<String>,
}

pub fn run_record(args: PostmortemRecordArgs) -> Result<()> {
    let repo = canonicalize_repo(&args.repo)?;
    let input_path = resolve_repo_relative_existing(&repo, &args.input)?;
    let input_text = fs::read_to_string(&input_path)
        .with_context(|| format!("read {}", input_path.display()))?;
    let entry = parse_entry(&repo, &input_text)?;
    let record_path =
        args.out.as_deref().map(PathBuf::from).unwrap_or_else(|| {
            postmortem_root(&repo).join(format!("{}.toml", entry.postmortem_id))
        });
    if let Some(parent) = record_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&record_path, input_text)?;
    let report = PostmortemRecordReport {
        schema_version: entry.schema_version.clone(),
        command: "jankurai postmortem record".to_string(),
        status: "complete".to_string(),
        repo: repo.display().to_string(),
        root: postmortem_root(&repo).display().to_string(),
        record_path: record_path.display().to_string(),
        record: entry,
        notes: vec!["postmortem record written only when explicitly requested".to_string()],
    };
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_record_markdown(&report))?;
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn run_list(args: PostmortemListArgs) -> Result<()> {
    let repo = canonicalize_repo(&args.repo)?;
    let root = resolve_repo_relative(&repo, &args.root)?;
    let records = collect_entries(&repo, &root)?;
    let report = PostmortemListReport {
        schema_version: "1.0.0".to_string(),
        command: "jankurai postmortem list".to_string(),
        status: "complete".to_string(),
        repo: repo.display().to_string(),
        root: root.display().to_string(),
        records_total: records.len(),
        records: records.into_iter().map(summary_from_entry).collect(),
        notes: vec![
            "list and show are read-only views over durable postmortem records".to_string(),
        ],
    };
    write_report(
        &report,
        args.out.as_deref(),
        args.md.as_deref(),
        &render_list_markdown(&report),
    )?;
    Ok(())
}

pub fn run_show(args: PostmortemShowArgs) -> Result<()> {
    let repo = canonicalize_repo(&args.repo)?;
    let root = resolve_repo_relative(&repo, &args.root)?;
    let record_path = root.join(format!("{}.toml", args.postmortem_id));
    let record = read_entry(&repo, &record_path)?;
    let report = PostmortemRecordReport {
        schema_version: record.schema_version.clone(),
        command: "jankurai postmortem show".to_string(),
        status: "complete".to_string(),
        repo: repo.display().to_string(),
        root: root.display().to_string(),
        record_path: record_path.display().to_string(),
        record,
        notes: vec!["show reads an existing durable record without mutating it".to_string()],
    };
    write_report(
        &report,
        args.out.as_deref(),
        args.md.as_deref(),
        &render_record_markdown(&report),
    )?;
    Ok(())
}

pub fn run_read(args: PostmortemReadArgs) -> Result<()> {
    let repo = canonicalize_repo(&args.repo)?;
    let record_path = resolve_repo_relative_existing(&repo, &args.path)?;
    let record = read_entry(&repo, &record_path)?;
    let report = PostmortemRecordReport {
        schema_version: record.schema_version.clone(),
        command: "jankurai postmortem read".to_string(),
        status: "complete".to_string(),
        repo: repo.display().to_string(),
        root: postmortem_root(&repo).display().to_string(),
        record_path: record_path.display().to_string(),
        record,
        notes: vec!["read inspects an arbitrary postmortem record without writing".to_string()],
    };
    write_report(
        &report,
        args.out.as_deref(),
        args.md.as_deref(),
        &render_record_markdown(&report),
    )?;
    Ok(())
}

fn write_report<T: Serialize>(
    report: &T,
    out: Option<&str>,
    md: Option<&str>,
    rendered_md: &str,
) -> Result<()> {
    if let Some(path) = out {
        crate::render::write_json(path, &serde_json::to_string_pretty(report)?)?;
    } else {
        println!("{}", serde_json::to_string_pretty(report)?);
    }
    if let Some(path) = md {
        crate::render::write_markdown(path, rendered_md)?;
    }
    Ok(())
}

fn render_record_markdown(report: &PostmortemRecordReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Postmortem");
    let _ = writeln!(out);
    let _ = writeln!(out, "- command: `{}`", report.command);
    let _ = writeln!(out, "- repo: `{}`", report.repo);
    let _ = writeln!(out, "- root: `{}`", report.root);
    let _ = writeln!(out, "- record path: `{}`", report.record_path);
    let record = &report.record;
    let _ = writeln!(out, "- id: `{}`", record.postmortem_id);
    let _ = writeln!(out, "- title: {}", record.title);
    let _ = writeln!(out, "- owner: `{}`", record.owner);
    let _ = writeln!(
        out,
        "- failure mode: `{}`",
        failure_mode_label(&record.failure_mode)
    );
    let _ = writeln!(out, "- severity: `{}`", record.severity);
    if let Some(blocker_type) = &record.blocker_type {
        let _ = writeln!(
            out,
            "- blocker type: `{}`",
            failure_mode_label(blocker_type)
        );
    }
    let _ = writeln!(out, "- summary: {}", record.summary);
    if !record.evidence.is_empty() {
        let _ = writeln!(out, "- evidence: `{}`", record.evidence.join(", "));
    }
    if !record.actions.is_empty() {
        let _ = writeln!(out, "- actions: `{}`", record.actions.join(", "));
    }
    if !record.notes.is_empty() {
        let _ = writeln!(out, "- notes: `{}`", record.notes.join(", "));
    }
    out
}

fn render_list_markdown(report: &PostmortemListReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Postmortem List");
    let _ = writeln!(out);
    let _ = writeln!(out, "- command: `{}`", report.command);
    let _ = writeln!(out, "- repo: `{}`", report.repo);
    let _ = writeln!(out, "- root: `{}`", report.root);
    let _ = writeln!(out, "- records: `{}`", report.records_total);
    let _ = writeln!(out);
    for record in &report.records {
        let _ = writeln!(
            out,
            "- `{}` `{}` `{}` [{}] - {}",
            record.postmortem_id,
            record.owner,
            failure_mode_label(&record.failure_mode),
            record.severity,
            record.title
        );
    }
    out
}

fn summary_from_entry(entry: PostmortemEntry) -> PostmortemEntrySummary {
    PostmortemEntrySummary {
        postmortem_id: entry.postmortem_id,
        title: entry.title,
        owner: entry.owner,
        failure_mode: entry.failure_mode,
        severity: entry.severity,
        blocker_type: entry.blocker_type,
        summary: entry.summary,
        source: entry.source,
    }
}

fn parse_entry(repo: &Path, text: &str) -> Result<PostmortemEntry> {
    let entry: PostmortemEntry = toml::from_str(text).context("parse postmortem TOML")?;
    validation::validate_serializable(repo, ArtifactSchema::Postmortem, &entry)?;
    if entry.postmortem_id.trim().is_empty() {
        bail!("postmortem_id must not be empty");
    }
    if entry.title.trim().is_empty() {
        bail!("title must not be empty");
    }
    if entry.owner.trim().is_empty() {
        bail!("owner must not be empty");
    }
    if entry.summary.trim().is_empty() {
        bail!("summary must not be empty");
    }
    Ok(entry)
}

fn read_entry(repo: &Path, path: &Path) -> Result<PostmortemEntry> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    parse_entry(repo, &text)
}

fn collect_entries(repo: &Path, root: &Path) -> Result<Vec<PostmortemEntry>> {
    let mut entries = Vec::new();
    if !root.exists() {
        return Ok(entries);
    }
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !entry.file_type().is_dir() || !is_skipped_dir(entry.path()))
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if entry.file_type().is_dir() || path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        entries.push(read_entry(repo, path)?);
    }
    entries.sort_by(|left, right| left.postmortem_id.cmp(&right.postmortem_id));
    Ok(entries)
}

fn postmortem_root(repo: &Path) -> PathBuf {
    repo.join(".jankurai/postmortems")
}

fn is_skipped_dir(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(part) => matches!(part.to_string_lossy().as_ref(), ".git" | "target"),
        _ => false,
    })
}

fn canonicalize_repo(repo: &Path) -> Result<PathBuf> {
    let canonical =
        fs::canonicalize(repo).with_context(|| format!("canonicalize {}", repo.display()))?;
    if !canonical.is_dir() {
        bail!("{} is not a directory", repo.display());
    }
    Ok(canonical)
}

fn resolve_repo_relative_existing(repo: &Path, rel: &str) -> Result<PathBuf> {
    let path = resolve_repo_relative(repo, rel)?;
    let canonical =
        fs::canonicalize(&path).with_context(|| format!("resolve {}", path.display()))?;
    if !canonical.starts_with(repo) {
        bail!("path escapes repo root: {}", path.display());
    }
    Ok(canonical)
}

fn resolve_repo_relative(repo: &Path, rel: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(rel);
    let abs = if candidate.is_absolute() {
        candidate
    } else {
        repo.join(candidate)
    };
    Ok(abs)
}

fn failure_mode_label(mode: &FailureMode) -> &'static str {
    match mode {
        FailureMode::AspirationalSpec => "aspirational-spec",
        FailureMode::EnvPrerequisite => "env-prerequisite",
        FailureMode::InteropRuntime => "interop-runtime",
        FailureMode::EquivalenceGap => "equivalence-gap",
        FailureMode::CutoverRollback => "cutover-rollback",
        FailureMode::PerfRegression => "perf-regression",
    }
}

pub fn parse_failure_mode(value: &str) -> Option<FailureMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "aspirational-spec" => Some(FailureMode::AspirationalSpec),
        "env-prerequisite" => Some(FailureMode::EnvPrerequisite),
        "interop-runtime" => Some(FailureMode::InteropRuntime),
        "equivalence-gap" => Some(FailureMode::EquivalenceGap),
        "cutover-rollback" => Some(FailureMode::CutoverRollback),
        "perf-regression" => Some(FailureMode::PerfRegression),
        _ => None,
    }
}
