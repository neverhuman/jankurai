use crate::commands::release_data::load_release_data;
use crate::commands::repair::now_string;
use crate::validation::{self, ArtifactSchema};
use anyhow::{anyhow, Result};
use chrono::{NaiveDate, Utc};
use ignore::WalkBuilder;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ExceptionExpireArgs {
    pub repo: PathBuf,
    pub warning_days: i64,
    pub strict: bool,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExceptionExpiryReport {
    pub schema_version: String,
    pub repo: String,
    pub generated_at: String,
    pub status: String,
    pub exception_root: String,
    pub warning_days: i64,
    pub total_exceptions: usize,
    pub expired_count: usize,
    pub expiring_soon_count: usize,
    pub invalid_count: usize,
    pub exceptions: Vec<ExceptionEntry>,
    pub proof_requirements: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExceptionEntry {
    pub path: String,
    pub code: String,
    pub owner: String,
    pub reason: String,
    pub expires: String,
    pub migration_plan: String,
    pub proof_lane: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_until_expiry: Option<i64>,
    pub repair_options: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ExceptionFrontMatter {
    code: Option<String>,
    owner: Option<String>,
    reason: Option<String>,
    expires: Option<String>,
    migration_plan: Option<String>,
    proof_lane: Option<String>,
    repair_guidance: Option<String>,
}

pub fn run_expire(args: ExceptionExpireArgs) -> Result<()> {
    let report = build_report(&args.repo, args.warning_days)?;
    if let Some(path) = args.out.as_deref() {
        validation::write_json(
            &args.repo,
            ArtifactSchema::ExceptionExpiryReport,
            path,
            &report,
        )?;
    } else {
        validation::validate_serializable(
            &args.repo,
            ArtifactSchema::ExceptionExpiryReport,
            &report,
        )?;
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&report))?;
    }
    if args.strict && report.status == "blocked" {
        return Err(anyhow!(
            "exception expiry is blocked (expired_count={} invalid_count={}); fix or renew exceptions or omit --strict for advisory-only runs",
            report.expired_count,
            report.invalid_count
        ));
    }
    Ok(())
}

pub fn build_report(repo: &Path, warning_days: i64) -> Result<ExceptionExpiryReport> {
    let release = load_release_data(repo)?;
    let root = repo.join("docs/exceptions");
    let mut exceptions = Vec::new();
    let mut expired_count = 0usize;
    let mut expiring_soon_count = 0usize;
    let mut invalid_count = 0usize;
    let today = Utc::now().date_naive();
    let files = collect_exception_files(&root)?;

    for path in &files {
        let entry = match parse_exception_file(path) {
            Ok(front_matter) => classify_exception(repo, path, front_matter, today, warning_days),
            Err(error) => {
                invalid_count += 1;
                ExceptionEntry {
                    path: repo_relative_path(repo, path),
                    code: "invalid-exception".to_string(),
                    owner: String::new(),
                    reason: String::new(),
                    expires: String::new(),
                    migration_plan: String::new(),
                    proof_lane: String::new(),
                    status: "invalid".to_string(),
                    days_until_expiry: None,
                    repair_options: vec![
                        "add YAML front matter with code, owner, reason, expires, migration_plan, and proof_lane".to_string(),
                    ],
                    notes: vec![error],
                }
            }
        };
        if entry.status == "expired" {
            expired_count += 1;
        } else if entry.status == "expiring-soon" {
            expiring_soon_count += 1;
        }
        exceptions.push(entry);
    }

    let status = if expired_count > 0 || invalid_count > 0 {
        "blocked"
    } else {
        "complete"
    };
    let mut proof_requirements = BTreeSet::new();
    proof_requirements.insert("docs/exceptions/*.md must carry code, owner, reason, expires, migration_plan, and proof_lane front matter".to_string());
    proof_requirements.insert(
        "keep exception proof lanes executable and route renewal or removal work through the owner"
            .to_string(),
    );
    let mut notes = vec![
        "exception expiry is advisory and does not mutate files; use `--strict` to exit non-zero when status is blocked (expired or invalid)".to_string(),
        format!("governance timebox: {} days", warning_days),
    ];
    if files.is_empty() {
        notes.push("no exception files were found under docs/exceptions".to_string());
    }
    if expired_count > 0 {
        notes.push("expired exceptions must be renewed, removed, or escalated".to_string());
    }
    if invalid_count > 0 {
        notes.push(
            "invalid exception files need front matter repair before they can be trusted"
                .to_string(),
        );
    }
    Ok(ExceptionExpiryReport {
        schema_version: release.schema_version,
        repo: repo.display().to_string(),
        generated_at: now_string(),
        status: status.to_string(),
        exception_root: "docs/exceptions".to_string(),
        warning_days,
        total_exceptions: exceptions.len(),
        expired_count,
        expiring_soon_count,
        invalid_count,
        exceptions,
        proof_requirements: proof_requirements.into_iter().collect(),
        notes,
    })
}

fn classify_exception(
    repo: &Path,
    path: &Path,
    front_matter: ExceptionFrontMatter,
    today: NaiveDate,
    warning_days: i64,
) -> ExceptionEntry {
    let code = front_matter.code.unwrap_or_default();
    let owner = front_matter.owner.unwrap_or_default();
    let reason = front_matter.reason.unwrap_or_default();
    let expires = front_matter.expires.unwrap_or_default();
    let migration_plan = front_matter.migration_plan.unwrap_or_default();
    let proof_lane = front_matter.proof_lane.unwrap_or_default();
    let mut notes = Vec::new();
    if let Some(guidance) = front_matter.repair_guidance {
        notes.push(guidance);
    }

    let (status, days_until_expiry) = match NaiveDate::parse_from_str(&expires, "%Y-%m-%d") {
        Ok(expiry) if expiry < today => (
            "expired".to_string(),
            Some(expiry.signed_duration_since(today).num_days()),
        ),
        Ok(expiry) => {
            let days = expiry.signed_duration_since(today).num_days();
            if days <= warning_days {
                ("expiring-soon".to_string(), Some(days))
            } else {
                ("current".to_string(), Some(days))
            }
        }
        Err(error) => {
            notes.push(format!("invalid expires date: {error}"));
            ("invalid".to_string(), None)
        }
    };
    let repair_options = match status.as_str() {
        "expired" => vec![
            "renew with owner and justification".to_string(),
            "remove the exception by fixing the violation".to_string(),
            "escalate to a human review if the exception is still necessary".to_string(),
        ],
        "expiring-soon" => vec![
            "renew before the expiry date with proof and owner signoff".to_string(),
            "fix the violation and delete the exception record".to_string(),
        ],
        "current" => vec!["keep the owner, proof lane, and migration path current".to_string()],
        _ => vec![
            "add YAML front matter with code, owner, reason, expires, migration_plan, and proof_lane".to_string(),
        ],
    };
    ExceptionEntry {
        path: repo_relative_path(repo, path),
        code,
        owner,
        reason,
        expires,
        migration_plan,
        proof_lane,
        status,
        days_until_expiry,
        repair_options,
        notes,
    }
}

fn parse_exception_file(path: &Path) -> Result<ExceptionFrontMatter, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("read {}: {}", path.display(), error))?;
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Err("missing YAML front matter".to_string());
    }
    let mut front_matter = String::new();
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        front_matter.push_str(line);
        front_matter.push('\n');
    }
    if !closed {
        return Err("missing closing YAML front matter delimiter".to_string());
    }
    serde_yaml::from_str::<ExceptionFrontMatter>(&front_matter)
        .map_err(|error| format!("parse YAML front matter in {}: {}", path.display(), error))
}

fn collect_exception_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in WalkBuilder::new(root).hidden(true).build() {
        let entry = entry?;
        let path = entry.into_path();
        if should_skip_path(&path) {
            continue;
        }
        if is_exception_doc(&path) && path.is_file() {
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

fn is_exception_doc(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if name == "README.md" {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return false;
    };
    let mut chars = stem.chars();
    let digits: String = chars.by_ref().take(4).collect();
    digits.len() == 4
        && digits.chars().all(|ch| ch.is_ascii_digit())
        && matches!(chars.next(), Some('-'))
        && path.extension().and_then(|ext| ext.to_str()) == Some("md")
}

pub(crate) fn repo_relative_path(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .ok()
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn render_markdown(report: &ExceptionExpiryReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Exception Expiry");
    let _ = writeln!(out);
    let _ = writeln!(out, "- repo: `{}`", report.repo);
    let _ = writeln!(out, "- status: `{}`", report.status);
    let _ = writeln!(out, "- warning days: `{}`", report.warning_days);
    let _ = writeln!(
        out,
        "- counts: total=`{}` expired=`{}` expiring-soon=`{}` invalid=`{}`",
        report.total_exceptions,
        report.expired_count,
        report.expiring_soon_count,
        report.invalid_count
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "## Exceptions");
    let _ = writeln!(out);
    for entry in &report.exceptions {
        let _ = writeln!(
            out,
            "- `{}` `{}` -> `{}`",
            entry.path, entry.code, entry.status
        );
        let _ = writeln!(out, "  owner: `{}`", entry.owner);
        let _ = writeln!(out, "  expires: `{}`", entry.expires);
        let _ = writeln!(out, "  proof lane: `{}`", entry.proof_lane);
        let _ = writeln!(
            out,
            "  repair options: `{}`",
            entry.repair_options.join(", ")
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Proof");
    let _ = writeln!(out);
    for proof in &report.proof_requirements {
        let _ = writeln!(out, "- {}", proof);
    }
    out
}
