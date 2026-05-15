//! Smart scan mode: after a clean full scan, scope subsequent audits to only
//! the files reported by `git status`. Full scans are forced by a timer, a
//! roulette check, HEAD-drift detection, or explicit `--full`.
//!
//! The decision is purely advisory to the caller — it returns a
//! [`SmartScanDecision`] and the caller decides how to proceed.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const STATE_REL_PATH: &str = "target/jankurai/audit-state.json";
const STATE_SCHEMA_VERSION: &str = "1.0.0";

// ── public types ─────────────────────────────────────────────────────────────

/// Persisted result of the most recent successful full-scope audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartScanState {
    pub schema_version: String,
    /// Unix seconds when the full scan completed.
    pub last_full_scan_at: i64,
    /// Short git commit SHA at the time of the full scan.
    pub last_full_scan_commit: String,
    /// Hard-finding count from the full scan (0 = clean).
    pub last_full_hard_findings: usize,
    /// Caps applied during the full scan (empty = clean).
    pub last_full_caps: Vec<String>,
    /// Auditor version that produced the state (rules may change between versions).
    pub last_full_auditor_version: String,
}

/// Caller-provided knobs for the decision logic.
#[derive(Debug, Clone)]
pub struct SmartScanConfig {
    /// When false the caller requested an explicit full scan.
    pub enabled: bool,
    /// Force full scan after this many seconds since the last one.  0 = timer disabled.
    pub interval_secs: u64,
    /// Fraction of sessions that run a full scan regardless (0.0–1.0).
    pub roulette_rate: f64,
}

impl Default for SmartScanConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 3600,
            roulette_rate: 0.10,
        }
    }
}

/// What the caller should do for this audit invocation.
pub enum SmartScanDecision {
    /// Run a complete inventory + copy-code audit.
    Full { reason: &'static str },
    /// Scope the audit to these changed paths (via `--changed-fast` machinery).
    Fast { paths: Vec<PathBuf> },
    /// Git status is empty and last full scan is still valid — nothing to do.
    Skip,
}

// ── public API ───────────────────────────────────────────────────────────────

/// Decide the scan scope for a no-explicit-scope invocation of `jankurai audit`.
///
/// Returns [`SmartScanDecision::Full`] for the first run, after a version change,
/// after HEAD moves, after a dirty full scan, when the timer fires, or on the
/// random periodic check. Returns [`SmartScanDecision::Fast`] when the state is
/// clean and git reports a small set of changes. Returns [`SmartScanDecision::Skip`]
/// when git status is empty and the state is valid.
pub fn decide(root: &Path, config: &SmartScanConfig) -> Result<SmartScanDecision> {
    if !config.enabled {
        return Ok(SmartScanDecision::Full { reason: "--full requested" });
    }
    let state = match load_state(root) {
        Some(s) => s,
        None => return Ok(SmartScanDecision::Full { reason: "no prior state" }),
    };
    let head = current_commit(root);
    if head.as_deref() != Some(state.last_full_scan_commit.as_str()) {
        return Ok(SmartScanDecision::Full { reason: "HEAD moved since last full scan" });
    }
    if state.last_full_hard_findings > 0 || !state.last_full_caps.is_empty() {
        return Ok(SmartScanDecision::Full { reason: "prior scan had findings" });
    }
    if state.last_full_auditor_version != crate::model::AUDITOR_VERSION {
        return Ok(SmartScanDecision::Full { reason: "auditor version changed" });
    }
    if config.interval_secs > 0 && elapsed_since(state.last_full_scan_at) > config.interval_secs {
        return Ok(SmartScanDecision::Full { reason: "interval elapsed" });
    }
    if roulette(config.roulette_rate) {
        return Ok(SmartScanDecision::Full { reason: "periodic check" });
    }
    let changed = git_status_changed_files(root)?;
    if changed.is_empty() {
        return Ok(SmartScanDecision::Skip);
    }
    Ok(SmartScanDecision::Fast { paths: changed })
}

/// Persist a clean-state record after a successful full audit.
///
/// Silently skips on I/O failure — the worst outcome is an unnecessary full scan
/// on the next invocation.
pub fn save_state(root: &Path, report: &crate::model::Report) -> Result<()> {
    let hard_findings = report
        .decision
        .as_ref()
        .map(|d| d.hard_findings)
        .unwrap_or(0);
    let commit = report
        .git
        .as_ref()
        .and_then(|g| g.head.clone())
        .unwrap_or_default();
    let state = SmartScanState {
        schema_version: STATE_SCHEMA_VERSION.into(),
        last_full_scan_at: now_unix_secs(),
        last_full_scan_commit: commit,
        last_full_hard_findings: hard_findings,
        last_full_caps: report.caps_applied.clone(),
        last_full_auditor_version: crate::model::AUDITOR_VERSION.into(),
    };
    let path = root.join(STATE_REL_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&state)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Return all modified, added, renamed, or untracked files from `git status`.
pub fn git_status_changed_files(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-u"])
        .current_dir(root)
        .output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut paths = Vec::new();
    for line in text.lines() {
        if line.len() < 4 {
            continue;
        }
        let xy = &line[..2];
        // Skip entries where the file was deleted in both index and worktree.
        if xy == "DD" || xy.starts_with('D') && xy.ends_with('D') {
            continue;
        }
        let rest = line[3..].trim();
        // Rename entries: "R  old -> new" — take the new path after " -> ".
        let path_str = if let Some(arrow) = rest.find(" -> ") {
            &rest[arrow + 4..]
        } else {
            rest
        };
        // Strip quotes that git adds for paths with special characters.
        let path_str = path_str.trim_matches('"');
        if !path_str.is_empty() {
            paths.push(PathBuf::from(path_str));
        }
    }
    Ok(paths)
}

// ── private helpers ───────────────────────────────────────────────────────────

fn load_state(root: &Path) -> Option<SmartScanState> {
    let path = root.join(STATE_REL_PATH);
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

fn current_commit(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn elapsed_since(then: i64) -> u64 {
    let now = now_unix_secs();
    if now > then { (now - then) as u64 } else { 0 }
}

/// Returns true approximately `rate * 100`% of the time, using sub-second
/// nanoseconds as a low-cost entropy source.
fn roulette(rate: f64) -> bool {
    if rate <= 0.0 {
        return false;
    }
    if rate >= 1.0 {
        return true;
    }
    let threshold = (rate * 100.0) as u32;
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    ns % 100 < threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roulette_rate_zero_never_fires() {
        for _ in 0..200 {
            assert!(!roulette(0.0));
        }
    }

    #[test]
    fn roulette_rate_one_always_fires() {
        for _ in 0..200 {
            assert!(roulette(1.0));
        }
    }

    #[test]
    fn elapsed_since_past_gives_positive() {
        let past = now_unix_secs() - 100;
        assert!(elapsed_since(past) >= 100);
    }

    #[test]
    fn elapsed_since_future_gives_zero() {
        let future = now_unix_secs() + 9999;
        assert_eq!(elapsed_since(future), 0);
    }
}
