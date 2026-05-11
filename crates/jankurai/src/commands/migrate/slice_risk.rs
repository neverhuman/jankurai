use anyhow::{bail, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

use super::plan::{MigrationPlan, MigrationSlice};

#[derive(Debug, Clone)]
pub struct SliceRiskArgs {
    pub repo: PathBuf,
    pub plan: String,
    pub slice_id: String,
    pub out: Option<String>,
    pub md: Option<String>,
    pub check_env: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SliceRiskReport {
    pub schema_version: String,
    pub command: String,
    pub status: String,
    pub decision: String,
    pub repo: String,
    pub plan: String,
    pub slice_id: String,
    pub slice_status: String,
    pub risk_level: String,
    pub check_env: bool,
    pub signals_total: usize,
    pub critical_signals: usize,
    pub high_signals: usize,
    pub medium_signals: usize,
    pub low_signals: usize,
    pub env_checks: Vec<SliceEnvCheck>,
    pub signals: Vec<SliceSignal>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SliceEnvCheck {
    pub name: String,
    pub present: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SliceSignal {
    pub kind: String,
    pub severity: String,
    pub decision: String,
    pub evidence: Vec<String>,
    pub recommendation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignalDecision {
    Block,
    Review,
    Pass,
}

static ENV_NAME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?m)\b(?:std::env::var|env::var|os\.getenv|getenv|process\.env\.|ENV\[)\s*(?:\(|\[\s*)?["']?(?P<name>[A-Z][A-Z0-9_]{2,})["']?"#,
    )
    .expect("env name regex")
});
static TORCH_LOAD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"torch\.load\s*\(").expect("torch.load"));
static WATERSHED_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(torch\.load|pickle\.load|joblib\.load)\b").expect("load"));
static CHECKPOINT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(checkpoint|weights_only|model_path|\.ckpt|\.pth|\.pt)\b")
        .expect("checkpoint")
});
static CUDA_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(cuda|gpu|torch\.device|nvidia|cupy)\b").expect("cuda"));
static TEST_CMD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(cargo test|pytest|go test|npm test|pnpm test|yarn test|python -m pytest)\b")
        .expect("test command")
});
static PYO3_TOKIO_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(pyo3|Python::with_gil|spawn_blocking|tokio::spawn|tokio)\b")
        .expect("pyo3 tokio")
});
static MULTIPROCESSING_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(multiprocessing|ProcessPoolExecutor|Pool\()\b").expect("multiprocessing")
});
static ASYNC_PY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(asyncio|run_in_executor|to_thread|await\s+)\b").expect("async python")
});
static MUTABLE_GLOBAL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^\s*(global\s+\w+|static\s+mut\s+\w+|let\s+mut\s+[A-Z_]+|[A-Z_]+\s*=\s*(\{\}|\[\]|dict\(\)|set\(\)))")
        .expect("mutable globals")
});
static NUMPY_RANDOM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(np\.random|numpy\.random|random\.seed|np\.random\.seed)\b")
        .expect("numpy random")
});
static PRIOR_FAILURE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(previous|prior|after|on)\s+failure\b|\bfailure[_ -]?hook\b")
        .expect("prior failure")
});
static MODEL_SINGLETON_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(OnceCell|Lazy|singleton|get_instance|instance\(\)|static\s+INSTANCE|static\s+MODEL)\b")
        .expect("singleton")
});
static HMAC_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(hmac|signing|secret key|private key|jwt secret|api key)\b").expect("hmac")
});
static PROSE_ENV_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\benv(?:ironment)?\s+(?:var(?:iable)?\s+)?(?P<name>[A-Z][A-Z0-9_]{2,})\b")
        .expect("prose env regex")
});

const THREAD_COUNT_ENV_NAMES: &[&str] = &[
    "OMP_NUM_THREADS",
    "MKL_NUM_THREADS",
    "NUMEXPR_NUM_THREADS",
    "OPENBLAS_NUM_THREADS",
    "RAYON_NUM_THREADS",
    "TORCH_NUM_THREADS",
    "NUM_THREADS",
];

pub fn run(args: SliceRiskArgs) -> Result<()> {
    let repo = canonicalize_repo(&args.repo)?;
    let plan_path = resolve_repo_relative_existing(&repo, &args.plan)?;
    let plan_text =
        fs::read_to_string(&plan_path).with_context(|| format!("read {}", plan_path.display()))?;
    let plan: MigrationPlan = serde_json::from_str(&plan_text)
        .with_context(|| format!("parse {}", plan_path.display()))?;
    let slice = plan
        .slices
        .iter()
        .find(|slice| slice.slice_id == args.slice_id)
        .with_context(|| format!("slice `{}` not found in plan", args.slice_id))?;

    let mut env_names = BTreeSet::new();
    if args.check_env {
        collect_env_names_from_slice(slice, &mut env_names);
    }

    let mut signals = Vec::new();
    let mut scan_inputs: Vec<(String, String, Option<String>)> = Vec::new();
    scan_inputs.push((
        format!("slice:{}", slice.slice_id),
        slice_strings(slice),
        Some("slice".to_string()),
    ));
    let scan_scope = slice_scoped_files(&repo, slice)?;
    for missing in scan_scope.missing_paths {
        signals.push(signal(
            "slice-path-missing",
            "medium",
            SignalDecision::Review,
            vec![format!("selected allowed path `{missing}` matched no files")],
            "update the slice allowed_paths or add the selected files before relying on this risk scan",
            Some(missing),
            None,
        ));
    }
    for path in scan_scope.files {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let rel = path
            .strip_prefix(&repo)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_string();
        scan_inputs.push((rel, text, Some(ext)));
    }

    for (origin, text, ext) in scan_inputs {
        signals.extend(scan_text(&origin, &text, ext.as_deref()));
        if args.check_env {
            collect_env_names(&text, &mut env_names);
        }
    }

    let env_checks = if args.check_env {
        env_names
            .into_iter()
            .map(|name| SliceEnvCheck {
                present: env::var_os(&name).is_some(),
                name,
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let critical_signals = signals.iter().filter(|s| s.severity == "critical").count();
    let high_signals = signals.iter().filter(|s| s.severity == "high").count();
    let medium_signals = signals.iter().filter(|s| s.severity == "medium").count();
    let low_signals = signals.iter().filter(|s| s.severity == "low").count();
    let decision = if critical_signals > 0 || high_signals > 0 {
        SignalDecision::Block
    } else if medium_signals > 0 {
        SignalDecision::Review
    } else {
        SignalDecision::Pass
    };

    let mut recommendations = Vec::new();
    if critical_signals > 0 || high_signals > 0 {
        recommendations.push("add shadow/equivalence gate before cutover".to_string());
    }
    for signal in &signals {
        recommendations.push(signal.recommendation.clone());
    }
    recommendations.sort();
    recommendations.dedup();

    let report = SliceRiskReport {
        schema_version: "1.0.0".to_string(),
        command: "jankurai migrate slice-risk".to_string(),
        status: "complete".to_string(),
        decision: decision_label(decision).to_string(),
        repo: repo.display().to_string(),
        plan: plan_path.display().to_string(),
        slice_id: slice.slice_id.clone(),
        slice_status: slice.status.clone(),
        risk_level: slice.risk_level.clone(),
        check_env: args.check_env,
        signals_total: signals.len(),
        critical_signals,
        high_signals,
        medium_signals,
        low_signals,
        env_checks,
        signals,
        recommendations,
    };

    if let Some(path) = args.out.as_deref() {
        crate::validation::write_json(
            &repo,
            crate::validation::ArtifactSchema::MigrationSliceRisk,
            path,
            &report,
        )?;
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&report))?;
    }

    Ok(())
}

fn scan_text(origin: &str, text: &str, ext: Option<&str>) -> Vec<SliceSignal> {
    let mut signals = Vec::new();
    let is_docs = matches!(ext, Some("md" | "markdown" | "txt" | "toml" | "json"));
    let lines: Vec<&str> = text.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        if !is_docs && is_comment_only(line) {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if contains_any(
            &lower,
            &["env::var", "process.env", "os.getenv", "getenv(", "env["],
        ) {
            if let Some(signal) = env_signal(origin, idx + 1, line) {
                signals.push(signal);
            }
        }
        if CHECKPOINT_RE.is_match(line)
            && contains_any(
                &lower,
                &["checkpoint", "weights_only", ".pt", ".pth", ".ckpt"],
            )
        {
            signals.push(signal(
                "checkpoint-path",
                if line.contains("weights_only") {
                    "medium"
                } else {
                    "high"
                },
                if line.contains("weights_only") {
                    SignalDecision::Review
                } else {
                    SignalDecision::Block
                },
                vec![format!("{origin}:{}", idx + 1), line.trim().to_string()],
                "require explicit weights_only or isolate model loading behind a trusted boundary",
                Some(origin.to_string()),
                Some(idx + 1),
            ));
        }
        if (TORCH_LOAD_RE.is_match(line) || WATERSHED_RE.is_match(line))
            && !line.contains("weights_only")
        {
            signals.push(signal(
                "torch-load-without-weights-only",
                "critical",
                SignalDecision::Block,
                vec![format!("{origin}:{}", idx + 1), line.trim().to_string()],
                "add explicit weights_only=True or stop loading untrusted checkpoints",
                Some(origin.to_string()),
                Some(idx + 1),
            ));
        }
        if HMAC_RE.is_match(line) && secret_reference_context(line) {
            let (severity, decision, recommendation) = classify_secret_reference(is_docs, line);
            signals.push(signal(
                "signing-or-secret-reference",
                severity,
                decision,
                vec![format!("{origin}:{}", idx + 1), line.trim().to_string()],
                recommendation,
                Some(origin.to_string()),
                Some(idx + 1),
            ));
        }
        if CUDA_RE.is_match(line) {
            signals.push(signal(
                "gpu-cuda-reference",
                "medium",
                SignalDecision::Review,
                vec![format!("{origin}:{}", idx + 1), line.trim().to_string()],
                "document GPU/CUDA prerequisites explicitly and keep the slice runnable on CPU-only machines where possible",
                Some(origin.to_string()),
                Some(idx + 1),
            ));
        }
        if TEST_CMD_RE.is_match(line) {
            signals.push(signal(
                "test-command",
                "low",
                SignalDecision::Review,
                vec![format!("{origin}:{}", idx + 1), line.trim().to_string()],
                "pin the exact test command in the slice plan and verify it in the proof lane",
                Some(origin.to_string()),
                Some(idx + 1),
            ));
        }
        if MULTIPROCESSING_RE.is_match(line) {
            signals.push(signal(
                "multiprocessing",
                "medium",
                SignalDecision::Review,
                vec![format!("{origin}:{}", idx + 1), line.trim().to_string()],
                "treat multiprocessing as a cross-process boundary and verify serialization plus startup cost",
                Some(origin.to_string()),
                Some(idx + 1),
            ));
        }
        if NUMPY_RANDOM_RE.is_match(line) {
            signals.push(signal(
                "numpy-random-state",
                "medium",
                SignalDecision::Review,
                vec![format!("{origin}:{}", idx + 1), line.trim().to_string()],
                "seed or isolate numpy random state before equivalence checks",
                Some(origin.to_string()),
                Some(idx + 1),
            ));
        }
        if contains_any(line, THREAD_COUNT_ENV_NAMES) {
            signals.push(signal(
                "thread-count-env",
                "low",
                SignalDecision::Review,
                vec![format!("{origin}:{}", idx + 1), line.trim().to_string()],
                "pin thread-count env vars explicitly so the slice stays reproducible across machines",
                Some(origin.to_string()),
                Some(idx + 1),
            ));
        }
        if PRIOR_FAILURE_RE.is_match(line) {
            signals.push(signal(
                "prior-failure-hook",
                "low",
                SignalDecision::Review,
                vec![format!("{origin}:{}", idx + 1), line.trim().to_string()],
                "treat prior-failure hooks as advisory until the recovery path and fallback state are proven",
                Some(origin.to_string()),
                Some(idx + 1),
            ));
        }
        if MODEL_SINGLETON_RE.is_match(line) {
            signals.push(signal(
                "model-singleton",
                "medium",
                SignalDecision::Review,
                vec![format!("{origin}:{}", idx + 1), line.trim().to_string()],
                "replace singleton model loading with an explicit lifecycle or test seam",
                Some(origin.to_string()),
                Some(idx + 1),
            ));
        }
    }

    let lower = text.to_ascii_lowercase();
    if PYO3_TOKIO_RE.is_match(text)
        && lower.contains("pyo3")
        && lower.contains("tokio")
        && contains_any(&lower, &["with_gil", "spawn_blocking", "tokio::spawn"])
    {
        signals.push(signal(
            "pyo3-tokio-crossing",
            "high",
            SignalDecision::Block,
            vec![format!("{origin} combines PyO3 and Tokio primitives")],
            "add a shadow/equivalence gate before cutover and keep the crossing behind a narrow adapter",
            Some(origin.to_string()),
            None,
        ));
    }

    if ASYNC_PY_RE.is_match(text)
        && lower.contains("asyncio")
        && contains_any(&lower, &["run_in_executor", "to_thread", "await "])
    {
        signals.push(signal(
            "async-python-crossing",
            "medium",
            SignalDecision::Review,
            vec![format!("{origin} contains asyncio executor crossings")],
            "pin the async boundary and verify it with targeted preflight tests",
            Some(origin.to_string()),
            None,
        ));
    }

    if MUTABLE_GLOBAL_RE.is_match(text) || lower.contains("global ") || text.contains("static mut")
    {
        signals.push(signal(
            "mutable-global",
            "medium",
            SignalDecision::Review,
            vec![format!("{origin} contains mutable global state markers")],
            "move mutable globals behind explicit constructors or immutable state snapshots",
            Some(origin.to_string()),
            None,
        ));
    }

    signals
}

fn env_signal(origin: &str, line_number: usize, line: &str) -> Option<SliceSignal> {
    let names = extract_env_names(line);
    if names.is_empty() {
        return None;
    }
    Some(signal(
        "env-reference",
        "low",
        SignalDecision::Review,
        vec![
            format!("{origin}:{line_number}"),
            format!("env vars referenced: {}", names.join(", ")),
        ],
        "check presence only and never print env values",
        Some(origin.to_string()),
        Some(line_number),
    ))
}

fn signal(
    kind: &str,
    severity: &str,
    decision: SignalDecision,
    evidence: Vec<String>,
    recommendation: &str,
    path: Option<String>,
    line: Option<usize>,
) -> SliceSignal {
    SliceSignal {
        kind: kind.to_string(),
        severity: severity.to_string(),
        decision: decision_label(decision).to_string(),
        evidence,
        recommendation: recommendation.to_string(),
        path,
        line,
    }
}

fn decision_label(decision: SignalDecision) -> &'static str {
    match decision {
        SignalDecision::Block => "block",
        SignalDecision::Review => "review",
        SignalDecision::Pass => "pass",
    }
}

fn collect_env_names(text: &str, names: &mut BTreeSet<String>) {
    for line in text.lines() {
        for name in extract_env_names(line) {
            names.insert(name);
        }
    }
}

fn collect_env_names_from_slice(slice: &MigrationSlice, names: &mut BTreeSet<String>) {
    for value in slice
        .allowed_paths
        .iter()
        .chain(slice.forbidden_paths.iter())
        .chain(slice.contracts.iter())
        .chain(slice.tests.iter())
        .chain(slice.proof_lanes.iter())
        .chain(slice.rollback_notes.iter())
        .chain(slice.cutover_notes.as_deref().unwrap_or(&[]))
        .chain(slice.notes.iter())
    {
        collect_env_names(value, names);
    }
}

fn slice_strings(slice: &MigrationSlice) -> String {
    let mut text = String::new();
    text.push_str(&slice.slice_id);
    text.push('\n');
    text.push_str(&slice.owner);
    text.push('\n');
    text.push_str(&slice.status);
    text.push('\n');
    text.push_str(&slice.risk_level);
    text.push('\n');
    for item in &slice.allowed_paths {
        text.push_str(item);
        text.push('\n');
    }
    for item in &slice.forbidden_paths {
        text.push_str(item);
        text.push('\n');
    }
    for item in &slice.contracts {
        text.push_str(item);
        text.push('\n');
    }
    for item in &slice.tests {
        text.push_str(item);
        text.push('\n');
    }
    for item in &slice.proof_lanes {
        text.push_str(item);
        text.push('\n');
    }
    for item in &slice.rollback_notes {
        text.push_str(item);
        text.push('\n');
    }
    if let Some(cutover) = &slice.cutover_notes {
        for item in cutover {
            text.push_str(item);
            text.push('\n');
        }
    }
    if let Some(notes) = &slice.notes {
        text.push_str(notes);
        text.push('\n');
    }
    text
}

struct SliceScanScope {
    files: Vec<PathBuf>,
    missing_paths: Vec<String>,
}

fn slice_scoped_files(repo: &Path, slice: &MigrationSlice) -> Result<SliceScanScope> {
    let mut files = Vec::new();
    let mut prefixes = slice
        .allowed_paths
        .iter()
        .map(|path| normalize_rel(path))
        .collect::<Result<Vec<_>>>()?;
    if prefixes.is_empty() {
        prefixes.push(String::new());
    }
    let all_sources = candidate_source_files(repo)?;
    let mut matched_prefixes = BTreeSet::new();
    for path in all_sources {
        let rel = path
            .strip_prefix(repo)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if let Some(prefix) = prefixes.iter().find(|prefix| {
            prefix.is_empty() || rel == **prefix || rel.starts_with(&format!("{prefix}/"))
        }) {
            if !prefix.is_empty() {
                matched_prefixes.insert(prefix.clone());
            }
            files.push(path);
        }
    }
    let missing_paths = prefixes
        .iter()
        .filter(|prefix| !prefix.is_empty() && !matched_prefixes.contains(*prefix))
        .cloned()
        .collect();
    Ok(SliceScanScope {
        files,
        missing_paths,
    })
}

fn candidate_source_files(repo: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(repo)
        .into_iter()
        .filter_entry(|entry| !entry.file_type().is_dir() || !is_skipped_dir(entry.path()))
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if entry.file_type().is_dir() {
            continue;
        }
        if !is_candidate_source(path) || is_ignored_path(path) {
            continue;
        }
        files.push(path.to_path_buf());
    }
    Ok(files)
}

fn is_candidate_source(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default(),
        "rs" | "py"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "java"
            | "kt"
            | "go"
            | "mjs"
            | "cjs"
            | "toml"
            | "json"
            | "yaml"
            | "yml"
            | "md"
            | "txt"
    )
}

fn is_skipped_dir(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(part) => {
            matches!(
                part.to_string_lossy().as_ref(),
                ".git" | "target" | "node_modules" | "dist" | "build"
            )
        }
        _ => false,
    })
}

fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(part) => {
            matches!(part.to_string_lossy().as_ref(), "generated" | "vendor")
        }
        _ => false,
    })
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn extract_env_names(line: &str) -> Vec<String> {
    let mut names = BTreeSet::new();
    for cap in ENV_NAME_RE.captures_iter(line) {
        names.insert(cap["name"].to_string());
    }
    for cap in PROSE_ENV_RE.captures_iter(line) {
        names.insert(cap["name"].to_string());
    }
    names.into_iter().collect()
}

fn classify_secret_reference(
    is_docs: bool,
    line: &str,
) -> (&'static str, SignalDecision, &'static str) {
    let lower = line.to_ascii_lowercase();
    if is_docs
        || !extract_env_names(line).is_empty()
        || contains_any(
            &lower,
            &[
                "required",
                "presence",
                "prereq",
                "prerequisite",
                "os.getenv",
                "env::var",
                "process.env",
            ],
        )
    {
        (
            "low",
            SignalDecision::Review,
            "track signing or secret prerequisites with presence-only env checks and never print values",
        )
    } else {
        (
            "high",
            SignalDecision::Block,
            "keep hardcoded signing and secret material out of the slice boundary; use presence-only env checks",
        )
    }
}

fn secret_reference_context(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    !extract_env_names(line).is_empty()
        || contains_any(
            &lower,
            &[
                "key",
                "secret",
                "sign",
                "required",
                "presence",
                "prereq",
                "prerequisite",
                "os.getenv",
                "env::var",
                "process.env",
            ],
        )
}

fn normalize_rel(path: &str) -> Result<String> {
    if path.trim().is_empty() {
        bail!("path must not be empty");
    }
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        bail!("absolute paths are not allowed: `{path}`");
    }
    let mut parts = Vec::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("path traversal is not allowed: `{path}`");
            }
        }
    }
    Ok(parts.join("/"))
}

fn canonicalize_repo(repo: &Path) -> Result<PathBuf> {
    fs::canonicalize(repo).with_context(|| format!("canonicalize {}", repo.display()))
}

fn resolve_repo_relative_existing(repo: &Path, value: &str) -> Result<PathBuf> {
    let normalized = normalize_rel(value)?;
    let resolved = repo.join(&normalized);
    let canonical = fs::canonicalize(&resolved)
        .with_context(|| format!("canonicalize {}", resolved.display()))?;
    if !canonical.starts_with(repo) {
        bail!("path escapes repo root: `{value}`");
    }
    Ok(canonical)
}

fn is_comment_only(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }
    let prefixes = ["//", "#", "/*", "*", "--", ";", "%", "<!--"];
    prefixes.iter().any(|prefix| trimmed.starts_with(prefix))
}

fn render_markdown(report: &SliceRiskReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Migration Slice Risk");
    let _ = writeln!(out);
    let _ = writeln!(out, "- repo: `{}`", report.repo);
    let _ = writeln!(out, "- plan: `{}`", report.plan);
    let _ = writeln!(out, "- slice id: `{}`", report.slice_id);
    let _ = writeln!(out, "- decision: `{}`", report.decision);
    let _ = writeln!(out, "- risk level: `{}`", report.risk_level);
    let _ = writeln!(out, "- signals: `{}`", report.signals_total);
    let _ = writeln!(out);
    if !report.env_checks.is_empty() {
        let _ = writeln!(out, "## Environment Presence");
        for env in &report.env_checks {
            let _ = writeln!(out, "- `{}`: `{}`", env.name, env.present);
        }
        let _ = writeln!(out);
    }
    for signal in &report.signals {
        let _ = writeln!(out, "## `{}`", signal.kind);
        let _ = writeln!(out, "- severity: `{}`", signal.severity);
        let _ = writeln!(out, "- decision: `{}`", signal.decision);
        if let Some(path) = &signal.path {
            let _ = writeln!(out, "- path: `{}`", path);
        }
        if let Some(line) = signal.line {
            let _ = writeln!(out, "- line: `{}`", line);
        }
        let _ = writeln!(out, "- evidence: `{}`", signal.evidence.join(" | "));
        let _ = writeln!(out, "- recommendation: {}", signal.recommendation);
        let _ = writeln!(out);
    }
    out
}
