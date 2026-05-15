use crate::audit::{fs, scan};
use crate::model::FileInfo;
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs as stdfs;
use std::path::Path;

pub const COPY_CODE_SCHEMA_VERSION: &str = "1.1.0";
pub const DEFAULT_MIN_LINES: usize = 10;
pub const DEFAULT_MIN_TOKENS: usize = 100;
pub const DEFAULT_MAX_FINDINGS: usize = 50;

pub const DEFAULT_JSON_PATH: &str = "target/jankurai/copy-code.json";
pub const DEFAULT_MD_PATH: &str = "target/jankurai/copy-code.md";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCodeOptions {
    pub min_lines: usize,
    pub min_tokens: usize,
    pub max_findings: usize,
    pub include_tests: bool,
    pub strict: bool,
}

impl Default for CopyCodeOptions {
    fn default() -> Self {
        Self {
            min_lines: DEFAULT_MIN_LINES,
            min_tokens: DEFAULT_MIN_TOKENS,
            max_findings: DEFAULT_MAX_FINDINGS,
            include_tests: false,
            strict: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCodePolicy {
    pub active_source_only: bool,
    pub include_tests: bool,
    pub min_lines: usize,
    pub min_tokens: usize,
    pub max_findings: usize,
    pub strict: bool,
    pub excluded_roots: Vec<String>,
    pub warning_only_roots: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CopyCodeSummary {
    pub files_scanned: usize,
    pub files_considered: usize,
    pub active_files: usize,
    pub exact_file_classes: usize,
    pub hard_classes: usize,
    pub warning_classes: usize,
    pub hard_instances: usize,
    pub warning_instances: usize,
    pub duplicate_lines: usize,
    pub duplicate_tokens: usize,
    pub duplicate_bytes: usize,
    pub total_redundant_lines: usize,
    pub total_redundant_tokens: usize,
    pub total_redundant_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCodeInstance {
    pub path: String,
    pub language: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyCodeKind {
    ExactFile,
    ExactUnitSameName,
    ExactUnitDifferentName,
    TokenBlock,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyCodeSeverity {
    Hard,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressedInfo {
    pub by: String,
    pub owner: Option<String>,
    pub reason: Option<String>,
    pub expires: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCodeClass {
    pub id: String,
    pub kind: CopyCodeKind,
    pub severity: CopyCodeSeverity,
    pub confidence: String,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_name: Option<String>,
    pub duplicate_lines: usize,
    pub duplicate_tokens: usize,
    pub duplicate_bytes: usize,
    pub instance_count: usize,
    pub total_redundant_lines: usize,
    pub total_redundant_tokens: usize,
    pub total_redundant_bytes: usize,
    pub effective_severity: CopyCodeSeverity,
    pub hard_fail: bool,
    pub fingerprint: String,
    pub reason: String,
    pub recommended_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppressed: Option<SuppressedInfo>,
    pub instances: Vec<CopyCodeInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCodeReport {
    pub schema_version: String,
    pub generated_by: String,
    pub generated_at: String,
    pub repo: String,
    pub auditor_version: String,
    pub status: String,
    pub policy: CopyCodePolicy,
    pub summary: CopyCodeSummary,
    pub classes: Vec<CopyCodeClass>,
    pub notes: Vec<String>,
}

impl CopyCodeReport {
    pub fn empty() -> Self {
        Self {
            schema_version: COPY_CODE_SCHEMA_VERSION.into(),
            generated_by: "jankurai copy-code".into(),
            generated_at: String::new(),
            repo: String::new(),
            auditor_version: crate::model::AUDITOR_VERSION.into(),
            status: "skipped".into(),
            policy: CopyCodePolicy {
                active_source_only: true,
                include_tests: false,
                min_lines: DEFAULT_MIN_LINES,
                min_tokens: DEFAULT_MIN_TOKENS,
                max_findings: DEFAULT_MAX_FINDINGS,
                strict: false,
                excluded_roots: vec![],
                warning_only_roots: vec![],
            },
            summary: CopyCodeSummary::default(),
            classes: vec![],
            notes: vec![],
        }
    }
}

#[derive(Debug, Clone)]
struct CandidateFile {
    file: FileInfo,
    language: String,
    path_kind: PathKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathKind {
    Active,
    WarningOnly,
}

#[derive(Debug, Clone)]
struct UnitCandidate {
    path: String,
    language: String,
    name: String,
    path_kind: PathKind,
    start_line: usize,
    end_line: usize,
    signature_text: String,
    body_text: String,
    token_count: usize,
    byte_count: usize,
}

#[derive(Debug, Clone)]
struct WindowCandidate {
    path: String,
    language: String,
    start_line: usize,
    end_line: usize,
    text: String,
    token_count: usize,
    byte_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct AllowlistEntry {
    fingerprint: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    expires: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct AllowlistFile {
    #[serde(default)]
    entries: Vec<AllowlistEntry>,
}

fn load_allowlist(repo: &Path) -> Vec<AllowlistEntry> {
    let path = repo.join("agent/copy-code-allowlist.toml");
    let Ok(text) = stdfs::read_to_string(&path) else {
        return Vec::new();
    };
    let file: AllowlistFile = toml::from_str(&text).unwrap_or_default();
    file.entries
}

fn allowlist_match<'a>(
    entries: &'a [AllowlistEntry],
    fingerprint: &str,
    today: &str,
) -> Option<&'a AllowlistEntry> {
    entries.iter().find(|e| {
        if e.fingerprint != fingerprint {
            return false;
        }
        if let Some(exp) = &e.expires {
            if exp.as_str() < today {
                return false;
            }
        }
        true
    })
}

pub fn scan_repo(repo: &Path, options: CopyCodeOptions) -> Result<CopyCodeReport> {
    let inventory = fs::inventory_repo_detailed(repo, &fs::InventoryOptions::from_policy(repo))?;
    Ok(scan_files(repo, &inventory.files, options))
}

pub fn scan_files(repo: &Path, files: &[FileInfo], options: CopyCodeOptions) -> CopyCodeReport {
    let policy = CopyCodePolicy {
        active_source_only: true,
        include_tests: options.include_tests,
        min_lines: options.min_lines,
        min_tokens: options.min_tokens,
        max_findings: options.max_findings,
        strict: options.strict,
        excluded_roots: excluded_roots(),
        warning_only_roots: warning_only_roots(),
    };

    let candidates: Vec<CandidateFile> = files
        .iter()
        .filter_map(|file| classify_file(file, options.include_tests))
        .collect();
    let considered = candidates.len();
    let active_files = candidates
        .iter()
        .filter(|candidate| candidate.path_kind == PathKind::Active)
        .count();

    let mut classes = Vec::new();
    let exact_file_classes = build_exact_file_classes(&candidates);
    let covered_paths: HashSet<String> = exact_file_classes
        .iter()
        .flat_map(|class| class.instances.iter().map(|instance| instance.path.clone()))
        .collect();
    let non_file_candidates: Vec<_> = candidates
        .iter()
        .filter(|candidate| !covered_paths.contains(&candidate.file.rel_path))
        .cloned()
        .collect();
    classes.extend(exact_file_classes);
    classes.extend(build_exact_unit_classes(
        &non_file_candidates,
        options.min_lines,
        options.min_tokens,
    ));
    classes.extend(build_warning_body_classes(&non_file_candidates));
    classes.extend(build_token_block_classes(
        &non_file_candidates,
        options.min_lines,
        options.min_tokens,
    ));

    let allowlist = load_allowlist(repo);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    for class in classes.iter_mut() {
        if let Some(entry) = allowlist_match(&allowlist, &class.fingerprint, &today) {
            class.suppressed = Some(SuppressedInfo {
                by: "allowlist".to_string(),
                owner: entry.owner.clone(),
                reason: entry.reason.clone(),
                expires: entry.expires.clone(),
            });
            class.effective_severity = CopyCodeSeverity::Warning;
            class.hard_fail = false;
        }
    }

    classes.sort_by(|a, b| {
        severity_rank(&b.effective_severity)
            .cmp(&severity_rank(&a.effective_severity))
            .then_with(|| b.total_redundant_lines.cmp(&a.total_redundant_lines))
            .then_with(|| b.total_redundant_tokens.cmp(&a.total_redundant_tokens))
            .then_with(|| b.total_redundant_bytes.cmp(&a.total_redundant_bytes))
            .then_with(|| b.instance_count.cmp(&a.instance_count))
            .then_with(|| a.id.cmp(&b.id))
    });

    let total_hard_instances = classes
        .iter()
        .filter(|class| class.hard_fail)
        .map(|class| class.instances.len())
        .sum();
    let total_warning_instances = classes
        .iter()
        .filter(|class| !class.hard_fail)
        .map(|class| class.instances.len())
        .sum();
    let summary = CopyCodeSummary {
        files_scanned: files.len(),
        files_considered: considered,
        active_files,
        exact_file_classes: classes
            .iter()
            .filter(|class| matches!(class.kind, CopyCodeKind::ExactFile))
            .count(),
        hard_classes: classes.iter().filter(|class| class.hard_fail).count(),
        warning_classes: classes.iter().filter(|class| !class.hard_fail).count(),
        hard_instances: total_hard_instances,
        warning_instances: total_warning_instances,
        duplicate_lines: classes.iter().map(|class| class.duplicate_lines).sum(),
        duplicate_tokens: classes.iter().map(|class| class.duplicate_tokens).sum(),
        duplicate_bytes: classes.iter().map(|class| class.duplicate_bytes).sum(),
        total_redundant_lines: classes
            .iter()
            .map(|class| class.total_redundant_lines)
            .sum(),
        total_redundant_tokens: classes
            .iter()
            .map(|class| class.total_redundant_tokens)
            .sum(),
        total_redundant_bytes: classes
            .iter()
            .map(|class| class.total_redundant_bytes)
            .sum(),
    };

    let mut notes = vec![
        "hard classes are limited to exact active-source file matches and substantial exact same-name units"
            .into(),
        "warning classes include same-body different-name units and token/block duplication".into(),
    ];
    if !options.include_tests {
        notes.push(
            "tests, fixtures, stories, config, Docker, and migrations are omitted unless --include-tests is set".into(),
        );
    }

    let truncated = classes.len().saturating_sub(options.max_findings);
    if truncated > 0 {
        notes.push(format!(
            "showing the top {} classes and omitting {} lower-ranked classes",
            options.max_findings, truncated
        ));
    }

    let status = if options.strict && summary.hard_classes > 0 {
        "fail"
    } else if summary.hard_classes > 0 || summary.warning_classes > 0 {
        "review"
    } else {
        "pass"
    };

    CopyCodeReport {
        schema_version: COPY_CODE_SCHEMA_VERSION.into(),
        generated_by: "jankurai copy-code".into(),
        generated_at: now_string(),
        repo: repo.display().to_string(),
        auditor_version: crate::model::AUDITOR_VERSION.into(),
        status: status.into(),
        policy,
        summary,
        classes: classes.into_iter().take(options.max_findings).collect(),
        notes,
    }
}

pub fn render_markdown(report: &CopyCodeReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Copy-Code Redundancy");
    let _ = writeln!(out);
    let _ = writeln!(out, "- status: `{}`", report.status);
    let _ = writeln!(out, "- repo: `{}`", report.repo);
    let _ = writeln!(out, "- generated by: `{}`", report.generated_by);
    let _ = writeln!(out, "- generated at: `{}`", report.generated_at);
    let _ = writeln!(out, "- auditor: `{}`", report.auditor_version);
    let _ = writeln!(
        out,
        "- policy: min-lines=`{}` min-tokens=`{}` max-findings=`{}` include-tests=`{}` strict=`{}`",
        report.policy.min_lines,
        report.policy.min_tokens,
        report.policy.max_findings,
        report.policy.include_tests,
        report.policy.strict
    );
    let _ = writeln!(
        out,
        "- files: scanned=`{}` considered=`{}` active=`{}`",
        report.summary.files_scanned, report.summary.files_considered, report.summary.active_files
    );
    let _ = writeln!(
        out,
        "- classes: hard=`{}` warning=`{}` exact-file=`{}`",
        report.summary.hard_classes,
        report.summary.warning_classes,
        report.summary.exact_file_classes
    );
    let _ = writeln!(
        out,
        "- duplicate volume: lines=`{}` tokens=`{}` bytes=`{}`",
        report.summary.duplicate_lines,
        report.summary.duplicate_tokens,
        report.summary.duplicate_bytes
    );
    if !report.notes.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Notes");
        for note in &report.notes {
            let _ = writeln!(out, "- {note}");
        }
    }
    // Top-10 by total redundant lines.
    let top10: Vec<_> = {
        let mut sorted = report.classes.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|c| std::cmp::Reverse(c.total_redundant_lines));
        sorted.into_iter().take(10).collect()
    };
    if !top10.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Top Classes by Redundant Volume");
        let _ = writeln!(
            out,
            "| # | Kind | Lang | Instances | Redundant Lines | Hard? | ID |"
        );
        let _ = writeln!(out, "| --- | --- | --- | ---: | ---: | --- | --- |");
        for (i, class) in top10.iter().enumerate() {
            let sev = if class.hard_fail { "YES" } else { "no" };
            let _ = writeln!(
                out,
                "| {} | `{}` | `{}` | {} | {} | {} | `{}` |",
                i + 1,
                kind_label(&class.kind),
                class.language,
                class.instance_count,
                class.total_redundant_lines,
                sev,
                class.id
            );
        }
    }
    let hard: Vec<_> = report
        .classes
        .iter()
        .filter(|class| class.hard_fail)
        .collect();
    let warnings: Vec<_> = report
        .classes
        .iter()
        .filter(|class| !class.hard_fail)
        .collect();

    if !hard.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Hard Classes");
        render_class_table(&mut out, &hard);
    }
    if !warnings.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Advisory Classes");
        render_class_table(&mut out, &warnings);
    }
    out
}

fn render_class_table(out: &mut String, classes: &[&CopyCodeClass]) {
    use std::fmt::Write;
    let _ = writeln!(
        out,
        "| Kind | Language | Lines | Tokens | Bytes | Instances | Reason |"
    );
    let _ = writeln!(out, "| --- | --- | ---: | ---: | ---: | ---: | --- |");
    for class in classes {
        let instances = class
            .instances
            .iter()
            .map(|instance| {
                format!(
                    "{}:{}-{}",
                    instance.path, instance.start_line, instance.end_line
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "| `{}` | `{}` | {} | {} | {} | {} | {} |",
            kind_label(&class.kind),
            class.language,
            class.duplicate_lines,
            class.duplicate_tokens,
            class.duplicate_bytes,
            instances,
            class.reason
        );
    }
}

fn build_exact_file_classes(candidates: &[CandidateFile]) -> Vec<CopyCodeClass> {
    let mut groups: HashMap<(String, String), Vec<&CandidateFile>> = HashMap::new();
    for candidate in candidates {
        let normalized = normalize_exact_text(&candidate.file.text);
        let hash = sha256(&normalized);
        groups
            .entry((candidate.language.clone(), hash))
            .or_default()
            .push(candidate);
    }

    let mut classes = Vec::new();
    for ((language, hash), files) in groups {
        if files.len() < 2 {
            continue;
        }
        let severity = if files.iter().all(|file| file.path_kind == PathKind::Active) {
            CopyCodeSeverity::Hard
        } else {
            CopyCodeSeverity::Warning
        };
        let instances = files
            .iter()
            .map(|file| CopyCodeInstance {
                path: file.file.rel_path.clone(),
                language: file.language.clone(),
                start_line: 1,
                end_line: file.file.line_count.max(1),
                unit_name: None,
            })
            .collect::<Vec<_>>();
        let sample = files[0];
        let dup_lines = sample.file.line_count.max(1);
        let dup_tokens = count_tokens(&normalize_exact_text(&sample.file.text));
        let dup_bytes = sample.file.text.len();
        let n = instances.len();
        let effective = effective_severity_for(&CopyCodeKind::ExactFile, &severity);
        let fp = class_fingerprint(&CopyCodeKind::ExactFile, &language, &instances, dup_lines);
        classes.push(CopyCodeClass {
            id: class_id("file", &language, &hash),
            kind: CopyCodeKind::ExactFile,
            severity,
            confidence: "high".into(),
            language,
            unit_name: None,
            duplicate_lines: dup_lines,
            duplicate_tokens: dup_tokens,
            duplicate_bytes: dup_bytes,
            instance_count: n,
            total_redundant_lines: redundant_volume(dup_lines, n),
            total_redundant_tokens: redundant_volume(dup_tokens, n),
            total_redundant_bytes: redundant_volume(dup_bytes, n),
            effective_severity: effective.clone(),
            hard_fail: matches!(effective, CopyCodeSeverity::Hard),
            fingerprint: fp,
            reason: "exact normalized source file copy".into(),
            recommended_action: "keep one owner for the copied file or extract the shared behavior into a single module".into(),
            suppressed: None,
            instances,
        });
    }
    classes
}

fn build_exact_unit_classes(
    candidates: &[CandidateFile],
    min_lines: usize,
    min_tokens: usize,
) -> Vec<CopyCodeClass> {
    let mut groups: HashMap<(String, String, String), Vec<UnitCandidate>> = HashMap::new();
    for candidate in candidates {
        for unit in extract_units(&candidate.file, &candidate.language, candidate.path_kind) {
            groups
                .entry((
                    candidate.language.clone(),
                    unit.name.clone(),
                    unit.signature_hash(),
                ))
                .or_default()
                .push(unit);
        }
    }

    let mut classes = Vec::new();
    for ((language, unit_name, signature_hash), units) in groups {
        if units.len() < 2 {
            continue;
        }
        let sample = &units[0];
        let severity = if units.iter().all(|unit| unit.path_kind == PathKind::Active)
            && sample.body_line_count() >= min_lines
            && sample.token_count >= min_tokens
        {
            CopyCodeSeverity::Hard
        } else {
            CopyCodeSeverity::Warning
        };
        let instances = units
            .iter()
            .map(|unit| CopyCodeInstance {
                path: unit.path.clone(),
                language: unit.language.clone(),
                start_line: unit.start_line,
                end_line: unit.end_line,
                unit_name: Some(unit.name.clone()),
            })
            .collect::<Vec<_>>();
        let dup_lines = sample.body_line_count();
        let dup_tokens = sample.token_count;
        let dup_bytes = sample.byte_count;
        let n = instances.len();
        let effective = effective_severity_for(&CopyCodeKind::ExactUnitSameName, &severity);
        let fp = class_fingerprint(
            &CopyCodeKind::ExactUnitSameName,
            &language,
            &instances,
            dup_lines,
        );
        classes.push(CopyCodeClass {
            id: class_id("unit", &language, &signature_hash),
            kind: CopyCodeKind::ExactUnitSameName,
            severity,
            confidence: "high".into(),
            language,
            unit_name: Some(unit_name),
            duplicate_lines: dup_lines,
            duplicate_tokens: dup_tokens,
            duplicate_bytes: dup_bytes,
            instance_count: n,
            total_redundant_lines: redundant_volume(dup_lines, n),
            total_redundant_tokens: redundant_volume(dup_tokens, n),
            total_redundant_bytes: redundant_volume(dup_bytes, n),
            effective_severity: effective.clone(),
            hard_fail: matches!(effective, CopyCodeSeverity::Hard),
            fingerprint: fp,
            reason: "same-name semantic unit copied across multiple files".into(),
            recommended_action: "keep the named unit in one owner and call it from the other sites"
                .into(),
            suppressed: None,
            instances,
        });
    }
    classes
}

fn build_warning_body_classes(candidates: &[CandidateFile]) -> Vec<CopyCodeClass> {
    let mut groups: HashMap<(String, String), Vec<UnitCandidate>> = HashMap::new();
    for candidate in candidates {
        for unit in extract_units(&candidate.file, &candidate.language, candidate.path_kind) {
            groups
                .entry((candidate.language.clone(), unit.body_hash()))
                .or_default()
                .push(unit);
        }
    }

    let mut classes = Vec::new();
    for ((language, body_hash), units) in groups {
        let distinct_names: BTreeSet<_> = units.iter().map(|unit| unit.name.clone()).collect();
        if units.len() < 2 || distinct_names.len() < 2 {
            continue;
        }
        let sample = &units[0];
        let instances = units
            .iter()
            .map(|unit| CopyCodeInstance {
                path: unit.path.clone(),
                language: unit.language.clone(),
                start_line: unit.start_line,
                end_line: unit.end_line,
                unit_name: Some(unit.name.clone()),
            })
            .collect::<Vec<_>>();
        let dup_lines = sample.body_line_count();
        let dup_tokens = sample.token_count;
        let dup_bytes = sample.byte_count;
        let n = instances.len();
        let effective = effective_severity_for(
            &CopyCodeKind::ExactUnitDifferentName,
            &CopyCodeSeverity::Warning,
        );
        let fp = class_fingerprint(
            &CopyCodeKind::ExactUnitDifferentName,
            &language,
            &instances,
            dup_lines,
        );
        classes.push(CopyCodeClass {
            id: class_id("body", &language, &body_hash),
            kind: CopyCodeKind::ExactUnitDifferentName,
            severity: CopyCodeSeverity::Warning,
            confidence: "medium".into(),
            language,
            unit_name: None,
            duplicate_lines: dup_lines,
            duplicate_tokens: dup_tokens,
            duplicate_bytes: dup_bytes,
            instance_count: n,
            total_redundant_lines: redundant_volume(dup_lines, n),
            total_redundant_tokens: redundant_volume(dup_tokens, n),
            total_redundant_bytes: redundant_volume(dup_bytes, n),
            effective_severity: effective.clone(),
            hard_fail: matches!(effective, CopyCodeSeverity::Hard),
            fingerprint: fp,
            reason: "same body appears under different names across files".into(),
            recommended_action:
                "choose one owner for the shared body or intentionally differentiate the behavior"
                    .into(),
            suppressed: None,
            instances,
        });
    }
    classes
}

fn build_token_block_classes(
    candidates: &[CandidateFile],
    min_lines: usize,
    min_tokens: usize,
) -> Vec<CopyCodeClass> {
    let mut groups: HashMap<(String, String), Vec<WindowCandidate>> = HashMap::new();
    for candidate in candidates {
        for window in extract_token_windows(
            &candidate.file,
            &candidate.language,
            candidate.path_kind,
            min_lines,
            min_tokens,
        ) {
            groups
                .entry((candidate.language.clone(), window.hash()))
                .or_default()
                .push(window);
        }
    }

    let mut classes = Vec::new();
    for ((language, hash), windows) in groups {
        if windows.len() < 2 {
            continue;
        }
        let sample = &windows[0];
        let instances = windows
            .iter()
            .map(|window| CopyCodeInstance {
                path: window.path.clone(),
                language: window.language.clone(),
                start_line: window.start_line,
                end_line: window.end_line,
                unit_name: None,
            })
            .collect::<Vec<_>>();
        let dup_lines = min_lines.max(sample.line_count());
        let dup_tokens = sample.token_count;
        let dup_bytes = sample.byte_count;
        let n = instances.len();
        let effective =
            effective_severity_for(&CopyCodeKind::TokenBlock, &CopyCodeSeverity::Warning);
        let fp = class_fingerprint(&CopyCodeKind::TokenBlock, &language, &instances, dup_lines);
        classes.push(CopyCodeClass {
            id: class_id("block", &language, &hash),
            kind: CopyCodeKind::TokenBlock,
            severity: CopyCodeSeverity::Warning,
            confidence: "medium".into(),
            language,
            unit_name: None,
            duplicate_lines: dup_lines,
            duplicate_tokens: dup_tokens,
            duplicate_bytes: dup_bytes,
            instance_count: n,
            total_redundant_lines: redundant_volume(dup_lines, n),
            total_redundant_tokens: redundant_volume(dup_tokens, n),
            total_redundant_bytes: redundant_volume(dup_bytes, n),
            effective_severity: effective.clone(),
            hard_fail: matches!(effective, CopyCodeSeverity::Hard),
            fingerprint: fp,
            reason: "strict token/block duplication exceeded the configured threshold".into(),
            recommended_action: "review whether the block is intentional; otherwise consolidate the shared logic behind one owner".into(),
            suppressed: None,
            instances,
        });
    }
    classes
}

fn is_workspace_manifest(rel_path: &str) -> bool {
    let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    matches!(
        name,
        "Cargo.toml" | "package.json" | "tsconfig.json" | "pyproject.toml" | "setup.cfg"
    )
}

fn classify_file(file: &FileInfo, include_tests: bool) -> Option<CandidateFile> {
    if is_excluded_path(&file.rel_path) || !file.is_code {
        return None;
    }
    let path_kind = if is_warning_only_path(&file.rel_path) || is_workspace_manifest(&file.rel_path)
    {
        if !include_tests {
            return None;
        }
        PathKind::WarningOnly
    } else {
        PathKind::Active
    };
    let language = language_for_path(file)?;
    Some(CandidateFile {
        file: file.clone(),
        language,
        path_kind,
    })
}

fn language_for_path(file: &FileInfo) -> Option<String> {
    let suffix = file.suffix.as_str();
    let name = file.name.as_str();
    let language = match suffix {
        ".rs" => "rust",
        ".py" => "python",
        ".ts" | ".tsx" => "typescript",
        ".js" | ".jsx" => "javascript",
        ".json" => "json",
        ".toml" => "toml",
        ".yaml" | ".yml" => "yaml",
        ".cfg" | ".conf" | ".ini" => "config",
        ".dockerfile" => "docker",
        _ if name.eq_ignore_ascii_case("Dockerfile") => "docker",
        _ => return None,
    };
    Some(language.into())
}

fn is_excluded_path(path: &str) -> bool {
    scan::is_generated_or_reference_path(path)
        || path.starts_with("vendor/")
        || path.contains("/vendor/")
        || path.starts_with("node_modules/")
        || path.contains("/node_modules/")
        || path.starts_with("dist/")
        || path.contains("/dist/")
        || path.starts_with("build/")
        || path.contains("/build/")
        || path.starts_with("cache/")
        || path.contains("/cache/")
        || path.starts_with(".cache/")
        || path.contains("/.cache/")
        || path.starts_with("tmp/")
        || path.contains("/tmp/")
        || path.starts_with("out/")
        || path.contains("/out/")
        || path.starts_with(".next/")
        || path.contains("/.next/")
        || path.starts_with(".turbo/")
        || path.contains("/.turbo/")
        || path.ends_with(".min.js")
        || path.ends_with(".min.ts")
        || path.ends_with(".min.jsx")
        || path.ends_with(".min.tsx")
        || is_lockfile(path)
        || path.contains("/__snapshots__/")
        || path.contains("/snapshots/")
}

fn is_warning_only_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with("tests/")
        || lower.contains("/tests/")
        || lower.starts_with("fixtures/")
        || lower.contains("/fixtures/")
        || lower.starts_with("stories/")
        || lower.contains("/stories/")
        || lower.starts_with("config/")
        || lower.contains("/config/")
        || lower.starts_with("docker/")
        || lower.contains("/docker/")
        || lower.ends_with("/dockerfile")
        || lower.ends_with("dockerfile")
        || lower.starts_with("ops/scripts/")
        || lower.contains("/ops/scripts/")
        || lower.starts_with("python_runtime/")
        || lower.contains("/python_runtime/")
        || lower.starts_with("runtime_payload/")
        || lower.contains("/runtime_payload/")
        || lower.starts_with("seed_data/")
        || lower.contains("/seed_data/")
        || lower.contains("/migrations/")
        || lower.starts_with("migrations/")
        || lower.contains("/spec/")
        || lower.ends_with(".test.rs")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".test.tsx")
        || lower.ends_with(".spec.rs")
        || lower.ends_with(".spec.ts")
        || lower.ends_with(".spec.tsx")
        || lower.ends_with("_test.rs")
        || lower.ends_with("_tests.rs")
}

fn is_lockfile(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "cargo.lock"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "poetry.lock"
            | "uv.lock"
    )
}

fn extract_units(file: &FileInfo, language: &str, path_kind: PathKind) -> Vec<UnitCandidate> {
    match language {
        "rust" => extract_rust_units(file, path_kind),
        "python" => extract_python_units(file, path_kind),
        "typescript" | "javascript" => extract_js_units(file, language, path_kind),
        _ => vec![],
    }
}

fn extract_rust_units(file: &FileInfo, path_kind: PathKind) -> Vec<UnitCandidate> {
    let mut units = Vec::new();
    let lines: Vec<_> = file.text.lines().collect();
    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some(name) = rust_unit_name(lines[idx]) {
            let start = idx;
            let mut brace_depth = brace_depth_before(&lines[..=idx]);
            let mut seen_body = lines[idx].contains('{');
            let mut end = idx;
            while end + 1 < lines.len() {
                end += 1;
                brace_depth += delta_braces(lines[end]);
                if lines[end].contains('{') {
                    seen_body = true;
                }
                if seen_body && brace_depth <= 0 {
                    break;
                }
            }
            if end <= start {
                end = start;
            }
            let unit_lines = &lines[start..=end.min(lines.len() - 1)];
            units.push(build_unit_candidate(
                file,
                "rust",
                path_kind,
                name,
                start + 1,
                end.min(lines.len() - 1) + 1,
                unit_lines,
            ));
            idx = end.saturating_add(1);
        } else {
            idx += 1;
        }
    }
    units
}

fn extract_python_units(file: &FileInfo, path_kind: PathKind) -> Vec<UnitCandidate> {
    let mut units = Vec::new();
    let lines: Vec<_> = file.text.lines().collect();
    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some((name, indent)) = python_unit_name(lines[idx]) {
            let start = idx;
            let mut end = idx;
            while end + 1 < lines.len() {
                let next = lines[end + 1];
                if next.trim().is_empty() || next.trim_start().starts_with('@') {
                    end += 1;
                    continue;
                }
                let next_indent = next.chars().take_while(|c| c.is_whitespace()).count();
                if next_indent <= indent && !next.trim_start().starts_with('#') {
                    break;
                }
                end += 1;
            }
            let unit_lines = &lines[start..=end];
            units.push(build_unit_candidate(
                file,
                "python",
                path_kind,
                name,
                start + 1,
                end + 1,
                unit_lines,
            ));
            idx = end.saturating_add(1);
        } else {
            idx += 1;
        }
    }
    units
}

fn extract_js_units(file: &FileInfo, language: &str, path_kind: PathKind) -> Vec<UnitCandidate> {
    let mut units = Vec::new();
    let lines: Vec<_> = file.text.lines().collect();
    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some(name) = js_unit_name(lines[idx]) {
            let start = idx;
            let mut brace_depth = brace_depth_before(&lines[..=idx]);
            let mut seen_body = lines[idx].contains('{') || lines[idx].contains("=>");
            let mut end = idx;
            while end + 1 < lines.len() {
                end += 1;
                brace_depth += delta_braces(lines[end]);
                if lines[end].contains('{') || lines[end].contains("=>") {
                    seen_body = true;
                }
                if seen_body
                    && brace_depth <= 0
                    && (lines[end].contains(';') || lines[end].contains('}'))
                {
                    break;
                }
            }
            let unit_lines = &lines[start..=end.min(lines.len() - 1)];
            units.push(build_unit_candidate(
                file,
                language,
                path_kind,
                name,
                start + 1,
                end.min(lines.len() - 1) + 1,
                unit_lines,
            ));
            idx = end.saturating_add(1);
        } else {
            idx += 1;
        }
    }
    units
}

fn build_unit_candidate(
    file: &FileInfo,
    language: &str,
    path_kind: PathKind,
    name: String,
    start_line: usize,
    end_line: usize,
    lines: &[&str],
) -> UnitCandidate {
    let signature_text = normalize_exact_text(lines.first().copied().unwrap_or_default());
    let body_lines = if lines.len() > 1 {
        &lines[1..]
    } else {
        &[][..]
    };
    let body_text = normalize_exact_text(&body_lines.join("\n"));
    let token_count = count_tokens(&body_text);
    let byte_count = body_text.len();
    UnitCandidate {
        path: file.rel_path.clone(),
        language: language.into(),
        name,
        path_kind,
        start_line,
        end_line,
        signature_text,
        body_text,
        token_count,
        byte_count,
    }
}

fn extract_token_windows(
    file: &FileInfo,
    language: &str,
    _path_kind: PathKind,
    min_lines: usize,
    min_tokens: usize,
) -> Vec<WindowCandidate> {
    let lines = file
        .text
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let normalized = normalize_token_line(line);
            if normalized.is_empty() {
                return None;
            }
            let token_count = count_tokens(&normalized);
            if token_count < 3 {
                return None;
            }
            if is_boilerplate_line(line) {
                return None;
            }
            Some((idx + 1, normalized, token_count))
        })
        .collect::<Vec<_>>();

    if lines.len() < min_lines {
        return vec![];
    }

    let mut windows = Vec::new();
    let mut seen = HashSet::new();
    for start in 0..=lines.len() - min_lines {
        let slice = &lines[start..start + min_lines];
        let token_count: usize = slice.iter().map(|(_, _, count)| *count).sum();
        if token_count < min_tokens {
            continue;
        }
        let text = slice
            .iter()
            .map(|(_, text, _)| text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let hash = sha256(&text);
        let start_line = slice.first().map(|item| item.0).unwrap_or(1);
        let end_line = slice.last().map(|item| item.0).unwrap_or(start_line);
        let key = (file.rel_path.clone(), hash.clone());
        if !seen.insert(key) {
            continue;
        }
        windows.push(WindowCandidate {
            path: file.rel_path.clone(),
            language: language.into(),
            start_line,
            end_line,
            text,
            token_count,
            byte_count: slice.iter().map(|(_, text, _)| text.len()).sum(),
        });
    }
    windows
}

fn normalize_exact_text(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.replace("\r\n", "\n").replace('\r', "\n").lines() {
        lines.push(line.trim_end().to_string());
    }
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let mut out = Vec::new();
    let mut blank = false;
    for line in lines {
        if line.trim().is_empty() {
            if !blank {
                out.push(String::new());
            }
            blank = true;
        } else {
            out.push(line);
            blank = false;
        }
    }
    out.join("\n")
}

fn normalize_token_line(line: &str) -> String {
    static STRING_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#""([^"\\]|\\.)*"|'([^'\\]|\\.)*'|`([^`\\]|\\.)*`"#).expect("regex")
    });
    static NUMBER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b\d+(?:\.\d+)?\b").expect("regex"));
    static IDENT_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\b[A-Za-z_][A-Za-z0-9_]*\b").expect("regex"));

    let mut text = line.trim().to_string();
    text = STRING_RE.replace_all(&text, "STR").to_string();
    text = NUMBER_RE.replace_all(&text, "NUM").to_string();
    text = IDENT_RE.replace_all(&text, "ID").to_string();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_boilerplate_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty()
        || trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with('#')
        || trimmed.starts_with("import ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("from ")
        || trimmed.starts_with("export type ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("interface ")
        || trimmed.starts_with("package ")
        || trimmed.starts_with("#[derive(")
        || trimmed.starts_with("#[serde(")
        || trimmed.starts_with("impl ") && trimmed.ends_with(" {")
        || trimmed == "{"
        || trimmed == "}"
        || trimmed == "};"
        || trimmed == "});"
        || trimmed == ";"
        || trimmed.len() < 12
}

fn rust_unit_name(line: &str) -> Option<String> {
    static RUST_UNIT_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\b")
            .expect("regex")
    });
    RUST_UNIT_RE
        .captures(line)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

fn python_unit_name(line: &str) -> Option<(String, usize)> {
    static PY_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^(\s*)(?:async\s+def|def|class)\s+([A-Za-z_][A-Za-z0-9_]*)\b").expect("regex")
    });
    PY_RE.captures(line).map(|caps| {
        let indent = caps.get(1).map(|m| m.as_str().len()).unwrap_or(0);
        let name = caps
            .get(2)
            .map(|m| m.as_str())
            .unwrap_or_default()
            .to_string();
        (name, indent)
    })
}

fn js_unit_name(line: &str) -> Option<String> {
    static JS_FUN_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)\b")
            .expect("regex")
    });
    static JS_ARROW_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(?:async\s*)?\(?[^=;]*\)?\s*=>",
        )
        .expect("regex")
    });
    static JS_CLASS_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^\s*(?:export\s+)?class\s+([A-Za-z_$][A-Za-z0-9_$]*)\b").expect("regex")
    });
    JS_FUN_RE
        .captures(line)
        .or_else(|| JS_ARROW_RE.captures(line))
        .or_else(|| JS_CLASS_RE.captures(line))
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

fn brace_depth_before(lines: &[&str]) -> isize {
    lines.iter().map(|line| delta_braces(line)).sum()
}

fn delta_braces(line: &str) -> isize {
    let mut delta = 0isize;
    let mut in_string = false;
    let mut quote = '\0';
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_string {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = false;
            }
            continue;
        }
        if ch == '"' || ch == '\'' || ch == '`' {
            in_string = true;
            quote = ch;
            continue;
        }
        if ch == '{' {
            delta += 1;
        } else if ch == '}' {
            delta -= 1;
        }
    }
    delta
}

fn redundant_volume(per_instance: usize, instance_count: usize) -> usize {
    if instance_count <= 1 {
        0
    } else {
        per_instance.saturating_mul(instance_count - 1)
    }
}

fn effective_severity_for(kind: &CopyCodeKind, raw: &CopyCodeSeverity) -> CopyCodeSeverity {
    match (kind, raw) {
        (CopyCodeKind::ExactFile, CopyCodeSeverity::Hard) => CopyCodeSeverity::Hard,
        (CopyCodeKind::ExactUnitSameName, CopyCodeSeverity::Hard) => CopyCodeSeverity::Hard,
        _ => CopyCodeSeverity::Warning,
    }
}

fn class_fingerprint(
    kind: &CopyCodeKind,
    language: &str,
    instances: &[CopyCodeInstance],
    per_instance_lines: usize,
) -> String {
    let mut h = Sha256::new();
    h.update(format!("{kind:?}|{language}|{per_instance_lines}|").as_bytes());
    let mut paths: Vec<&str> = instances.iter().map(|i| i.path.as_str()).collect();
    paths.sort();
    for p in paths {
        h.update(p.as_bytes());
        h.update(b"\n");
    }
    let digest = h.finalize();
    let mut s = String::with_capacity(16);
    for b in &digest[..8] {
        use std::fmt::Write as FmtWrite;
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn kind_label(kind: &CopyCodeKind) -> &'static str {
    match kind {
        CopyCodeKind::ExactFile => "exact_file",
        CopyCodeKind::ExactUnitSameName => "exact_unit_same_name",
        CopyCodeKind::ExactUnitDifferentName => "exact_unit_different_name",
        CopyCodeKind::TokenBlock => "token_block",
    }
}

fn severity_rank(severity: &CopyCodeSeverity) -> usize {
    match severity {
        CopyCodeSeverity::Hard => 2,
        CopyCodeSeverity::Warning => 1,
    }
}

fn class_id(kind: &str, language: &str, hash: &str) -> String {
    let prefix: String = hash.chars().skip(7).take(16).collect();
    format!("copy-code-{kind}-{language}-{prefix}")
}

fn sha256(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

fn count_tokens(text: &str) -> usize {
    text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
        .filter(|part| !part.is_empty())
        .count()
}

fn excluded_roots() -> Vec<String> {
    vec![
        "build/".into(),
        "cache/".into(),
        "dist/".into(),
        "docs/".into(),
        "generated/".into(),
        "node_modules/".into(),
        "paper/".into(),
        "reference/".into(),
        "target/".into(),
        "tips/".into(),
        "vendor/".into(),
    ]
}

fn warning_only_roots() -> Vec<String> {
    vec![
        "config/".into(),
        "docker/".into(),
        "fixtures/".into(),
        "migrations/".into(),
        "ops/scripts/".into(),
        "python_runtime/".into(),
        "stories/".into(),
        "runtime_payload/".into(),
        "seed_data/".into(),
        "tests/".into(),
    ]
}

fn now_string() -> String {
    chrono::Utc::now().to_rfc3339()
}

impl UnitCandidate {
    fn signature_hash(&self) -> String {
        sha256(&format!(
            "{}\n{}\n{}",
            self.name, self.signature_text, self.body_text
        ))
    }

    fn body_hash(&self) -> String {
        sha256(&self.body_text)
    }

    fn body_line_count(&self) -> usize {
        self.body_text.lines().count().max(1)
    }
}

impl WindowCandidate {
    fn hash(&self) -> String {
        sha256(&self.text)
    }

    fn line_count(&self) -> usize {
        self.end_line
            .saturating_sub(self.start_line)
            .saturating_add(1)
    }
}
