use super::helpers::{AuditContext, PYTHON_STACK_RECLASSIFY_CAPS};
use super::language_rules::common::{contains_unqualified_python_builtin_call, python_code_lines};
use crate::boundaries::manifest::{self, AuditedRuntimeBoundary};
use crate::model::{BoundaryEvidenceArtifactSummary, BoundaryReclassification};
use crate::validation::{self, ArtifactSchema};
use globset::{Glob, GlobSetBuilder};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

const BUILTIN_CHECKS: &[(&str, &[&str])] = &[
    (
        "no-direct-db-access",
        &[
            "sqlalchemy",
            "psycopg",
            "sqlite3",
            "mysql.connector",
            "pymongo",
        ],
    ),
    (
        "no-product-routes",
        &["@app.route", "@router.", "fastapi(", "apirouter("],
    ),
    (
        "no-subprocess",
        &[
            "subprocess",
            "os.system",
            "popen(",
            "shell=true",
            "shell = true",
        ],
    ),
    ("no-import-escape", &["sys.path", "importlib", "__import__"]),
    (
        "no-builtin-escape",
        &["eval(", "exec(", "compile(", "__builtins__"],
    ),
];

#[derive(Debug, Deserialize)]
struct BoundaryEvidence {
    boundary_id: String,
    classification: String,
    runtime_language: String,
    paths: Vec<String>,
    files: Vec<BoundaryEvidenceFile>,
    checks: Vec<BoundaryEvidenceCheck>,
    summary: BoundaryEvidenceSummary,
}

#[derive(Debug, Deserialize)]
struct BoundaryEvidenceFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct BoundaryEvidenceCheck {
    id: String,
    status: String,
    message: Option<String>,
    path: Option<String>,
    line: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct BoundaryEvidenceSummary {
    passed: bool,
    failed_count: usize,
}

struct BoundaryWork {
    entry: AuditedRuntimeBoundary,
    covered_files: Vec<String>,
    covered_line_count: usize,
    invalid: Vec<String>,
    unsupported: Vec<String>,
}

pub fn evaluate(ctx: &AuditContext) -> Vec<BoundaryReclassification> {
    let Ok(manifest) = manifest::load(&ctx.root.join("agent/boundaries.toml")) else {
        return vec![];
    };

    let mut works = manifest
        .audited_runtime_boundary
        .into_iter()
        .map(|entry| build_work(ctx, entry))
        .collect::<Vec<_>>();
    mark_overlaps(&mut works);
    works
        .into_iter()
        .map(|work| finalize_work(ctx, work))
        .collect()
}

fn build_work(ctx: &AuditContext, entry: AuditedRuntimeBoundary) -> BoundaryWork {
    let mut invalid = Vec::new();
    let mut unsupported = Vec::new();
    if entry.runtime_language != "python" || !entry.target_stack_exception {
        unsupported.push("only python target_stack_exception boundaries are supported".into());
    }
    if entry.reclassifies.is_empty() {
        unsupported
            .push("boundary must declare at least one Python stack cap reclassification".into());
    }
    for cap in &entry.reclassifies {
        if !PYTHON_STACK_RECLASSIFY_CAPS
            .iter()
            .any(|allowed| allowed == cap)
        {
            invalid.push(format!("unsupported reclassifies cap `{cap}`"));
        }
    }

    let mut builder = GlobSetBuilder::new();
    for path in &entry.paths {
        if let Some(problem) = invalid_boundary_path(path) {
            invalid.push(format!("{path}: {problem}"));
            continue;
        }
        match Glob::new(path) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(err) => invalid.push(format!("{path}: invalid glob: {err}")),
        }
    }

    let globset = if invalid.is_empty() {
        builder.build().ok()
    } else {
        None
    };
    let mut covered_files = Vec::new();
    let mut covered_line_count = 0;
    if let Some(globset) = globset.as_ref() {
        for file in &ctx.all_files {
            if file.suffix == ".py" && globset.is_match(&file.rel_path) {
                covered_line_count += file.line_count;
                covered_files.push(file.rel_path.clone());
            }
        }
        if covered_files.is_empty() {
            invalid.push("boundary paths did not match any Python files".into());
        }
    }

    BoundaryWork {
        entry,
        covered_files,
        covered_line_count,
        invalid,
        unsupported,
    }
}

fn invalid_boundary_path(path: &str) -> Option<&'static str> {
    if path.trim().is_empty() {
        return Some("empty path");
    }
    if path == "." || path == "/" || path == "*" || path == "**" || path == "**/*" {
        return Some("repo-root glob is not allowed");
    }
    if path.starts_with('*') || !path.contains('/') {
        return Some("repo-root glob is not allowed");
    }
    if path.starts_with('/') || Path::new(path).is_absolute() {
        return Some("absolute paths are not allowed");
    }
    if Path::new(path)
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Some("parent traversal is not allowed");
    }
    for prefix in ["agent/", ".github/", "docs/", "reference/"] {
        if path == prefix.trim_end_matches('/') || path.starts_with(prefix) {
            return Some("control paths cannot be audited runtime payloads");
        }
    }
    None
}

fn mark_overlaps(works: &mut [BoundaryWork]) {
    let mut owners: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, work) in works.iter().enumerate() {
        for file in &work.covered_files {
            owners.entry(file.clone()).or_default().push(idx);
        }
    }
    for (file, indices) in owners {
        if indices.len() < 2 {
            continue;
        }
        let ids = indices
            .iter()
            .map(|idx| works[*idx].entry.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        for idx in indices {
            works[idx].invalid.push(format!(
                "overlapping audited boundaries for `{file}`: {ids}"
            ));
        }
    }
}

fn finalize_work(ctx: &AuditContext, work: BoundaryWork) -> BoundaryReclassification {
    let mut missing_checks = Vec::new();
    let mut failed_checks = Vec::new();
    let mut artifacts = Vec::new();
    failed_checks.extend(work.invalid.iter().cloned());
    failed_checks.extend(work.unsupported.iter().cloned());

    let allowed_caps = work
        .entry
        .reclassifies
        .iter()
        .filter(|cap| {
            PYTHON_STACK_RECLASSIFY_CAPS
                .iter()
                .any(|allowed| allowed == cap)
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut evidence_loaded = false;
    for rel in &work.entry.required_evidence {
        match load_evidence(ctx, rel) {
            EvidenceLoad::Missing(summary) => {
                artifacts.push(summary);
                missing_checks.push(format!("missing evidence artifact `{rel}`"));
            }
            EvidenceLoad::Invalid(summary, err) => {
                artifacts.push(summary);
                failed_checks.push(format!("invalid evidence artifact `{rel}`: {err}"));
            }
            EvidenceLoad::Loaded(summary, evidence) => {
                artifacts.push(summary);
                evidence_loaded = true;
                validate_evidence(
                    ctx,
                    &work,
                    &evidence,
                    &mut missing_checks,
                    &mut failed_checks,
                );
            }
        }
    }
    if work.entry.required_evidence.is_empty() {
        missing_checks.push("required_evidence is empty".into());
    } else if !evidence_loaded && failed_checks.is_empty() {
        missing_checks.push("no boundary evidence could be loaded".into());
    }
    run_builtin_checks(ctx, &work.covered_files, &mut failed_checks);

    let status = if !work.unsupported.is_empty() {
        "unsupported"
    } else if !work.invalid.is_empty() {
        "invalid"
    } else if !evidence_loaded && failed_checks.is_empty() {
        "missing_evidence"
    } else if !missing_checks.is_empty() || !failed_checks.is_empty() {
        "failed"
    } else {
        "passed"
    };
    let suppresses_python_stack_caps = matches!(status, "missing_evidence" | "failed")
        && !allowed_caps.is_empty()
        && work.invalid.is_empty()
        && work.unsupported.is_empty()
        && !failed_checks
            .iter()
            .any(|check| BUILTIN_CHECKS.iter().any(|(id, _)| check.starts_with(*id)));

    BoundaryReclassification {
        id: work.entry.id,
        paths: work.entry.paths,
        classification: work.entry.classification,
        product_surface: work.entry.product_surface,
        runtime_language: work.entry.runtime_language,
        status: status.into(),
        reclassified_caps: allowed_caps,
        covered_file_count: work.covered_files.len(),
        covered_line_count: work.covered_line_count,
        covered_files: work.covered_files,
        evidence_artifacts: artifacts,
        missing_checks,
        failed_checks,
        rerun_command: work
            .entry
            .rerun_command
            .or(work.entry.proof_command)
            .unwrap_or_else(|| "cargo run -p jankurai -- .".into()),
        suppresses_python_stack_caps,
    }
}

enum EvidenceLoad {
    Missing(BoundaryEvidenceArtifactSummary),
    Invalid(BoundaryEvidenceArtifactSummary, String),
    Loaded(BoundaryEvidenceArtifactSummary, BoundaryEvidence),
}

fn load_evidence(ctx: &AuditContext, rel: &str) -> EvidenceLoad {
    let path = ctx.root.join(rel);
    if !path.is_file() {
        return EvidenceLoad::Missing(artifact_summary(rel, "missing_evidence", None, 0, 0));
    }
    let sha = file_sha256(&path);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            return EvidenceLoad::Invalid(
                artifact_summary(rel, "invalid", sha, 0, 0),
                err.to_string(),
            )
        }
    };
    let value = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(value) => value,
        Err(err) => {
            return EvidenceLoad::Invalid(
                artifact_summary(rel, "invalid", sha, 0, 0),
                err.to_string(),
            )
        }
    };
    if let Err(err) =
        validation::validate_value(&ctx.root, ArtifactSchema::BoundaryEvidence, &value)
    {
        return EvidenceLoad::Invalid(artifact_summary(rel, "invalid", sha, 0, 0), err.to_string());
    }
    let evidence = match serde_json::from_value::<BoundaryEvidence>(value) {
        Ok(evidence) => evidence,
        Err(err) => {
            return EvidenceLoad::Invalid(
                artifact_summary(rel, "invalid", sha, 0, 0),
                err.to_string(),
            )
        }
    };
    let summary = artifact_summary(
        rel,
        "loaded",
        sha,
        evidence.files.len(),
        evidence.checks.len(),
    );
    EvidenceLoad::Loaded(summary, evidence)
}

fn artifact_summary(
    path: &str,
    status: &str,
    sha256: Option<String>,
    file_count: usize,
    check_count: usize,
) -> BoundaryEvidenceArtifactSummary {
    BoundaryEvidenceArtifactSummary {
        path: path.into(),
        status: status.into(),
        sha256,
        file_count,
        check_count,
    }
}

fn validate_evidence(
    ctx: &AuditContext,
    work: &BoundaryWork,
    evidence: &BoundaryEvidence,
    missing: &mut Vec<String>,
    failed: &mut Vec<String>,
) {
    if evidence.boundary_id != work.entry.id {
        failed.push(format!(
            "boundary-id-match: evidence `{}` does not match manifest `{}`",
            evidence.boundary_id, work.entry.id
        ));
    }
    if evidence.classification != work.entry.classification {
        failed.push("classification-match: evidence classification differs from manifest".into());
    }
    if evidence.runtime_language != work.entry.runtime_language {
        failed
            .push("runtime-language-match: evidence runtime language differs from manifest".into());
    }
    if evidence.paths != work.entry.paths {
        failed.push("paths-match: evidence paths differ from manifest".into());
    }

    let covered = work.covered_files.iter().cloned().collect::<BTreeSet<_>>();
    let by_file = evidence
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.sha256.as_str()))
        .collect::<BTreeMap<_, _>>();
    for file in &covered {
        let Some(recorded_sha) = by_file.get(file.as_str()) else {
            missing.push(format!("manifest-coverage: missing `{file}`"));
            continue;
        };
        let Some(actual_sha) = file_sha256(&ctx.root.join(file)) else {
            failed.push(format!("payload-hash-match: could not hash `{file}`"));
            continue;
        };
        if normalize_sha256(recorded_sha) != actual_sha {
            failed.push(format!(
                "payload-hash-match: `{file}` expected current {actual_sha}"
            ));
        }
    }
    for file in &evidence.files {
        if !covered.contains(&file.path) {
            failed.push(format!(
                "manifest-coverage: evidence file `{}` is outside boundary",
                file.path
            ));
        }
    }
    for required in &work.entry.required_checks {
        let Some(check) = evidence.checks.iter().find(|check| check.id == *required) else {
            missing.push(format!("{required}: missing required check"));
            continue;
        };
        if !matches!(check.status.as_str(), "passed" | "pass") {
            failed.push(format_check(check));
        }
    }
    if !evidence.summary.passed || evidence.summary.failed_count > 0 {
        failed.push(format!(
            "summary: evidence reports passed={} failed_count={}",
            evidence.summary.passed, evidence.summary.failed_count
        ));
    }
}

fn format_check(check: &BoundaryEvidenceCheck) -> String {
    let mut value = format!("{}: status `{}`", check.id, check.status);
    if let Some(path) = &check.path {
        value.push_str(&format!(" path `{path}`"));
    }
    if let Some(line) = check.line {
        value.push_str(&format!(" line `{line}`"));
    }
    if let Some(message) = &check.message {
        value.push_str(&format!(" {message}"));
    }
    value
}

fn run_builtin_checks(ctx: &AuditContext, covered_files: &[String], failed: &mut Vec<String>) {
    for rel in covered_files {
        let Some(file) = ctx.all_files.iter().find(|file| file.rel_path == *rel) else {
            continue;
        };
        for (line_no, line) in python_code_lines(&file.text) {
            let lower = line.to_ascii_lowercase();
            for (id, markers) in BUILTIN_CHECKS {
                for marker in markers.iter().copied() {
                    let matched = if matches!(marker, "eval(" | "exec(" | "compile(") {
                        contains_unqualified_python_builtin_call(
                            &lower,
                            marker.trim_end_matches('('),
                        )
                    } else {
                        lower.contains(marker)
                    };
                    if matched {
                        failed.push(format!("{id}: `{rel}` line {line_no} contains `{marker}`"));
                        break;
                    }
                }
            }
        }
    }
}

fn file_sha256(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn normalize_sha256(value: &str) -> String {
    if value.starts_with("sha256:") {
        value.to_string()
    } else {
        format!("sha256:{value}")
    }
}
