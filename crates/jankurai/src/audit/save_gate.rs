//! Single-file save-gate: audits one candidate file change and decides whether
//! the write may land. The decision is delta-based — pre-existing repo debt does
//! not block a save; only findings the candidate newly introduces, worsens, or
//! leaves in place at `critical` severity do.

use super::fs::{CandidateOverlay, OverlayOp};
use super::scan;
use super::{run_candidate_audit, AuditOptions, CandidateAuditOptions};
use crate::model::{Finding, Report};
use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// JSON schema identifier emitted with every decision.
pub const SAVE_GATE_SCHEMA: &str = "jankurai-save-gate/1";

/// Whether the gate blocks on new hard findings (`save-gate`) or only reports
/// them (`advisory`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveGateMode {
    /// Block the save when the candidate introduces or worsens a hard finding.
    SaveGate,
    /// Never block; surface findings as advisory only.
    Advisory,
}

impl SaveGateMode {
    /// Parses a mode string.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "save-gate" => Ok(Self::SaveGate),
            "advisory" => Ok(Self::Advisory),
            other => bail!("unknown save-gate mode `{other}` (expected save-gate|advisory)"),
        }
    }

    /// Returns the canonical mode name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SaveGate => "save-gate",
            Self::Advisory => "advisory",
        }
    }
}

/// The outcome class of a save-gate evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SaveGateVerdict {
    /// No new, worsened, or always-block findings — the save may land.
    Pass,
    /// Findings exist but none block the save.
    Advisory,
    /// The candidate introduces or worsens a hard finding, or leaves a
    /// `critical` finding in place — the save is blocked.
    Block,
}

/// Blocking findings, split by why they block.
#[derive(Debug, Clone, Serialize, Default)]
pub struct BlockingFindings {
    /// Hard-severity findings the candidate newly introduces.
    pub new_hard_findings: Vec<Finding>,
    /// Findings whose severity the candidate raises into a hard tier.
    pub worsened_findings: Vec<Finding>,
    /// Pre-existing `critical` findings the candidate leaves in place.
    pub always_block_findings: Vec<Finding>,
}

/// Findings that are surfaced but do not block.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AdvisoryFindings {
    /// Soft-severity findings the candidate newly introduces.
    pub new_soft_findings: Vec<Finding>,
}

/// The full save-gate decision, serialized as the `jankurai-save-gate/1` schema.
#[derive(Debug, Clone, Serialize)]
pub struct SaveGateDecision {
    /// Schema identifier.
    pub schema: &'static str,
    /// Outcome class.
    pub verdict: SaveGateVerdict,
    /// Process exit code: 0 pass, 2 advisory, 3 block.
    pub exit_code: i32,
    /// Repo-relative path of the candidate.
    pub path: String,
    /// The mode the gate ran in.
    pub mode: String,
    /// Score of the repo with the candidate overlaid.
    pub candidate_score: i32,
    /// Score of the repo with the last-good content, when a baseline exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_score: Option<i32>,
    /// One-line human summary.
    pub summary: String,
    /// Findings that block the save.
    pub blocking: BlockingFindings,
    /// Findings surfaced without blocking.
    pub advisory: AdvisoryFindings,
    /// Findings present in both candidate and baseline that do not block.
    pub preexisting_findings: Vec<Finding>,
    /// Command that re-runs this exact check.
    pub rerun_command: String,
}

impl SaveGateDecision {
    /// Returns `true` when the save is blocked.
    pub fn is_block(&self) -> bool {
        self.verdict == SaveGateVerdict::Block
    }
}

/// Inputs to [`evaluate`].
#[derive(Debug, Clone)]
pub struct SaveGateRequest {
    /// Repo root.
    pub root: PathBuf,
    /// Repo-relative path of the candidate (forward-slash normalized).
    pub rel_path: String,
    /// The change being applied.
    pub op: OverlayOp,
    /// Candidate bytes; `None` only for [`OverlayOp::Delete`].
    pub candidate_bytes: Option<Vec<u8>>,
    /// Explicit last-good content; when `None`, the on-disk file is used.
    pub baseline_bytes: Option<Vec<u8>>,
    /// Blocking or advisory.
    pub mode: SaveGateMode,
    /// Audit the tool's own source surface — needed only when the target repo
    /// is the jankurai repo itself.
    pub self_audit: bool,
}

/// Audits a candidate file change and returns a delta-based decision.
pub fn evaluate(req: SaveGateRequest) -> Result<SaveGateDecision> {
    if crate::audit::fs::is_read_only_exception_path(&req.rel_path) {
        return Ok(block_exception_write(&req.rel_path, req.mode.as_str()));
    }
    let scope = vec![req.rel_path.clone()];
    let candidate_overlay = CandidateOverlay {
        rel_path: req.rel_path.clone(),
        op: req.op.clone(),
        bytes: req.candidate_bytes.clone(),
    };
    let (candidate_report, _) = run_candidate_audit(
        &req.root,
        CandidateAuditOptions {
            overlay: candidate_overlay,
            scope_paths: scope.clone(),
            options: audit_options(req.self_audit),
        },
    )?;

    let baseline_report = baseline_report(&req, &scope)?;
    let fail_on = hard_severities(&candidate_report);
    let candidate_findings = findings_for_path(&candidate_report, &req.rel_path);
    let baseline_findings = baseline_report
        .as_ref()
        .map(|r| findings_for_path(r, &req.rel_path))
        .unwrap_or_default();
    let mut candidate_findings = candidate_findings;
    candidate_findings.extend(candidate_todo_comment_findings(&req));
    let mut baseline_findings = baseline_findings;
    baseline_findings.extend(baseline_todo_comment_findings(&req));
    let buckets = classify(&candidate_findings, &baseline_findings, &fail_on);
    Ok(build_decision(
        &req,
        &candidate_report,
        baseline_report.as_ref(),
        buckets,
    ))
}

/// Runs the baseline audit using either the caller-supplied last-good bytes or
/// the on-disk file. Returns `None` for a brand-new path with no prior content.
fn baseline_report(req: &SaveGateRequest, scope: &[String]) -> Result<Option<Report>> {
    let bytes = match &req.baseline_bytes {
        Some(bytes) => Some(bytes.clone()),
        None => std::fs::read(req.root.join(&req.rel_path)).ok(),
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let overlay = CandidateOverlay {
        rel_path: req.rel_path.clone(),
        op: OverlayOp::Modify,
        bytes: Some(bytes),
    };
    let (report, _) = run_candidate_audit(
        &req.root,
        CandidateAuditOptions {
            overlay,
            scope_paths: scope.to_vec(),
            options: audit_options(req.self_audit),
        },
    )?;
    Ok(Some(report))
}

/// Builds the [`AuditOptions`] for a save-gate audit pass.
fn audit_options(self_audit: bool) -> AuditOptions {
    AuditOptions {
        self_audit,
        ..AuditOptions::default()
    }
}

/// Findings the report attributes to exactly this path.
fn findings_for_path(report: &Report, rel_path: &str) -> Vec<Finding> {
    report
        .findings
        .iter()
        .filter(|f| f.path == rel_path)
        .cloned()
        .collect()
}

/// The severities the candidate report's policy treats as hard.
fn hard_severities(report: &Report) -> Vec<String> {
    report
        .policy
        .as_ref()
        .map(|p| p.fail_on.clone())
        .filter(|f| !f.is_empty())
        .unwrap_or_else(|| vec!["critical".to_string(), "high".to_string()])
}

/// Sorts candidate findings against the baseline into delta buckets.
fn classify(candidate: &[Finding], baseline: &[Finding], fail_on: &[String]) -> BucketSet {
    let mut base_map: HashMap<String, i32> = HashMap::new();
    for finding in baseline {
        let rank = severity_rank(&finding.severity);
        base_map
            .entry(delta_key(finding))
            .and_modify(|existing| *existing = (*existing).max(rank))
            .or_insert(rank);
    }
    let mut buckets = BucketSet::default();
    for finding in candidate {
        let key = delta_key(finding);
        let hard = fail_on.iter().any(|s| s == &finding.severity);
        let critical = finding.severity == "critical";
        match base_map.get(&key) {
            None if hard => buckets.blocking.new_hard_findings.push(finding.clone()),
            None => buckets.advisory.new_soft_findings.push(finding.clone()),
            Some(&base_rank) => {
                if hard && severity_rank(&finding.severity) > base_rank {
                    buckets.blocking.worsened_findings.push(finding.clone());
                } else if critical {
                    buckets.blocking.always_block_findings.push(finding.clone());
                } else {
                    buckets.preexisting.push(finding.clone());
                }
            }
        }
    }
    buckets
}

pub fn block_exception_write(rel_path: &str, mode: &str) -> SaveGateDecision {
    let finding = exception_write_finding(rel_path);
    SaveGateDecision {
        schema: SAVE_GATE_SCHEMA,
        verdict: SaveGateVerdict::Block,
        exit_code: 3,
        path: rel_path.into(),
        mode: mode.into(),
        candidate_score: 0,
        baseline_score: None,
        summary: "docs/exceptions is read-only to automation; edit exception records manually and keep the dated front matter reviewable".into(),
        blocking: BlockingFindings {
            new_hard_findings: vec![finding],
            worsened_findings: vec![],
            always_block_findings: vec![],
        },
        advisory: AdvisoryFindings::default(),
        preexisting_findings: vec![],
        rerun_command: "edit the exception file manually, then re-run the original command after human review".into(),
    }
}

fn exception_write_finding(rel_path: &str) -> Finding {
    Finding {
        severity: "high".into(),
        category: "audit".into(),
        path: rel_path.into(),
        problem: "automation is not allowed to write exception records under docs/exceptions"
            .into(),
        agent_fix: "use a manual editor and keep the dated exception front matter current".into(),
        evidence: vec![rel_path.into()],
        check_id: "exception-write-block".into(),
        hardness: "hard".into(),
        confidence: 1.0,
        evidence_kind: "policy".into(),
        rerun_command: String::new(),
        fingerprint: String::new(),
        rule_id: None,
        tlr: None,
        lane: None,
        docs_url: None,
        owner: None,
        line: None,
        matched_term: Some("docs/exceptions".into()),
        reason: Some("exception files are reserved for manual review".into()),
    }
}

/// Intermediate classification result.
#[derive(Debug, Default)]
struct BucketSet {
    blocking: BlockingFindings,
    advisory: AdvisoryFindings,
    preexisting: Vec<Finding>,
}

impl BucketSet {
    fn would_block(&self) -> bool {
        !self.blocking.new_hard_findings.is_empty()
            || !self.blocking.worsened_findings.is_empty()
            || !self.blocking.always_block_findings.is_empty()
    }
}

/// Assembles the final decision from the classified buckets.
fn build_decision(
    req: &SaveGateRequest,
    candidate_report: &Report,
    baseline_report: Option<&Report>,
    buckets: BucketSet,
) -> SaveGateDecision {
    let would_block = buckets.would_block();
    let has_advisory = !buckets.advisory.new_soft_findings.is_empty();
    let (verdict, exit_code) = match req.mode {
        SaveGateMode::SaveGate if would_block => (SaveGateVerdict::Block, 3),
        SaveGateMode::SaveGate if has_advisory => (SaveGateVerdict::Advisory, 2),
        SaveGateMode::SaveGate => (SaveGateVerdict::Pass, 0),
        SaveGateMode::Advisory if would_block || has_advisory => (SaveGateVerdict::Advisory, 2),
        SaveGateMode::Advisory => (SaveGateVerdict::Pass, 0),
    };
    SaveGateDecision {
        schema: SAVE_GATE_SCHEMA,
        verdict,
        exit_code,
        path: req.rel_path.clone(),
        mode: req.mode.as_str().to_string(),
        candidate_score: candidate_report.score,
        baseline_score: baseline_report.map(|r| r.score),
        summary: summarize(verdict, &buckets),
        rerun_command: format!(
            "jankurai audit-file . --path {} --candidate - --mode save-gate",
            req.rel_path
        ),
        blocking: buckets.blocking,
        advisory: buckets.advisory,
        preexisting_findings: buckets.preexisting,
    }
}

/// Builds the one-line human summary.
fn summarize(verdict: SaveGateVerdict, buckets: &BucketSet) -> String {
    let new_hard = buckets.blocking.new_hard_findings.len();
    let worsened = buckets.blocking.worsened_findings.len();
    let always = buckets.blocking.always_block_findings.len();
    let soft = buckets.advisory.new_soft_findings.len();
    match verdict {
        SaveGateVerdict::Pass => "ok: no new findings".to_string(),
        SaveGateVerdict::Advisory => format!("advisory: {soft} new soft finding(s)"),
        SaveGateVerdict::Block => format!(
            "blocked: {new_hard} new hard, {worsened} worsened, {always} always-block finding(s)"
        ),
    }
}

/// A stable delta key for a finding, ignoring line numbers so a finding that
/// merely shifts position is not counted as new.
fn delta_key(finding: &Finding) -> String {
    let rule = finding
        .rule_id
        .as_deref()
        .filter(|r| !r.is_empty())
        .unwrap_or(&finding.check_id);
    let problem: String = finding
        .problem
        .chars()
        .filter(|c| !c.is_ascii_digit())
        .collect();
    format!("{rule}|{}|{problem}", finding.category)
}

/// Orders severities so worsening can be detected.
fn severity_rank(severity: &str) -> i32 {
    match severity {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn candidate_todo_comment_findings(req: &SaveGateRequest) -> Vec<Finding> {
    let Some(bytes) = req.candidate_bytes.as_deref() else {
        return vec![];
    };
    todo_comment_findings_from_bytes(&req.rel_path, bytes)
}

fn baseline_todo_comment_findings(req: &SaveGateRequest) -> Vec<Finding> {
    let bytes: Option<Vec<u8>> = match &req.baseline_bytes {
        Some(bytes) => Some(bytes.clone()),
        None => std::fs::read(req.root.join(&req.rel_path)).ok(),
    };
    let Some(bytes) = bytes else {
        return vec![];
    };
    todo_comment_findings_from_bytes(&req.rel_path, &bytes)
}

fn todo_comment_findings_from_bytes(rel_path: &str, bytes: &[u8]) -> Vec<Finding> {
    let file = super::fs::file_info_from_candidate(rel_path, bytes, 4096);
    let hits =
        scan::pattern_hits_filtered(&[file], scan::TODO_PATTERNS, Some("HLT-001-DEAD-MARKER"));
    hits
        .into_iter()
        .map(|hit| Finding {
            severity: "high".into(),
            category: "audit".into(),
            path: hit.path.clone(),
            problem: hit.problem.clone(),
            agent_fix: "replace TODO/placeholder markers with implemented behavior or a tracked exception record".into(),
            evidence: vec![format!(
                "{}:{} {}",
                hit.path,
                hit.line.unwrap_or(1),
                hit.text
            )],
            check_id: "HLT-001-DEAD-MARKER:audit".into(),
            hardness: "hard".into(),
            confidence: 1.0,
            evidence_kind: "string-match".into(),
            rerun_command: String::new(),
            fingerprint: String::new(),
            rule_id: Some("HLT-001-DEAD-MARKER".into()),
            tlr: None,
            lane: None,
            docs_url: None,
            owner: None,
            line: hit.line,
            matched_term: hit.matched_term,
            reason: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(severity: &str, rule: &str, problem: &str) -> Finding {
        Finding {
            severity: severity.into(),
            category: "audit".into(),
            path: "src/a.rs".into(),
            problem: problem.into(),
            agent_fix: "fix it".into(),
            evidence: vec![],
            check_id: format!("{rule}:audit"),
            hardness: "hard".into(),
            confidence: 1.0,
            evidence_kind: "string-match".into(),
            rerun_command: String::new(),
            fingerprint: String::new(),
            rule_id: Some(rule.into()),
            tlr: None,
            lane: None,
            docs_url: None,
            owner: None,
            line: Some(10),
            matched_term: None,
            reason: None,
        }
    }

    #[test]
    fn new_hard_finding_blocks() {
        let fail_on = vec!["critical".to_string(), "high".to_string()];
        let buckets = classify(
            &[finding("high", "HLT-029", "swallows error")],
            &[],
            &fail_on,
        );
        assert_eq!(buckets.blocking.new_hard_findings.len(), 1);
        assert!(buckets.would_block());
    }

    #[test]
    fn preexisting_debt_does_not_block() {
        let fail_on = vec!["critical".to_string(), "high".to_string()];
        let base = vec![finding("high", "HLT-029", "swallows error")];
        let cand = vec![finding("high", "HLT-029", "swallows error")];
        let buckets = classify(&cand, &base, &fail_on);
        assert!(!buckets.would_block());
        assert_eq!(buckets.preexisting.len(), 1);
    }

    #[test]
    fn preexisting_critical_always_blocks() {
        let fail_on = vec!["critical".to_string(), "high".to_string()];
        let base = vec![finding("critical", "HLT-030", "destructive sql")];
        let cand = vec![finding("critical", "HLT-030", "destructive sql")];
        let buckets = classify(&cand, &base, &fail_on);
        assert_eq!(buckets.blocking.always_block_findings.len(), 1);
        assert!(buckets.would_block());
    }

    #[test]
    fn worsened_severity_blocks() {
        let fail_on = vec!["critical".to_string(), "high".to_string()];
        let base = vec![finding("medium", "HLT-001", "marker")];
        let cand = vec![finding("high", "HLT-001", "marker")];
        let buckets = classify(&cand, &base, &fail_on);
        assert_eq!(buckets.blocking.worsened_findings.len(), 1);
        assert!(buckets.would_block());
    }

    #[test]
    fn new_soft_finding_is_advisory_only() {
        let fail_on = vec!["critical".to_string(), "high".to_string()];
        let buckets = classify(&[finding("low", "HLT-041", "comment")], &[], &fail_on);
        assert!(!buckets.would_block());
        assert_eq!(buckets.advisory.new_soft_findings.len(), 1);
    }

    #[test]
    fn delta_key_ignores_line_numbers() {
        let a = finding("high", "HLT-029", "error at line 12");
        let b = finding("high", "HLT-029", "error at line 88");
        assert_eq!(delta_key(&a), delta_key(&b));
    }

    #[test]
    fn mode_parsing_rejects_unknown() {
        assert!(SaveGateMode::parse("save-gate").is_ok());
        assert!(SaveGateMode::parse("advisory").is_ok());
        assert!(SaveGateMode::parse("nonsense").is_err());
    }
}
