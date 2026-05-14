//! `jankurai audit-file` — audits a single candidate file change and prints a
//! delta-based save-gate decision. Built for the Jankurai Guard hook: a guarded
//! filesystem calls this on every agent write so failures are caught and
//! surfaced before (or the instant after) the bytes land.

use crate::audit::fs::OverlayOp;
use crate::audit::save_gate::{
    evaluate, SaveGateDecision, SaveGateMode, SaveGateRequest, SaveGateVerdict,
};
use crate::model::Finding;
use anyhow::{anyhow, bail, Context, Result};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Arguments for `jankurai audit-file`.
#[derive(clap::Args, Debug)]
pub struct AuditFileArgs {
    /// Repo root.
    #[arg(default_value = ".")]
    pub repo: PathBuf,
    /// Repo-relative path of the candidate file being saved.
    #[arg(long, value_name = "RELPATH")]
    pub path: String,
    /// Candidate bytes source: a file path, or `-` for stdin. Omit to audit the
    /// file currently on disk at `--path`.
    #[arg(long, value_name = "FILE|-")]
    pub candidate: Option<String>,
    /// The change being applied.
    #[arg(long, default_value = "modify", value_parser = ["create", "modify", "delete", "rename"])]
    pub op: String,
    /// For `--op rename`: the previous repo-relative path.
    #[arg(long, value_name = "RELPATH")]
    pub rename_from: Option<String>,
    /// Last-good content for the delta comparison. Omit to use the on-disk file.
    #[arg(long, value_name = "FILE")]
    pub baseline: Option<String>,
    /// `save-gate` blocks on new hard findings; `advisory` only reports.
    #[arg(long, default_value = "save-gate", value_parser = ["save-gate", "advisory"])]
    pub mode: String,
    /// Output format.
    #[arg(long, default_value = "agent", value_parser = ["agent", "json"])]
    pub format: String,
    /// Also write the JSON decision to this path.
    #[arg(long, value_name = "PATH")]
    pub json_out: Option<String>,
    /// Audit the tool's own source surface. Only needed when the target repo is
    /// the jankurai repo itself.
    #[arg(long)]
    pub self_audit: bool,
}

/// Runs the save-gate and returns the process exit code: 0 pass, 2 advisory,
/// 3 block, 4 internal error. Internal errors are reported to stderr rather
/// than propagated so the guard hook always gets a stable code.
pub fn run(args: AuditFileArgs) -> Result<i32> {
    match decide(&args) {
        Ok(decision) => {
            emit(&decision, &args)?;
            Ok(decision.exit_code)
        }
        Err(err) => {
            eprintln!("jankurai audit-file: {err:#}");
            Ok(4)
        }
    }
}

/// Resolves arguments into a [`SaveGateRequest`] and evaluates it.
fn decide(args: &AuditFileArgs) -> Result<SaveGateDecision> {
    let root = args
        .repo
        .canonicalize()
        .with_context(|| format!("resolving repo root {}", args.repo.display()))?;
    let rel_path = normalize_rel(&args.path);
    if rel_path.is_empty() {
        bail!("--path must not be empty");
    }
    let op = parse_op(&args.op, args.rename_from.as_deref())?;
    let candidate_bytes = read_candidate(args, &root, &rel_path, &op)?;
    let baseline_bytes = match &args.baseline {
        Some(path) => {
            Some(std::fs::read(path).with_context(|| format!("reading baseline {path}"))?)
        }
        None => None,
    };
    let mode = SaveGateMode::parse(&args.mode)?;
    evaluate(SaveGateRequest {
        root,
        rel_path,
        op,
        candidate_bytes,
        baseline_bytes,
        mode,
        self_audit: args.self_audit,
    })
}

/// Normalizes a repo-relative path to forward slashes with no leading `./`.
fn normalize_rel(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}

/// Parses the `--op` value into an [`OverlayOp`].
fn parse_op(op: &str, rename_from: Option<&str>) -> Result<OverlayOp> {
    match op {
        "create" => Ok(OverlayOp::Create),
        "modify" => Ok(OverlayOp::Modify),
        "delete" => Ok(OverlayOp::Delete),
        "rename" => {
            let from = rename_from.ok_or_else(|| anyhow!("--op rename requires --rename-from"))?;
            Ok(OverlayOp::Rename {
                from: normalize_rel(from),
            })
        }
        other => bail!("unknown --op `{other}`"),
    }
}

/// Reads the candidate bytes from stdin, an explicit file, or the on-disk path.
fn read_candidate(
    args: &AuditFileArgs,
    root: &Path,
    rel_path: &str,
    op: &OverlayOp,
) -> Result<Option<Vec<u8>>> {
    if matches!(op, OverlayOp::Delete) {
        return Ok(None);
    }
    let bytes = match args.candidate.as_deref() {
        Some("-") => {
            let mut buf = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buf)
                .context("reading candidate bytes from stdin")?;
            buf
        }
        Some(file) => {
            std::fs::read(file).with_context(|| format!("reading candidate file {file}"))?
        }
        None => std::fs::read(root.join(rel_path))
            .with_context(|| format!("reading on-disk file {rel_path}"))?,
    };
    Ok(Some(bytes))
}

/// Writes the decision to stdout (and `--json-out` when set).
fn emit(decision: &SaveGateDecision, args: &AuditFileArgs) -> Result<()> {
    let json = serde_json::to_string_pretty(decision)?;
    if let Some(path) = &args.json_out {
        std::fs::write(path, format!("{json}\n"))
            .with_context(|| format!("writing decision JSON to {path}"))?;
    }
    match args.format.as_str() {
        "json" => println!("{json}"),
        _ => print!("{}", render_agent(decision)),
    }
    Ok(())
}

/// Renders the agent-friendly text form of a decision.
fn render_agent(decision: &SaveGateDecision) -> String {
    let mut out = String::new();
    match decision.verdict {
        SaveGateVerdict::Pass => {
            out.push_str(&format!(
                "JANKURAI GUARD: OK  {}  (no new findings)\n",
                decision.path
            ));
            return out;
        }
        SaveGateVerdict::Advisory => {
            out.push_str(&format!("JANKURAI GUARD: ADVISORY  {}\n\n", decision.path));
            out.push_str("New issues on this save (not blocking, fix when you can):\n");
            for finding in &decision.advisory.new_soft_findings {
                push_finding(&mut out, "WARN", finding);
            }
        }
        SaveGateVerdict::Block => {
            out.push_str(&format!("JANKURAI GUARD: BLOCKED  {}\n\n", decision.path));
            out.push_str("This save introduces problems that must be fixed before it can land.\n");
            for finding in &decision.blocking.new_hard_findings {
                push_finding(&mut out, "BLOCK", finding);
            }
            for finding in &decision.blocking.worsened_findings {
                push_finding(&mut out, "WORSENED", finding);
            }
            for finding in &decision.blocking.always_block_findings {
                push_finding(&mut out, "ALWAYS-BLOCK", finding);
            }
        }
    }
    if !decision.preexisting_findings.is_empty() {
        out.push_str(
            "\nPre-existing issues on this file (not caused by this save, not blocking):\n",
        );
        for finding in &decision.preexisting_findings {
            let rule = finding.rule_id.as_deref().unwrap_or(&finding.check_id);
            out.push_str(&format!("  - {rule}  {}\n", finding.problem));
        }
    }
    out.push_str("\nRe-run after fixing:\n  ");
    out.push_str(&decision.rerun_command);
    out.push('\n');
    out
}

/// Appends a single finding block to the agent-readable output.
fn push_finding(out: &mut String, tag: &str, finding: &Finding) {
    let rule = finding.rule_id.as_deref().unwrap_or(&finding.check_id);
    out.push_str(&format!("\n  [{tag}] {rule}  {}\n", finding.problem));
    if let Some(line) = finding.line {
        out.push_str(&format!("    line {line}\n"));
    }
    if !finding.agent_fix.is_empty() {
        out.push_str(&format!("    fix: {}\n", finding.agent_fix));
    }
}
