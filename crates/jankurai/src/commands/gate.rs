//! `jankurai gate` — a BLOCKING pre-commit gate built on the existing audit
//! engine.
//!
//! The gate runs the audit (scoped to staged/changed files when a git worktree
//! is available, otherwise the full repo), then renders a single BLOCK / PASS
//! decision that a pre-commit hook can act on via its exit code. It composes the
//! existing engine — it does not reimplement any scoring or detection. Every
//! blocking reason is derived from already-computed report fields:
//!
//! 1. Hard findings — `report.decision.hard_findings > 0` (critical/high).
//! 2. All-caps / issue markers — the vibe-placeholder, fallback-soup, and
//!    future-hostile dead-language caps (TODO/FIXME/HACK/XXX, error-hiding
//!    fallbacks, and dead-language markers) detected by the audit's own scan
//!    passes. Their presence blocks regardless of the numeric score.
//! 3. Score regression — when a ratchet baseline is supplied, a current score
//!    below the baseline blocks.
//!
//! ## Opt-in, escape hatch, and never-freeze guarantee
//!
//! Blocking is per-repo-after-green. The gate reads `[precommit_gate] blocking`
//! from `agent/audit-policy.toml` (default `false` = advisory). When blocking is
//! off, the gate ALWAYS exits 0 and prints advisory warnings, so it can never
//! freeze a repo that is not yet green. `--blocking` forces blocking on for a
//! single run. The shared bypass token `JANKURAI_SKIP_HOOKS=1` short-circuits to
//! exit 0 with a printed bypass notice.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::audit::{self, AuditOptions};
use crate::model::Report;

/// The three cap keys that represent all-caps section markers and issue markers
/// in product code: vibe placeholders (TODO/FIXME/HACK/XXX/stub/…), error-hiding
/// fallback soup, and future-hostile dead-language markers. The audit engine
/// raises exactly these caps when its scan passes (`scan::todo_hits`,
/// `scan::fallback_hits`, `scan::future_hostile_hits`) find a hit, so the gate
/// detects markers by reusing the engine's decision rather than re-scanning.
pub const MARKER_CAP_KEYS: &[&str] = &[
    "vibe-placeholders-in-product-code",
    "fallback-soup-in-product-code",
    "future-hostile-dead-language-in-product-code",
];

/// CLI arguments for `jankurai gate`.
///
/// Defined here (not in main.rs) so tests and library consumers can construct
/// the args without depending on the bin.
#[derive(Debug, Clone)]
pub struct GateArgs {
    /// Repo to gate.
    pub repo: PathBuf,
    /// Force blocking on for this run, regardless of config.
    pub blocking: bool,
    /// Restrict the audit scope to staged files only (ignore unstaged worktree
    /// changes). When false the gate also considers unstaged changes so a dirty
    /// worktree cannot smuggle markers past the gate.
    pub staged_only: bool,
    /// Optional ratchet baseline JSON; when present a score regression blocks.
    /// Defaults to `agent/baselines/main.repo-score.json` if that file exists.
    pub baseline: Option<String>,
}

impl Default for GateArgs {
    fn default() -> Self {
        Self {
            repo: PathBuf::from("."),
            blocking: false,
            staged_only: false,
            baseline: None,
        }
    }
}

/// Why the gate would block, projected from report fields. Empty means PASS.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GateReasons {
    /// Count of hard (critical/high) findings.
    pub hard_findings: usize,
    /// Marker cap keys present in the report (all-caps / issue markers).
    pub marker_caps: Vec<String>,
    /// `Some((current, baseline))` when the score regressed below baseline.
    pub score_regression: Option<(i32, i32)>,
}

impl GateReasons {
    /// True when any blocking reason is present.
    pub fn any(&self) -> bool {
        self.hard_findings > 0 || !self.marker_caps.is_empty() || self.score_regression.is_some()
    }
}

/// The gate verdict: whether to block, why, and whether blocking was enforced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDecision {
    /// Reasons the gate would block (independent of blocking mode).
    pub reasons: GateReasons,
    /// Whether blocking was enforced for this run.
    pub blocking: bool,
    /// The score observed by the audit.
    pub score: i32,
}

impl GateDecision {
    /// The process exit code: 1 only when blocking is enforced AND there is a
    /// reason to block; otherwise 0 (advisory or clean).
    pub fn exit_code(&self) -> i32 {
        if self.blocking && self.reasons.any() {
            1
        } else {
            0
        }
    }

    /// True when the gate blocks the commit (exit 1).
    pub fn blocked(&self) -> bool {
        self.exit_code() == 1
    }
}

/// Project a report (and optional ratchet baseline score) into gate reasons.
///
/// Pure and total: this is the unit-testable core. `blocking` only affects
/// enforcement, never the reasons, so advisory and blocking runs surface the
/// same diagnostics.
pub fn decide_gate(report: &Report, baseline_score: Option<i32>, blocking: bool) -> GateDecision {
    let hard_findings = report
        .decision
        .as_ref()
        .map(|d| d.hard_findings)
        .unwrap_or_else(|| {
            report
                .findings
                .iter()
                .filter(|f| matches!(f.severity.as_str(), "high" | "critical"))
                .count()
        });

    let marker_caps = report
        .caps_applied
        .iter()
        .filter(|cap| MARKER_CAP_KEYS.contains(&cap.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    let score_regression = baseline_score
        .filter(|&baseline| report.score < baseline)
        .map(|baseline| (report.score, baseline));

    GateDecision {
        reasons: GateReasons {
            hard_findings,
            marker_caps,
            score_regression,
        },
        blocking,
        score: report.score,
    }
}

/// Entry point for `jankurai gate`. Returns the process exit code (0 = pass /
/// advisory, 1 = blocked).
pub fn run(args: GateArgs) -> Result<i32> {
    if std::env::var("JANKURAI_SKIP_HOOKS").as_deref() == Ok("1") {
        println!("jankurai gate: bypassed (JANKURAI_SKIP_HOOKS=1) — commit allowed without audit");
        return Ok(0);
    }

    let repo = args
        .repo
        .canonicalize()
        .unwrap_or_else(|_| args.repo.clone());
    let blocking = args.blocking || config_blocking(&repo);

    // Scope: staged ∪ (optionally) unstaged worktree changes. When no git
    // worktree is reachable, fall back to a full-repo audit.
    let changed = collect_changed_paths(&repo, args.staged_only);
    let changed_fast = !changed.is_empty();
    let report = audit::run_audit_with_options(
        &repo,
        &changed,
        AuditOptions {
            self_audit: false,
            proof_receipts: None,
            changed_fast,
        },
    )
    .with_context(|| format!("audit {}", repo.display()))?;

    let baseline_score = resolve_baseline_score(&repo, args.baseline.as_deref());
    let decision = decide_gate(&report, baseline_score, blocking);

    print_decision(&decision, changed_fast, changed.len());
    Ok(decision.exit_code())
}

/// Human-readable report of the gate decision and the escape-hatch hint.
fn print_decision(decision: &GateDecision, changed_fast: bool, changed_count: usize) {
    let scope = if changed_fast {
        format!("{changed_count} changed file(s)")
    } else {
        "full repo".to_string()
    };
    println!(
        "jankurai gate: scope={scope} score={} blocking={}",
        decision.score, decision.blocking
    );

    let reasons = &decision.reasons;
    if !reasons.any() {
        println!("jankurai gate: clean — no hard findings, markers, or score regression.");
        return;
    }

    if reasons.hard_findings > 0 {
        println!(
            "jankurai gate: {} hard finding(s) (critical/high) in scope.",
            reasons.hard_findings
        );
    }
    if !reasons.marker_caps.is_empty() {
        println!(
            "jankurai gate: all-caps / issue markers present ({}). These are TODO/FIXME/HACK/XXX, error-hiding fallbacks, or dead-language markers in product code.",
            reasons.marker_caps.join(", ")
        );
    }
    if let Some((current, baseline)) = reasons.score_regression {
        println!("jankurai gate: score regression — {current} is below baseline {baseline}.");
    }

    if decision.blocking {
        println!("jankurai gate: BLOCKED. Fix the findings above before committing.");
        println!(
            "jankurai gate: to bypass this once, run with JANKURAI_SKIP_HOOKS=1 (e.g. `JANKURAI_SKIP_HOOKS=1 git commit ...`)."
        );
    } else {
        println!(
            "jankurai gate: advisory only (blocking disabled) — commit allowed. Enable `[precommit_gate] blocking = true` in agent/audit-policy.toml once the repo is green."
        );
    }
}

/// Read `[precommit_gate] blocking` from `agent/audit-policy.toml`. Default is
/// `false` (advisory) so a not-yet-green repo is never frozen.
fn config_blocking(repo: &Path) -> bool {
    #[derive(Debug, Deserialize, Default)]
    struct PolicyFile {
        #[serde(default)]
        precommit_gate: GatePolicy,
    }
    #[derive(Debug, Deserialize, Default)]
    struct GatePolicy {
        #[serde(default)]
        blocking: bool,
    }
    std::fs::read_to_string(repo.join("agent/audit-policy.toml"))
        .ok()
        .and_then(|text| toml::from_str::<PolicyFile>(&text).ok())
        .map(|parsed| parsed.precommit_gate.blocking)
        .unwrap_or(false)
}

/// Resolve the baseline score: from the explicit path, else the conventional
/// `agent/baselines/main.repo-score.json` when present. Returns `None` when no
/// baseline is available (score regression is then never a blocking reason).
fn resolve_baseline_score(repo: &Path, explicit: Option<&str>) -> Option<i32> {
    let path = match explicit {
        Some(p) => PathBuf::from(p),
        None => {
            let default = repo.join("agent/baselines/main.repo-score.json");
            if !default.exists() {
                return None;
            }
            default
        }
    };
    let text = std::fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value
        .get("score")
        .and_then(serde_json::Value::as_i64)
        .and_then(|v| i32::try_from(v).ok())
}

/// Collect staged changes, optionally unioned with unstaged worktree changes.
/// Returns an empty vec when git is unreachable, which signals the caller to run
/// a full-repo audit.
fn collect_changed_paths(repo: &Path, staged_only: bool) -> Vec<PathBuf> {
    let mut set: BTreeSet<PathBuf> = BTreeSet::new();
    push_git_diff_names(repo, &["diff", "--name-only", "--cached"], &mut set);
    if !staged_only {
        push_git_diff_names(repo, &["diff", "--name-only"], &mut set);
    }
    set.into_iter().collect()
}

/// Build a git Command that bypasses jeryu CI shims so the gate reads the real
/// worktree, mirroring `diff_audit`'s unmediated git access.
fn git_cmd(repo: &Path) -> Command {
    let mut c = Command::new("git");
    c.arg("-C").arg(repo);
    c.env("JERYU_GIT_BYPASS", "1");
    c.env("JERYU_GIT_INTERNAL", "1");
    c
}

fn push_git_diff_names(repo: &Path, args: &[&str], set: &mut BTreeSet<PathBuf>) {
    let Ok(output) = git_cmd(repo).args(args).output() else {
        return;
    };
    if !output.status.success() {
        return;
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            set.insert(PathBuf::from(trimmed));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Finding, PolicySummary, Report, ReportDecision, Scope, ToolAdoptionReadiness,
    };

    fn marker_finding(matched_term: &str) -> Finding {
        Finding {
            severity: "high".to_string(),
            category: "vibe".to_string(),
            path: "src/work.rs".to_string(),
            problem: "all-caps issue marker in product code".to_string(),
            agent_fix: "resolve the marker or move the work into a tracked task".to_string(),
            evidence: vec![],
            check_id: format!("vibe:{matched_term}"),
            hardness: "hard".to_string(),
            confidence: 1.0,
            evidence_kind: "static".to_string(),
            rerun_command: String::new(),
            fingerprint: format!("fp-{matched_term}"),
            rule_id: Some("HLT-001-DEAD-MARKER".to_string()),
            tlr: None,
            lane: None,
            docs_url: None,
            owner: None,
            line: Some(12),
            matched_term: Some(matched_term.to_string()),
            reason: None,
        }
    }

    fn policy() -> PolicySummary {
        PolicySummary {
            path: "agent/audit-policy.toml".to_string(),
            minimum_score: 85,
            fail_on: vec!["critical".to_string(), "high".to_string()],
            advisory_on: vec!["medium".to_string(), "low".to_string()],
            ..PolicySummary::default()
        }
    }

    /// Build a minimal Report directly; the model does not derive Deserialize,
    /// so the few non-Default nested types are constructed inline. Mirrors the
    /// `repair_tasks` test fixture so the gate tests need no real repo.
    fn report(score: i32, findings: Vec<Finding>, caps: Vec<String>) -> Report {
        let decision = ReportDecision {
            status: "pass".to_string(),
            minimum_score: 85,
            passed: true,
            hard_findings: findings
                .iter()
                .filter(|f| matches!(f.severity.as_str(), "critical" | "high"))
                .count(),
            soft_findings: 0,
            ratchet: None,
        };
        Report {
            report_fingerprint: String::new(),
            input_fingerprint: String::new(),
            policy_fingerprint: String::new(),
            manifest_fingerprints: Default::default(),
            dirty_worktree: false,
            generated_at: String::new(),
            schema_url: String::new(),
            standard: String::new(),
            standard_version: String::new(),
            auditor_version: String::new(),
            schema_version: String::new(),
            paper_edition: String::new(),
            target_stack_id: String::new(),
            target_stack: String::new(),
            claimed_conformance_level: "HL3".to_string(),
            observed_conformance_level: "HL3".to_string(),
            conformance_decision: String::new(),
            conformance_blockers: vec![],
            repo: String::new(),
            run_id: None,
            started_at: None,
            elapsed_ms: None,
            scope: Scope {
                mode: "changed-fast".to_string(),
                paths: vec![],
            },
            score,
            raw_score: score,
            decision: Some(decision),
            git: None,
            policy: Some(policy()),
            proof_receipts: vec![],
            caps_applied: caps,
            hard_rules: vec![],
            dimensions: vec![],
            ux_qa: crate::model::UxQaReadiness {
                web_surface: false,
                has_rendered_ux_lane: false,
                missing_categories: vec![],
                evidence: serde_json::Value::Null,
                artifact: None,
            },
            tool_adoption: ToolAdoptionReadiness {
                control_plane_present: true,
                applicable_count: 0,
                configured_count: 0,
                ci_evidence_count: 0,
                artifact_verified_count: 0,
                replaced_count: 0,
                items: vec![],
                evidence: serde_json::Value::Null,
                missing: vec![],
            },
            security_evidence: Default::default(),
            boundaries: Default::default(),
            copy_code: None,
            profile_structure: crate::model::ProfileStructureReadiness {
                applicable_count: 0,
                canonical_count: 0,
                noncanonical_count: 0,
                guidance_missing_count: 0,
                cells: vec![],
                evidence: serde_json::Value::Null,
            },
            vibe_coverage: None,
            coverage_evidence: None,
            findings,
            agent_fix_queue: vec![],
        }
    }

    #[test]
    fn clean_report_passes() {
        let decision = decide_gate(&report(96, vec![], vec![]), Some(96), true);
        assert!(!decision.reasons.any(), "clean report has no reasons");
        assert_eq!(
            decision.exit_code(),
            0,
            "clean report exits 0 even blocking"
        );
        assert!(!decision.blocked());
    }

    #[test]
    fn blocking_with_all_caps_marker_blocks() {
        // A staged file with a TODO marker raises the vibe-placeholder cap.
        let rep = report(
            68,
            vec![marker_finding("TODO")],
            vec!["vibe-placeholders-in-product-code".to_string()],
        );
        let decision = decide_gate(&rep, Some(96), true);
        assert_eq!(
            decision.reasons.marker_caps,
            vec!["vibe-placeholders-in-product-code"],
            "the all-caps marker cap is surfaced"
        );
        assert!(
            decision.reasons.hard_findings > 0,
            "marker is a hard finding"
        );
        assert_eq!(decision.exit_code(), 1, "blocking + marker => exit 1");
        assert!(decision.blocked());
    }

    #[test]
    fn advisory_does_not_block_despite_markers() {
        // Same findings, but blocking disabled: never freeze a not-yet-green repo.
        let rep = report(
            68,
            vec![marker_finding("FIXME")],
            vec!["vibe-placeholders-in-product-code".to_string()],
        );
        let decision = decide_gate(&rep, Some(96), false);
        assert!(
            decision.reasons.any(),
            "reasons are still surfaced for the advisory message"
        );
        assert_eq!(decision.exit_code(), 0, "advisory always exits 0");
        assert!(!decision.blocked());
    }

    #[test]
    fn hard_findings_block_when_blocking() {
        let rep = report(50, vec![marker_finding("XXX")], vec![]);
        let decision = decide_gate(&rep, None, true);
        assert!(
            decision.reasons.marker_caps.is_empty(),
            "no cap, but a hard finding"
        );
        assert_eq!(decision.reasons.hard_findings, 1);
        assert_eq!(decision.exit_code(), 1);
    }

    #[test]
    fn score_regression_blocks_when_blocking() {
        // No findings, no markers, but score dropped below the ratchet baseline.
        let rep = report(90, vec![], vec![]);
        let decision = decide_gate(&rep, Some(96), true);
        assert_eq!(
            decision.reasons.score_regression,
            Some((90, 96)),
            "regression captures current and baseline"
        );
        assert_eq!(decision.exit_code(), 1, "regression blocks under blocking");
    }

    #[test]
    fn score_at_or_above_baseline_is_not_a_regression() {
        let rep = report(96, vec![], vec![]);
        let decision = decide_gate(&rep, Some(96), true);
        assert_eq!(decision.reasons.score_regression, None);
        assert_eq!(decision.exit_code(), 0);
    }

    #[test]
    fn missing_baseline_means_no_regression_check() {
        let rep = report(10, vec![], vec![]);
        let decision = decide_gate(&rep, None, true);
        assert_eq!(
            decision.reasons.score_regression, None,
            "without a baseline, a low score alone is not a regression"
        );
        assert_eq!(decision.exit_code(), 0, "no other reasons => pass");
    }

    #[test]
    fn skip_hooks_env_bypasses_gate() {
        // The escape hatch short-circuits run() to exit 0 before any audit.
        std::env::set_var("JANKURAI_SKIP_HOOKS", "1");
        let code = run(GateArgs {
            repo: PathBuf::from("/nonexistent-repo-for-gate-test"),
            blocking: true,
            staged_only: false,
            baseline: None,
        })
        .expect("bypass path never errors");
        std::env::remove_var("JANKURAI_SKIP_HOOKS");
        assert_eq!(code, 0, "JANKURAI_SKIP_HOOKS=1 => exit 0 without auditing");
    }

    #[test]
    fn marker_cap_keys_cover_the_three_marker_caps() {
        assert!(MARKER_CAP_KEYS.contains(&"vibe-placeholders-in-product-code"));
        assert!(MARKER_CAP_KEYS.contains(&"fallback-soup-in-product-code"));
        assert!(MARKER_CAP_KEYS.contains(&"future-hostile-dead-language-in-product-code"));
    }
}
