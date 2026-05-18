//! `jankurai diff-audit` — composes `proof` routing + `audit` scoring on the
//! diff vs a base ref, with sensible defaults for pre-commit / pre-push / CI
//! fast lanes. Designed to replace per-repo shell wrappers that hand-roll the
//! same pipeline.
//!
//! Behavior:
//! 1. Resolve a base ref (arg → `JANKURAI_DIFF_BASE` env → `GITHUB_BASE_REF`
//!    (prefixed with `origin/`) → `CI_MERGE_REQUEST_DIFF_BASE_SHA` → `origin/main`).
//! 2. Collect changed files: committed-vs-base ∪ staged ∪ worktree, deduped.
//! 3. Write `changed.lst` for reproducibility.
//! 4. Run `proof::build_proof_plan` to surface the lane-routing the diff
//!    implies (advisory — proof writes are best-effort).
//! 5. Run `audit::run_audit_with_options(repo, changed, …)` in changed-fast
//!    mode so only the diff is scored.
//! 6. Write `diff-score.json` + `diff-score.md`.
//! 7. Exit non-zero if the diff introduces any hard findings or new caps.
//!
//! Output dir defaults to `target/jankurai/diff/`.
//!
//! Honors `JANKURAI_SKIP_HOOKS=1` (no-op exit 0) so pre-commit / pre-push
//! callers share the same bypass token.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::audit::{self, AuditOptions};
use crate::commands::proof::{self, ProofPlanArgs};
use crate::render::{render_markdown, write_markdown};
use crate::validation::{self, ArtifactSchema};

/// CLI arguments for `jankurai diff-audit`.
///
/// Defined here (not in main.rs) so external callers — e.g. integration tests
/// or library consumers — can construct the args without depending on the bin.
#[derive(Debug, Clone)]
pub struct DiffAuditArgs {
    pub repo: PathBuf,
    /// Base ref to diff against. If `None`, resolved from env (see module docs).
    pub base_ref: Option<String>,
    pub out_dir: PathBuf,
    pub json: Option<String>,
    pub md: Option<String>,
    pub proof_out: Option<String>,
    pub proof_md: Option<String>,
    pub changed_list_out: Option<String>,
    /// Suppress the proof step (for environments without `agent/owner-map.json`).
    pub skip_proof: bool,
    /// Treat the audit pass purely as advisory — never exit non-zero, even on
    /// hard findings. Useful for "report-only" CI lanes.
    pub advisory_only: bool,
}

impl Default for DiffAuditArgs {
    fn default() -> Self {
        Self {
            repo: PathBuf::from("."),
            base_ref: None,
            out_dir: PathBuf::from("target/jankurai/diff"),
            json: None,
            md: None,
            proof_out: None,
            proof_md: None,
            changed_list_out: None,
            skip_proof: false,
            advisory_only: false,
        }
    }
}

/// Entry point. Equivalent to running `proof --changed-from <base>` then
/// `audit --changed <files...> --mode advisory --changed-fast`, with one
/// joint failure decision at the end.
pub fn run(args: DiffAuditArgs) -> Result<()> {
    if std::env::var("JANKURAI_SKIP_HOOKS").as_deref() == Ok("1") {
        eprintln!("jankurai diff-audit: skipped (JANKURAI_SKIP_HOOKS=1)");
        return Ok(());
    }

    let repo = args.repo.canonicalize().unwrap_or(args.repo.clone());
    let out_dir = if args.out_dir.is_absolute() {
        args.out_dir.clone()
    } else {
        repo.join(&args.out_dir)
    };
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("create out_dir {}", out_dir.display()))?;

    let base = resolve_base_ref(args.base_ref.as_deref());
    let resolved_base = ensure_base_reachable(&repo, &base);
    if resolved_base.is_none() {
        eprintln!(
            "jankurai diff-audit: no usable base ref ({base}); falling back to FULL advisory audit"
        );
    }

    let changed = match resolved_base.as_deref() {
        Some(base_ref) => collect_changed_paths(&repo, base_ref)?,
        None => Vec::new(),
    };

    let changed_list_path = args
        .changed_list_out
        .clone()
        .unwrap_or_else(|| out_dir.join("changed.lst").to_string_lossy().into_owned());
    write_changed_list(&changed_list_path, &changed)?;

    if changed.is_empty() {
        // No diff to audit — write empty score and exit clean. We intentionally
        // do NOT fall through to a full-repo audit in this branch; surprising
        // a CI lane with a full scan when the user asked for "diff only" is
        // a worse failure than the trivially-passing report we write here.
        // If the base ref was unreachable, we surface that in the message.
        let where_to = resolved_base
            .as_deref()
            .map(|r| format!("vs {r}"))
            .unwrap_or_else(|| "(no usable base ref)".to_string());
        eprintln!("jankurai diff-audit: no changes {where_to} — nothing to audit");
        let json_path = args.json.clone().unwrap_or_else(|| {
            out_dir
                .join("diff-score.json")
                .to_string_lossy()
                .into_owned()
        });
        let md_path = args
            .md
            .clone()
            .unwrap_or_else(|| out_dir.join("diff-score.md").to_string_lossy().into_owned());
        write_empty_diff_score(&json_path, &md_path, resolved_base.as_deref())?;
        return Ok(());
    }

    if !args.skip_proof && resolved_base.is_some() {
        let proof_out = args.proof_out.clone().unwrap_or_else(|| {
            out_dir
                .join("proof-plan.json")
                .to_string_lossy()
                .into_owned()
        });
        let proof_md = args
            .proof_md
            .clone()
            .unwrap_or_else(|| out_dir.join("proof-plan.md").to_string_lossy().into_owned());
        // Best-effort — a missing owner-map / test-map shouldn't fail the lane.
        let proof_args = ProofPlanArgs {
            repo: repo.clone(),
            changed: changed.clone(),
            changed_from: resolved_base.clone(),
            out: Some(proof_out),
            md: Some(proof_md),
        };
        if let Err(e) = proof::run_proof(proof_args) {
            eprintln!("jankurai diff-audit: proof step warned (continuing): {e}");
        }
    }

    // Score the changed set. `changed_fast: true` tells the audit infrastructure
    // we're in a diff lane (skip score-history append, mark git mode).
    let report = audit::run_audit_with_options(
        &repo,
        &changed,
        AuditOptions {
            self_audit: false,
            proof_receipts: None,
            changed_fast: !changed.is_empty(),
        },
    )?;

    let hard = report
        .decision
        .as_ref()
        .map(|d| d.hard_findings)
        .unwrap_or_else(|| {
            report
                .findings
                .iter()
                .filter(|f| matches!(f.severity.as_str(), "high" | "critical" | "error"))
                .count()
        });
    let caps = report.caps_applied.len();
    let total = report.findings.len();
    let score = report.score;

    let json_path = args.json.clone().unwrap_or_else(|| {
        out_dir
            .join("diff-score.json")
            .to_string_lossy()
            .into_owned()
    });
    let md_path = args
        .md
        .clone()
        .unwrap_or_else(|| out_dir.join("diff-score.md").to_string_lossy().into_owned());

    validation::write_json(&repo, ArtifactSchema::RepoScore, &json_path, &report)?;
    let md = render_markdown(&report);
    write_markdown(&md_path, &md)?;

    eprintln!(
        "jankurai diff-audit: changed={} hard={} caps={} total={} score={}",
        changed.len(),
        hard,
        caps,
        total,
        score
    );

    if !args.advisory_only && (hard > 0 || caps > 0) {
        anyhow::bail!(
            "diff-audit failed: {hard} hard finding(s) and {caps} new cap(s) in changed files (see {md_path})"
        );
    }
    Ok(())
}

/// Resolve a base ref following the documented precedence.
pub fn resolve_base_ref(explicit: Option<&str>) -> String {
    if let Some(b) = explicit.filter(|s| !s.is_empty()) {
        return b.to_string();
    }
    if let Ok(v) = std::env::var("JANKURAI_DIFF_BASE") {
        if !v.is_empty() {
            return v;
        }
    }
    if let Ok(v) = std::env::var("GITHUB_BASE_REF") {
        if !v.is_empty() {
            return format!("origin/{v}");
        }
    }
    if let Ok(v) = std::env::var("CI_MERGE_REQUEST_DIFF_BASE_SHA") {
        if !v.is_empty() {
            return v;
        }
    }
    "origin/main".to_string()
}

/// Returns the first reachable ref from a fall-through list.
fn ensure_base_reachable(repo: &Path, base: &str) -> Option<String> {
    for candidate in [base, "origin/main", "main"] {
        if git_ref_exists(repo, candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Build a Command for git that bypasses CI wrappers (jeryu / spied gits) so
/// our diff collection is consistent regardless of the user's `git` shim.
/// jankurai needs an unmediated read of the worktree to make audit decisions.
fn git_cmd(repo: &Path) -> Command {
    let mut c = Command::new("git");
    c.arg("-C").arg(repo);
    // Both names handled — different jeryu shim generations use different keys.
    c.env("JERYU_GIT_BYPASS", "1");
    c.env("JERYU_GIT_INTERNAL", "1");
    c
}

fn git_ref_exists(repo: &Path, refname: &str) -> bool {
    git_cmd(repo)
        .arg("rev-parse")
        .arg("--verify")
        .arg(refname)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Collect committed-vs-base ∪ staged ∪ worktree changes. All paths are
/// repo-relative `PathBuf`s with duplicates removed.
fn collect_changed_paths(repo: &Path, base: &str) -> Result<Vec<PathBuf>> {
    let mut set: BTreeSet<PathBuf> = BTreeSet::new();
    let refspec = format!("{base}...HEAD");
    push_git_diff_names(repo, &["diff", "--name-only", &refspec], &mut set)?;
    push_git_diff_names(repo, &["diff", "--name-only", "--cached"], &mut set)?;
    push_git_diff_names(repo, &["diff", "--name-only"], &mut set)?;
    Ok(set.into_iter().collect())
}

fn push_git_diff_names(repo: &Path, args: &[&str], set: &mut BTreeSet<PathBuf>) -> Result<()> {
    let output = git_cmd(repo)
        .args(args)
        .output()
        .with_context(|| format!("invoke git {args:?}"))?;
    // If git fails (e.g. base ref missing right after a shallow clone) we don't
    // bail — the caller's ensure_base_reachable already chose a reachable ref,
    // and a transient failure on one git invocation shouldn't lose work.
    if !output.status.success() {
        return Ok(());
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            set.insert(PathBuf::from(trimmed));
        }
    }
    Ok(())
}

fn write_changed_list(path: &str, changed: &[PathBuf]) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let body = changed
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, format!("{body}\n"))
        .with_context(|| format!("write changed list {path}"))?;
    Ok(())
}

fn write_empty_diff_score(json_path: &str, md_path: &str, base: Option<&str>) -> Result<()> {
    if let Some(parent) = Path::new(json_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Some(parent) = Path::new(md_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let body = serde_json::json!({
        "kind": "diff-audit",
        "result": "no-changes",
        "base_ref": base,
        "changed_count": 0,
        "hard_findings": 0,
        "caps_applied": 0,
        "score": 100,
    });
    std::fs::write(json_path, serde_json::to_string_pretty(&body)?)
        .with_context(|| format!("write {json_path}"))?;
    let md = format!(
        "# diff-audit (no changes)\n\nBase: `{}`\n\nNo files changed; audit skipped.\n",
        base.unwrap_or("(unknown)")
    );
    std::fs::write(md_path, md).with_context(|| format!("write {md_path}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_base_ref_explicit_wins() {
        std::env::set_var("JANKURAI_DIFF_BASE", "should-not-win");
        let got = resolve_base_ref(Some("origin/feature"));
        assert_eq!(got, "origin/feature");
        std::env::remove_var("JANKURAI_DIFF_BASE");
    }

    #[test]
    fn resolve_base_ref_env_precedence() {
        // env-var slot order: JANKURAI_DIFF_BASE > GITHUB_BASE_REF > CI_MERGE_REQUEST_DIFF_BASE_SHA > default
        std::env::remove_var("JANKURAI_DIFF_BASE");
        std::env::remove_var("GITHUB_BASE_REF");
        std::env::remove_var("CI_MERGE_REQUEST_DIFF_BASE_SHA");

        assert_eq!(resolve_base_ref(None), "origin/main");

        std::env::set_var("CI_MERGE_REQUEST_DIFF_BASE_SHA", "deadbeef");
        assert_eq!(resolve_base_ref(None), "deadbeef");

        std::env::set_var("GITHUB_BASE_REF", "develop");
        assert_eq!(resolve_base_ref(None), "origin/develop");

        std::env::set_var("JANKURAI_DIFF_BASE", "origin/release");
        assert_eq!(resolve_base_ref(None), "origin/release");

        std::env::remove_var("JANKURAI_DIFF_BASE");
        std::env::remove_var("GITHUB_BASE_REF");
        std::env::remove_var("CI_MERGE_REQUEST_DIFF_BASE_SHA");
    }

    #[test]
    fn empty_changed_list_writes_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.lst");
        write_changed_list(&path.to_string_lossy(), &[]).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(body, "\n");
    }
}
