use crate::model::Report;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const HISTORY_SCHEMA_VERSION: &str = "1.1.0";
const LEGACY_HISTORY_SCHEMA_VERSION: &str = "1.0.0";
const DEFAULT_HISTORY_MAX_ROWS: usize = 500;
const DEFAULT_HISTORY_MAX_BYTES: usize = 1_048_576;
const DEFAULT_MIRROR_MAX_ROWS: usize = 5_000;
const DEFAULT_HISTORY_MIRROR_ENV: &str = "JANKURAI_HISTORY_MIRROR";
const DEFAULT_DEDUPE_POLICY: &str = "consecutive-equivalent";
const HISTORY_LOCK_STALE_SECS: u64 = 30;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HistorySource {
    #[default]
    Auto,
    Local,
    Mirror,
}

impl HistorySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Local => "local",
            Self::Mirror => "mirror",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreHistoryEntry {
    pub schema_version: String,
    pub standard_version: String,
    pub auditor_version: String,
    pub generated_at: String,
    pub run_id: String,
    pub repo_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_remote: Option<String>,
    pub branch: String,
    pub commit: String,
    pub dirty_worktree: bool,
    pub scope: String,
    pub changed_paths: Vec<String>,
    pub score: i32,
    pub raw_score: i32,
    pub finding_count: usize,
    pub hard_findings: usize,
    pub soft_findings: usize,
    pub decision: String,
    pub minimum_score: i32,
    pub caps_applied: Vec<String>,
    pub report_fingerprint: String,
    pub input_fingerprint: String,
    pub policy_fingerprint: String,
    pub repo_score_json_path: String,
    pub repo_score_md_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreHistoryRow {
    #[serde(default = "default_history_schema_version")]
    pub schema_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auditor_version: Option<String>,
    pub generated_at: String,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_remote: Option<String>,
    pub branch: String,
    pub commit: String,
    pub dirty_worktree: bool,
    pub scope: String,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    pub score: i32,
    pub raw_score: i32,
    pub finding_count: usize,
    pub hard_findings: usize,
    pub soft_findings: usize,
    pub decision: String,
    pub minimum_score: i32,
    #[serde(default)]
    pub caps_applied: Vec<String>,
    pub report_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_fingerprint: Option<String>,
    pub repo_score_json_path: String,
    pub repo_score_md_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreHistorySummary {
    pub source: HistorySource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<String>,
    pub history_bytes: usize,
    pub sample_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_generated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_delta: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worst_score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_decision: Option<String>,
    pub high_or_critical_latest: usize,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreHistoryExport {
    pub schema_version: String,
    pub command: String,
    pub history: String,
    pub window: usize,
    pub source: HistorySource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<String>,
    pub history_bytes: usize,
    pub sample_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_generated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_commit: Option<String>,
    #[serde(default)]
    pub rows: Vec<ScoreHistoryRow>,
    pub summary: ScoreHistorySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreHistoryPolicy {
    pub max_rows: usize,
    pub max_bytes: usize,
    pub dedupe: String,
    pub mirror_env: String,
    pub mirror_required: bool,
    pub mirror_max_rows: usize,
}

impl Default for ScoreHistoryPolicy {
    fn default() -> Self {
        Self {
            max_rows: DEFAULT_HISTORY_MAX_ROWS,
            max_bytes: DEFAULT_HISTORY_MAX_BYTES,
            dedupe: DEFAULT_DEDUPE_POLICY.into(),
            mirror_env: DEFAULT_HISTORY_MIRROR_ENV.into(),
            mirror_required: false,
            mirror_max_rows: DEFAULT_MIRROR_MAX_ROWS,
        }
    }
}

impl ScoreHistoryPolicy {
    pub fn from_repo(repo: &Path) -> Self {
        #[derive(Debug, Deserialize)]
        struct AuditPolicyFile {
            #[serde(default)]
            history: Option<HistoryPolicyFile>,
        }

        #[derive(Debug, Deserialize)]
        struct HistoryPolicyFile {
            #[serde(default = "default_history_max_rows")]
            max_rows: usize,
            #[serde(default = "default_history_max_bytes")]
            max_bytes: usize,
            #[serde(default = "default_history_dedupe")]
            dedupe: String,
            #[serde(default = "default_history_mirror_env")]
            mirror_env: String,
            #[serde(default)]
            mirror_required: bool,
            #[serde(default = "default_mirror_max_rows")]
            mirror_max_rows: usize,
        }

        fn default_history_max_rows() -> usize {
            DEFAULT_HISTORY_MAX_ROWS
        }
        fn default_history_max_bytes() -> usize {
            DEFAULT_HISTORY_MAX_BYTES
        }
        fn default_history_dedupe() -> String {
            DEFAULT_DEDUPE_POLICY.into()
        }
        fn default_history_mirror_env() -> String {
            DEFAULT_HISTORY_MIRROR_ENV.into()
        }
        fn default_mirror_max_rows() -> usize {
            DEFAULT_MIRROR_MAX_ROWS
        }

        let path = repo.join("agent/audit-policy.toml");
        let parsed = fs::read_to_string(&path)
            .ok()
            .and_then(|text| toml::from_str::<AuditPolicyFile>(&text).ok())
            .and_then(|policy| policy.history)
            .map(|history| Self {
                max_rows: history.max_rows,
                max_bytes: history.max_bytes,
                dedupe: history.dedupe,
                mirror_env: history.mirror_env,
                mirror_required: history.mirror_required,
                mirror_max_rows: history.mirror_max_rows,
            });
        parsed.unwrap_or_default()
    }

    pub fn with_overrides(
        mut self,
        max_rows: Option<usize>,
        max_bytes: Option<usize>,
        mirror_required: Option<bool>,
    ) -> Self {
        if let Some(max_rows) = max_rows {
            self.max_rows = max_rows;
        }
        if let Some(max_bytes) = max_bytes {
            self.max_bytes = max_bytes;
        }
        if let Some(mirror_required) = mirror_required {
            self.mirror_required = mirror_required;
        }
        self
    }
}

#[derive(Debug, Clone)]
pub struct ScoreHistoryAppendOptions {
    pub history_path: String,
    pub csv_path: Option<String>,
    pub mirror_path: Option<String>,
    pub mirror_required: bool,
    pub policy: ScoreHistoryPolicy,
}

#[derive(Debug, Clone)]
pub struct RepoIdentity {
    pub repo_id: String,
    pub repo_remote: Option<String>,
}

impl From<ScoreHistoryEntry> for ScoreHistoryRow {
    fn from(value: ScoreHistoryEntry) -> Self {
        Self {
            schema_version: value.schema_version,
            standard_version: Some(value.standard_version),
            auditor_version: Some(value.auditor_version),
            generated_at: value.generated_at,
            run_id: value.run_id,
            repo_id: Some(value.repo_id),
            repo_remote: value.repo_remote,
            branch: value.branch,
            commit: value.commit,
            dirty_worktree: value.dirty_worktree,
            scope: value.scope,
            changed_paths: value.changed_paths,
            score: value.score,
            raw_score: value.raw_score,
            finding_count: value.finding_count,
            hard_findings: value.hard_findings,
            soft_findings: value.soft_findings,
            decision: value.decision,
            minimum_score: value.minimum_score,
            caps_applied: value.caps_applied,
            report_fingerprint: value.report_fingerprint,
            input_fingerprint: Some(value.input_fingerprint),
            policy_fingerprint: Some(value.policy_fingerprint),
            repo_score_json_path: value.repo_score_json_path,
            repo_score_md_path: value.repo_score_md_path,
        }
    }
}

impl ScoreHistoryRow {
    fn equivalent_to(&self, other: &Self) -> bool {
        self.repo_id == other.repo_id
            && self.commit == other.commit
            && self.dirty_worktree == other.dirty_worktree
            && self.scope == other.scope
            && self.changed_paths == other.changed_paths
            && self.score == other.score
            && self.raw_score == other.raw_score
            && self.finding_count == other.finding_count
            && self.hard_findings == other.hard_findings
            && self.soft_findings == other.soft_findings
            && self.decision == other.decision
            && self.minimum_score == other.minimum_score
            && self.caps_applied == other.caps_applied
    }
}

pub fn append_score_history(
    repo: &Path,
    report: &Report,
    json_path: &str,
    md_path: &str,
    history_path: &str,
    csv_path: Option<&str>,
) -> Result<PathBuf> {
    let options = ScoreHistoryAppendOptions {
        history_path: history_path.to_string(),
        csv_path: csv_path.map(|value| value.to_string()),
        mirror_path: None,
        mirror_required: false,
        policy: ScoreHistoryPolicy::from_repo(repo),
    };
    append_score_history_with_options(repo, report, json_path, md_path, options)?;
    Ok(resolve_output_path(repo, history_path))
}

pub fn append_score_history_with_options(
    repo: &Path,
    report: &Report,
    json_path: &str,
    md_path: &str,
    options: ScoreHistoryAppendOptions,
) -> Result<Option<PathBuf>> {
    let history_path = resolve_output_path(repo, &options.history_path);
    if let Some(parent) = history_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }

    let local_lock = lock_path_for_repo(repo);
    let Some(_guard) = acquire_lock(&local_lock, HISTORY_LOCK_STALE_SECS, "score history")? else {
        if options.mirror_required || options.policy.mirror_required {
            bail!("score history lock is busy and mirror-required mode is enabled");
        }
        eprintln!(
            "warning: score history lock is busy; skipping history append for {}",
            history_path.display()
        );
        return Ok(None);
    };

    let mut rows = match fs::read_to_string(&history_path) {
        Ok(text) => load_history_rows_text(&history_path, &text)?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(err) => {
            eprintln!(
                "warning: could not load {}: {err:#}; skipping history append",
                history_path.display()
            );
            return Ok(None);
        }
    };

    let identity = repo_identity(repo);
    let entry = build_entry(repo, report, json_path, md_path, &identity)?;
    let row = ScoreHistoryRow::from(entry);
    if !rows
        .last()
        .map(|last| last.equivalent_to(&row))
        .unwrap_or(false)
    {
        rows.push(row);
    }

    let rows = compact_history_rows(rows, options.policy.max_rows, options.policy.max_bytes)?;
    write_history_jsonl(&history_path, &rows)?;
    if let Some(csv_path) = options.csv_path.as_deref() {
        write_history_csv(repo, &rows, csv_path)?;
    }

    if let Some(mirror_path) = mirror_path_from_options(repo, &options) {
        if let Err(err) = append_mirror_history(
            &rows,
            &mirror_path,
            options.policy.mirror_max_rows,
            options.mirror_required || options.policy.mirror_required,
        ) {
            if options.mirror_required || options.policy.mirror_required {
                return Err(err);
            }
            eprintln!("warning: {err:#}");
        }
    }

    Ok(Some(history_path))
}

pub fn load_history_rows(history: &Path) -> Result<Vec<ScoreHistoryRow>> {
    let text =
        fs::read_to_string(history).with_context(|| format!("read {}", history.display()))?;
    load_history_rows_text(history, &text)
}

fn load_history_rows_text(history: &Path, text: &str) -> Result<Vec<ScoreHistoryRow>> {
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row: ScoreHistoryRow = serde_json::from_str(line).with_context(|| {
            format!(
                "parse score history line {} in {}",
                index + 1,
                history.display()
            )
        })?;
        rows.push(row);
    }
    Ok(rows)
}

pub fn build_history_export(
    history: &Path,
    window: usize,
    source: HistorySource,
) -> Result<ScoreHistoryExport> {
    let rows = load_history_rows(history)?;
    let history_bytes = fs::metadata(history).map(|m| m.len() as usize).unwrap_or(0);
    let selected = select_window(&rows, window);
    let summary = summarize_rows(
        &selected,
        history_bytes,
        source.clone(),
        latest_repo_id(&selected),
        latest_generated_at(&selected),
        latest_commit(&selected),
    );
    Ok(ScoreHistoryExport {
        schema_version: HISTORY_SCHEMA_VERSION.into(),
        command: "jankurai history export".into(),
        history: history.display().to_string(),
        window,
        source,
        repo_id: summary.repo_id.clone(),
        history_bytes,
        sample_count: selected.len(),
        latest_generated_at: summary.latest_generated_at.clone(),
        latest_commit: summary.latest_commit.clone(),
        rows: selected,
        summary,
    })
}

pub fn build_history_latest(history: &Path) -> Result<ScoreHistoryRow> {
    let rows = load_history_rows(history)?;
    rows.into_iter()
        .last()
        .with_context(|| format!("no score history rows found in {}", history.display()))
}

pub fn compact_history_file(
    history: &Path,
    max_rows: usize,
    max_bytes: usize,
) -> Result<Vec<ScoreHistoryRow>> {
    let rows = load_history_rows(history)?;
    let compacted = compact_history_rows(rows, max_rows, max_bytes)?;
    write_history_jsonl(history, &compacted)?;
    Ok(compacted)
}

pub fn restore_history_file(
    mirror: &Path,
    repo_id: &str,
    out: &Path,
    max_rows: usize,
    max_bytes: usize,
) -> Result<Vec<ScoreHistoryRow>> {
    let rows = load_history_rows(mirror)?;
    let filtered: Vec<ScoreHistoryRow> = rows
        .into_iter()
        .filter(|row| row.repo_id.as_deref() == Some(repo_id))
        .collect();
    if filtered.is_empty() {
        bail!(
            "no score history rows matched repo_id `{repo_id}` in {}",
            mirror.display()
        );
    }
    let compacted = compact_history_rows(filtered, max_rows, max_bytes)?;
    write_history_jsonl(out, &compacted)?;
    Ok(compacted)
}

pub fn build_trend_report(
    history: &Path,
    window: usize,
) -> Result<crate::commands::score::ScoreTrendReport> {
    let rows = load_history_rows(history)?;
    let history_bytes = fs::metadata(history).map(|m| m.len() as usize).unwrap_or(0);
    let selected = select_window(&rows, window);
    let summary = summarize_rows(
        &selected,
        history_bytes,
        HistorySource::Auto,
        latest_repo_id(&selected),
        latest_generated_at(&selected),
        latest_commit(&selected),
    );
    Ok(crate::commands::score::ScoreTrendReport {
        schema_version: HISTORY_SCHEMA_VERSION.into(),
        command: "jankurai score trend".into(),
        history: history.display().to_string(),
        window,
        source: HistorySource::Auto,
        repo_id: summary.repo_id.clone(),
        history_bytes,
        sample_count: summary.sample_count,
        first_score: summary.first_score,
        latest_score: summary.latest_score,
        score_delta: summary.score_delta,
        best_score: summary.best_score,
        worst_score: summary.worst_score,
        latest_decision: summary.latest_decision,
        latest_generated_at: summary.latest_generated_at,
        latest_commit: summary.latest_commit,
        high_or_critical_latest: summary.high_or_critical_latest,
        recurrence_counts: vec![],
        decision: summary.decision,
    })
}

pub fn build_entry(
    repo: &Path,
    report: &Report,
    json_path: &str,
    md_path: &str,
    identity: &RepoIdentity,
) -> Result<ScoreHistoryEntry> {
    let decision = report
        .decision
        .as_ref()
        .context("repo score report missing decision")?;
    let branch = git_output(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_else(|| "unknown".into());
    let commit = report
        .git
        .as_ref()
        .and_then(|git| git.head.clone())
        .or_else(|| git_output(repo, &["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".into());
    Ok(ScoreHistoryEntry {
        schema_version: HISTORY_SCHEMA_VERSION.into(),
        standard_version: report.standard_version.clone(),
        auditor_version: report.auditor_version.clone(),
        generated_at: report.generated_at.clone(),
        run_id: report
            .run_id
            .clone()
            .unwrap_or_else(|| report.generated_at.clone()),
        repo_id: identity.repo_id.clone(),
        repo_remote: identity.repo_remote.clone(),
        branch,
        commit,
        dirty_worktree: report.dirty_worktree,
        scope: report.scope.mode.clone(),
        changed_paths: report.scope.paths.clone(),
        score: report.score,
        raw_score: report.raw_score,
        finding_count: report.findings.len(),
        hard_findings: decision.hard_findings,
        soft_findings: decision.soft_findings,
        decision: decision.status.clone(),
        minimum_score: decision.minimum_score,
        caps_applied: report.caps_applied.clone(),
        report_fingerprint: report.report_fingerprint.clone(),
        input_fingerprint: report.input_fingerprint.clone(),
        policy_fingerprint: report.policy_fingerprint.clone(),
        repo_score_json_path: display_path(repo, json_path),
        repo_score_md_path: display_path(repo, md_path),
    })
}

pub fn repo_identity(repo: &Path) -> RepoIdentity {
    let repo_remote = git_output(repo, &["remote", "get-url", "origin"])
        .map(|remote| sanitize_remote_url(&remote));
    let repo_id = if let Some(remote) = repo_remote.as_ref() {
        hash_string(remote)
    } else {
        let canonical = repo
            .canonicalize()
            .unwrap_or_else(|_| repo.to_path_buf())
            .to_string_lossy()
            .replace('\\', "/");
        hash_string(&canonical)
    };
    RepoIdentity {
        repo_id,
        repo_remote,
    }
}

pub fn sanitize_remote_url(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://") {
        if let Some(at) = rest.find('@') {
            return format!("{scheme}://{}", &rest[at + 1..]);
        }
    }
    url.to_string()
}

pub fn history_mirror_path_from_env(policy: &ScoreHistoryPolicy) -> Option<String> {
    std::env::var(&policy.mirror_env)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn render_history_export_markdown(export: &ScoreHistoryExport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai History Export");
    let _ = writeln!(out);
    let _ = writeln!(out, "- History: `{}`", export.history);
    let _ = writeln!(out, "- Source: `{}`", export.summary.source.as_str());
    let _ = writeln!(out, "- Window: `{}`", export.window);
    let _ = writeln!(out, "- Samples: `{}`", export.sample_count);
    let _ = writeln!(out, "- History bytes: `{}`", export.history_bytes);
    let _ = writeln!(
        out,
        "- Repo ID: `{}`",
        export.summary.repo_id.as_deref().unwrap_or("none")
    );
    let _ = writeln!(
        out,
        "- Latest commit: `{}`",
        export.summary.latest_commit.as_deref().unwrap_or("none")
    );
    let _ = writeln!(
        out,
        "- Latest generated at: `{}`",
        export
            .summary
            .latest_generated_at
            .as_deref()
            .unwrap_or("none")
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "## Summary");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- First score: `{}`",
        opt_int(export.summary.first_score)
    );
    let _ = writeln!(
        out,
        "- Latest score: `{}`",
        opt_int(export.summary.latest_score)
    );
    let _ = writeln!(out, "- Delta: `{}`", opt_int(export.summary.score_delta));
    let _ = writeln!(
        out,
        "- Best score: `{}`",
        opt_int(export.summary.best_score)
    );
    let _ = writeln!(
        out,
        "- Worst score: `{}`",
        opt_int(export.summary.worst_score)
    );
    let _ = writeln!(
        out,
        "- Latest decision: `{}`",
        export.summary.latest_decision.as_deref().unwrap_or("none")
    );
    let _ = writeln!(
        out,
        "- High/critical latest: `{}`",
        export.summary.high_or_critical_latest
    );
    let _ = writeln!(out, "- Decision: `{}`", export.summary.decision);
    let _ = writeln!(out);
    let _ = writeln!(out, "## Rows");
    let _ = writeln!(out);
    for (index, row) in export.rows.iter().enumerate().take(10) {
        let _ = writeln!(
            out,
            "{}. `{}` `{}` score `{}` decision `{}`",
            index + 1,
            row.generated_at,
            row.commit,
            row.score,
            row.decision
        );
    }
    out
}

pub fn render_history_trend_markdown(report: &crate::commands::score::ScoreTrendReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Score Trend");
    let _ = writeln!(out);
    let _ = writeln!(out, "- History: `{}`", report.history);
    let _ = writeln!(out, "- Source: `{}`", report.source.as_str());
    let _ = writeln!(out, "- Window: `{}`", report.window);
    let _ = writeln!(out, "- Samples: `{}`", report.sample_count);
    let _ = writeln!(out, "- History bytes: `{}`", report.history_bytes);
    let _ = writeln!(
        out,
        "- Repo ID: `{}`",
        report.repo_id.as_deref().unwrap_or("none")
    );
    let _ = writeln!(
        out,
        "- Latest commit: `{}`",
        report.latest_commit.as_deref().unwrap_or("none")
    );
    let _ = writeln!(
        out,
        "- Latest generated at: `{}`",
        report.latest_generated_at.as_deref().unwrap_or("none")
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "## Summary");
    let _ = writeln!(out);
    let _ = writeln!(out, "- First score: `{}`", opt_int(report.first_score));
    let _ = writeln!(out, "- Latest score: `{}`", opt_int(report.latest_score));
    let _ = writeln!(out, "- Delta: `{}`", opt_int(report.score_delta));
    let _ = writeln!(out, "- Best score: `{}`", opt_int(report.best_score));
    let _ = writeln!(out, "- Worst score: `{}`", opt_int(report.worst_score));
    let _ = writeln!(
        out,
        "- Latest decision: `{}`",
        report.latest_decision.as_deref().unwrap_or("none")
    );
    let _ = writeln!(
        out,
        "- High/critical latest: `{}`",
        report.high_or_critical_latest
    );
    let _ = writeln!(out, "- Decision: `{}`", report.decision);
    let _ = writeln!(out);
    let _ = writeln!(out, "## Recurrence");
    let _ = writeln!(out);
    for item in report.recurrence_counts.iter().take(10) {
        let _ = writeln!(out, "- `{}`: `{}`", item.key, item.count);
    }
    out
}

fn summarize_rows(
    rows: &[ScoreHistoryRow],
    history_bytes: usize,
    source: HistorySource,
    repo_id: Option<String>,
    latest_generated_at: Option<String>,
    latest_commit: Option<String>,
) -> ScoreHistorySummary {
    let scores: Vec<i32> = rows.iter().map(|row| row.score).collect();
    let first_score = scores.first().copied();
    let latest_score = scores.last().copied();
    let score_delta = first_score
        .zip(latest_score)
        .map(|(first, latest)| latest - first);
    let latest_decision = rows.last().map(|row| row.decision.clone());
    let high_or_critical_latest = rows.last().map(|row| row.hard_findings).unwrap_or(0);
    let decision = if score_delta.unwrap_or(0) < 0 || high_or_critical_latest > 0 {
        "review"
    } else {
        "pass"
    };
    ScoreHistorySummary {
        source,
        repo_id,
        history_bytes,
        sample_count: rows.len(),
        latest_generated_at,
        latest_commit,
        first_score,
        latest_score,
        score_delta,
        best_score: scores.iter().max().copied(),
        worst_score: scores.iter().min().copied(),
        latest_decision,
        high_or_critical_latest,
        decision: decision.into(),
    }
}

fn select_window(rows: &[ScoreHistoryRow], window: usize) -> Vec<ScoreHistoryRow> {
    if window == 0 || window >= rows.len() {
        return rows.to_vec();
    }
    rows[rows.len() - window..].to_vec()
}

fn latest_commit(rows: &[ScoreHistoryRow]) -> Option<String> {
    rows.last().map(|row| row.commit.clone())
}

fn latest_generated_at(rows: &[ScoreHistoryRow]) -> Option<String> {
    rows.last().map(|row| row.generated_at.clone())
}

fn latest_repo_id(rows: &[ScoreHistoryRow]) -> Option<String> {
    rows.last().and_then(|row| row.repo_id.clone())
}

fn compact_history_rows(
    mut rows: Vec<ScoreHistoryRow>,
    max_rows: usize,
    max_bytes: usize,
) -> Result<Vec<ScoreHistoryRow>> {
    if max_rows == 0 {
        bail!("max_rows must be greater than zero");
    }
    rows.dedup_by(|left, right| left.equivalent_to(right));
    if rows.len() > max_rows {
        let start = rows.len() - max_rows;
        rows = rows.split_off(start);
    }
    if rows.is_empty() {
        bail!("compaction would remove all score history rows");
    }
    while rows.len() > 1 && score_history_bytes(&rows)? > max_bytes {
        rows.remove(0);
    }
    if score_history_bytes(&rows)? > max_bytes {
        bail!("a single score history row exceeds max_bytes");
    }
    Ok(rows)
}

fn score_history_bytes(rows: &[ScoreHistoryRow]) -> Result<usize> {
    let mut bytes = 0usize;
    for row in rows {
        bytes = bytes
            .checked_add(serde_json::to_string(row)?.len() + 1)
            .context("score history byte count overflow")?;
    }
    Ok(bytes)
}

fn write_history_jsonl(path: &Path, rows: &[ScoreHistoryRow]) -> Result<()> {
    if rows.is_empty() {
        bail!("refusing to write an empty score history");
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let text = rows
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");
    write_atomic(path, &(text + "\n"))
}

fn write_history_csv(repo: &Path, rows: &[ScoreHistoryRow], csv_path: &str) -> Result<()> {
    let csv_path = resolve_output_path(repo, csv_path);
    if let Some(parent) = csv_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut text = String::new();
    text.push_str(csv_header());
    text.push('\n');
    for (index, row) in rows.iter().enumerate() {
        text.push_str(&row_to_csv(index + 1, row));
        text.push('\n');
    }
    write_atomic(&csv_path, &text)
}

fn append_mirror_history(
    rows: &[ScoreHistoryRow],
    mirror_path: &Path,
    max_rows: usize,
    required: bool,
) -> Result<()> {
    if let Some(parent) = mirror_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let lock = mirror_path.with_extension("jsonl.lock");
    let Some(_guard) = acquire_lock(&lock, HISTORY_LOCK_STALE_SECS, "score history mirror")? else {
        if required {
            bail!("score history mirror lock is busy");
        }
        eprintln!(
            "warning: score history mirror lock is busy; skipping {}",
            mirror_path.display()
        );
        return Ok(());
    };

    let mut mirror_rows = match fs::read_to_string(mirror_path) {
        Ok(text) => load_history_rows_text(mirror_path, &text)?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(err) => {
            if required {
                return Err(err).with_context(|| format!("read {}", mirror_path.display()));
            }
            eprintln!(
                "warning: could not read {}: {err:#}; skipping mirror append",
                mirror_path.display()
            );
            return Ok(());
        }
    };
    if let Some(row) = rows.last() {
        if !mirror_rows
            .last()
            .map(|last| last.equivalent_to(row))
            .unwrap_or(false)
        {
            mirror_rows.push(row.clone());
        }
    }
    mirror_rows = compact_history_rows(mirror_rows, max_rows, usize::MAX)?;
    write_history_jsonl(mirror_path, &mirror_rows)?;
    Ok(())
}

fn mirror_path_from_options(repo: &Path, options: &ScoreHistoryAppendOptions) -> Option<PathBuf> {
    if let Some(path) = options.mirror_path.as_deref() {
        return Some(resolve_output_path(repo, path));
    }
    history_mirror_path_from_env(&options.policy).map(|path| resolve_output_path(repo, &path))
}

fn write_atomic(path: &Path, text: &str) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let suffix = format!(
        "{}.{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("score-history"),
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let tmp = path.with_file_name(suffix);
    {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&tmp)
            .with_context(|| format!("create {}", tmp.display()))?;
        file.write_all(text.as_bytes())
            .with_context(|| format!("write {}", tmp.display()))?;
        let _ = file.sync_all();
    }
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&tmp, path)
        .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

fn default_history_schema_version() -> String {
    LEGACY_HISTORY_SCHEMA_VERSION.into()
}

fn hash_string(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

fn csv_header() -> &'static str {
    "index,schema_version,standard_version,auditor_version,generated_at,run_id,repo_id,repo_remote,branch,commit,dirty_worktree,scope,changed_paths,score,raw_score,finding_count,hard_findings,soft_findings,decision,minimum_score,caps_applied,report_fingerprint,input_fingerprint,policy_fingerprint,repo_score_json_path,repo_score_md_path"
}

fn row_to_csv(index: usize, row: &ScoreHistoryRow) -> String {
    let cols = [
        index.to_string(),
        csv_escape(&row.schema_version),
        csv_escape(row.standard_version.as_deref().unwrap_or("")),
        csv_escape(row.auditor_version.as_deref().unwrap_or("")),
        csv_escape(&row.generated_at),
        csv_escape(&row.run_id),
        csv_escape(row.repo_id.as_deref().unwrap_or("")),
        csv_escape(row.repo_remote.as_deref().unwrap_or("")),
        csv_escape(&row.branch),
        csv_escape(&row.commit),
        csv_escape(&row.dirty_worktree.to_string()),
        csv_escape(&row.scope),
        csv_escape(&serde_json::to_string(&row.changed_paths).unwrap_or_else(|_| "[]".into())),
        csv_escape(&row.score.to_string()),
        csv_escape(&row.raw_score.to_string()),
        csv_escape(&row.finding_count.to_string()),
        csv_escape(&row.hard_findings.to_string()),
        csv_escape(&row.soft_findings.to_string()),
        csv_escape(&row.decision),
        csv_escape(&row.minimum_score.to_string()),
        csv_escape(&serde_json::to_string(&row.caps_applied).unwrap_or_else(|_| "[]".into())),
        csv_escape(&row.report_fingerprint),
        csv_escape(row.input_fingerprint.as_deref().unwrap_or("")),
        csv_escape(row.policy_fingerprint.as_deref().unwrap_or("")),
        csv_escape(&row.repo_score_json_path),
        csv_escape(&row.repo_score_md_path),
    ];
    cols.join(",")
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn resolve_output_path(repo: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.join(path)
    }
}

fn display_path(repo: &Path, path: &str) -> String {
    let path = resolve_output_path(repo, path);
    path.strip_prefix(repo)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn lock_path_for_repo(repo: &Path) -> PathBuf {
    let git_dir = git_output(repo, &["rev-parse", "--git-dir"])
        .map(|value| {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                path
            } else {
                repo.join(path)
            }
        })
        .unwrap_or_else(|| repo.join(".git"));
    git_dir.join("jankurai").join("score-history.lock")
}

struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn acquire_lock(path: &Path, stale_secs: u64, label: &str) -> Result<Option<LockGuard>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    if path.exists() {
        let stale = fs::metadata(path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.elapsed().ok())
            .map(|elapsed| elapsed > Duration::from_secs(stale_secs))
            .unwrap_or(false);
        if stale {
            let _ = fs::remove_file(path);
        } else {
            return Ok(None);
        }
    }
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            let _ = writeln!(file, "pid={}", std::process::id());
            Ok(Some(LockGuard {
                path: path.to_path_buf(),
            }))
        }
        Err(err) => {
            if err.kind() == std::io::ErrorKind::AlreadyExists {
                Ok(None)
            } else {
                Err(err).with_context(|| format!("acquire {label} lock {}", path.display()))
            }
        }
    }
}

fn opt_int(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".into())
}
