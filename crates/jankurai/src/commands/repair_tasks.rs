//! Repair-task bank generation.
//!
//! The `jankurai repair-tasks` command runs the existing full audit engine over
//! a single repo and projects its findings into a structured, deduplicated
//! repair-task feed that downstream agents (Jekko/ZYAL) consume. It composes the
//! audit engine (it does not reimplement any scoring or detection logic): each
//! task is derived from already-computed report fields — copy-code classes,
//! tool-adoption opportunities, and hard structure/ownership findings — and is
//! given a stable SHA fingerprint id so the feed diffs cleanly across runs.

use crate::audit::{run_audit_with_options, AuditOptions};
use crate::model::Report;
use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

/// Arguments for the repair-tasks command.
#[derive(Debug, Clone)]
pub struct RepairTasksArgs {
    /// Repo to audit and convert into a repair-task feed.
    pub repo: PathBuf,
    /// Optional output file; when absent the feed is written to stdout.
    pub out: Option<String>,
    /// Output format: `json` (default) or `md`.
    pub format: String,
}

/// Entry point for `jankurai repair-tasks`.
pub fn run(_args: RepairTasksArgs) -> Result<()> {
    // Implemented in the follow-up commit.
    let _ = build_task_bank;
    Ok(())
}

/// Placeholder; the real projection lands in the implementation commit.
fn build_task_bank(_repo: &str, _report: &Report) -> RepairTaskBank {
    RepairTaskBank {
        repo: String::new(),
        generated_at_note: String::new(),
        tasks: Vec::new(),
        totals: RepairTaskTotals::default(),
    }
}

/// The full repair-task feed: the data contract consumed by repair agents.
#[derive(Debug, Clone, Serialize)]
pub struct RepairTaskBank {
    pub repo: String,
    pub generated_at_note: String,
    pub tasks: Vec<RepairTask>,
    pub totals: RepairTaskTotals,
}

/// One actionable repair task projected from the audit report.
#[derive(Debug, Clone, Serialize)]
pub struct RepairTask {
    pub id: String,
    pub kind: RepairTaskKind,
    pub target_paths: Vec<String>,
    pub instances: usize,
    pub rationale: String,
    pub suggested_action: String,
    pub est_effort: String,
    pub score_delta_if_fixed: i32,
    pub finding_fingerprints: Vec<String>,
}

/// The category of repair work a task represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairTaskKind {
    Dedup,
    Variety,
    ToolAdoption,
}

/// Aggregate counts across the feed.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RepairTaskTotals {
    pub task_count: usize,
    pub dedup_tasks: usize,
    pub variety_tasks: usize,
    pub tool_adoption_tasks: usize,
    pub total_instances: usize,
    pub total_score_delta_if_fixed: i32,
}

#[allow(dead_code)]
fn audit_repo(repo: &std::path::Path) -> Result<Report> {
    run_audit_with_options(repo, &[], AuditOptions::default())
}
