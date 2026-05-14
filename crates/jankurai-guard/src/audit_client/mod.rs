//! The audit interface. The guard never depends on jankurai's internal Rust
//! types: it speaks the `jankurai-save-gate/1` JSON contract over a subprocess
//! boundary. [`AuditClient`] is the trait the guard calls; [`CliAuditClient`]
//! shells out to the real `jankurai audit-file` binary; [`MockAuditClient`]
//! (in the [`mock`] submodule) replays a scripted decision for tests.

pub mod mock;

pub use mock::MockAuditClient;

use crate::policy::GuardPolicy;
use crate::GuardError;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// The verdict of a single-file audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// The candidate is clean and may land.
    Pass,
    /// The candidate has soft findings but may land.
    Advisory,
    /// The candidate must not land.
    Block,
}

impl Verdict {
    /// Returns `true` when the verdict forbids the write from landing.
    pub fn is_block(self) -> bool {
        matches!(self, Verdict::Block)
    }
}

/// One finding from the audit engine, modeling the `blocking.*` and `advisory.*`
/// finding objects in the save-gate JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardFinding {
    /// Severity label: `critical` | `high` | `medium` | `low`.
    #[serde(default)]
    pub severity: String,
    /// Finding category, e.g. `audit`.
    #[serde(default)]
    pub category: String,
    /// Repo-relative path the finding applies to.
    #[serde(default)]
    pub path: String,
    /// One-line description of the problem.
    #[serde(default)]
    pub problem: String,
    /// The concrete fix the agent should apply.
    #[serde(default)]
    pub agent_fix: String,
    /// Line number the finding points at, when known.
    #[serde(default)]
    pub line: Option<u64>,
    /// The rule identifier, e.g. `HLT-029`.
    #[serde(default)]
    pub rule_id: String,
    /// The fully-qualified check identifier, e.g. `HLT-029:audit`.
    #[serde(default)]
    pub check_id: String,
    /// Supporting evidence lines.
    #[serde(default)]
    pub evidence: Vec<String>,
}

/// The `blocking` block of the save-gate JSON.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockingFindings {
    /// Hard findings newly introduced by the candidate.
    #[serde(default)]
    pub new_hard_findings: Vec<GuardFinding>,
    /// Pre-existing findings the candidate made worse.
    #[serde(default)]
    pub worsened_findings: Vec<GuardFinding>,
    /// Findings that always block regardless of baseline.
    #[serde(default)]
    pub always_block_findings: Vec<GuardFinding>,
}

impl BlockingFindings {
    /// Iterates every blocking finding across the three buckets.
    pub fn all(&self) -> impl Iterator<Item = &GuardFinding> {
        self.new_hard_findings
            .iter()
            .chain(self.worsened_findings.iter())
            .chain(self.always_block_findings.iter())
    }

    /// Returns `true` when no blocking findings are present.
    pub fn is_empty(&self) -> bool {
        self.new_hard_findings.is_empty()
            && self.worsened_findings.is_empty()
            && self.always_block_findings.is_empty()
    }
}

/// The `advisory` block of the save-gate JSON.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisoryFindings {
    /// Soft findings newly introduced by the candidate.
    #[serde(default)]
    pub new_soft_findings: Vec<GuardFinding>,
}

/// A parsed `jankurai-save-gate/1` decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardDecision {
    /// Schema discriminator, expected to be `jankurai-save-gate/1`.
    #[serde(default = "default_schema")]
    pub schema: String,
    /// The overall verdict.
    pub verdict: Verdict,
    /// The exit code the audit engine reported.
    #[serde(default)]
    pub exit_code: i32,
    /// Repo-relative path that was audited.
    #[serde(default)]
    pub path: String,
    /// The audit mode, e.g. `save-gate`.
    #[serde(default)]
    pub mode: String,
    /// The candidate's score.
    #[serde(default)]
    pub candidate_score: Option<i64>,
    /// The baseline score the candidate is compared against.
    #[serde(default)]
    pub baseline_score: Option<i64>,
    /// A one-line human summary.
    #[serde(default)]
    pub summary: String,
    /// The blocking findings.
    #[serde(default)]
    pub blocking: BlockingFindings,
    /// The advisory findings.
    #[serde(default)]
    pub advisory: AdvisoryFindings,
    /// Findings that existed before the candidate and are not attributable to it.
    #[serde(default)]
    pub preexisting_findings: Vec<GuardFinding>,
    /// The command to re-run the audit after fixing.
    #[serde(default)]
    pub rerun_command: String,
}

/// The schema discriminator string for the save-gate JSON contract.
pub(crate) fn default_schema() -> String {
    "jankurai-save-gate/1".to_string()
}

impl GuardDecision {
    /// Builds a synthetic block decision used when the audit engine is
    /// unavailable and the policy is fail-closed.
    pub fn audit_unavailable(rel_path: &str, reason: &str) -> Self {
        let finding = GuardFinding {
            severity: "high".to_string(),
            category: "guard".to_string(),
            path: rel_path.to_string(),
            problem: format!("audit engine unavailable: {reason}"),
            agent_fix: "ensure `jankurai` is installed and on PATH, then re-run".to_string(),
            line: None,
            rule_id: "GUARD-AUDIT-UNAVAILABLE".to_string(),
            check_id: "GUARD-AUDIT-UNAVAILABLE:guard".to_string(),
            evidence: vec![reason.to_string()],
        };
        Self {
            schema: default_schema(),
            verdict: Verdict::Block,
            exit_code: 4,
            path: rel_path.to_string(),
            mode: "save-gate".to_string(),
            candidate_score: None,
            baseline_score: None,
            summary: format!("audit unavailable: {reason}"),
            blocking: BlockingFindings {
                always_block_findings: vec![finding],
                ..BlockingFindings::default()
            },
            advisory: AdvisoryFindings::default(),
            preexisting_findings: Vec::new(),
            rerun_command: format!(
                "jankurai audit-file . --path {rel_path} --candidate - --mode save-gate"
            ),
        }
    }

    /// Builds a synthetic advisory decision used when the audit engine is
    /// unavailable but the policy is in `observe` mode (degrade to a warning).
    pub fn audit_degraded(rel_path: &str, reason: &str) -> Self {
        let mut decision = Self::audit_unavailable(rel_path, reason);
        decision.verdict = Verdict::Advisory;
        decision.exit_code = 2;
        decision.advisory.new_soft_findings =
            std::mem::take(&mut decision.blocking.always_block_findings);
        decision.blocking = BlockingFindings::default();
        decision.summary = format!("audit unavailable (observe mode): {reason}");
        decision
    }
}

/// The audit interface the guard depends on. Implementors take candidate bytes
/// and return a [`GuardDecision`].
pub trait AuditClient: Send + Sync {
    /// Audits `candidate_bytes` as the content of `rel_path` within `repo_root`.
    fn audit(
        &self,
        repo_root: &Path,
        rel_path: &Path,
        candidate_bytes: &[u8],
    ) -> Result<GuardDecision, GuardError>;
}

/// The production audit client: it shells out to `jankurai audit-file`.
#[derive(Debug, Clone)]
pub struct CliAuditClient {
    binary: String,
    fail_closed: bool,
    observe: bool,
}

/// Resolves the jankurai binary path. Uses `JANKURAI_BIN` when it is set and
/// non-empty; otherwise defaults to `"jankurai"` on `PATH`.
fn resolve_jankurai_binary() -> String {
    match std::env::var("JANKURAI_BIN") {
        Ok(bin) if !bin.is_empty() => bin,
        _ => "jankurai".to_string(),
    }
}

impl CliAuditClient {
    /// Builds a client honoring `policy.fail_closed` and the observe flag.
    pub fn from_policy(policy: &GuardPolicy) -> Self {
        Self {
            binary: resolve_jankurai_binary(),
            fail_closed: policy.fail_closed,
            observe: matches!(policy.mode, crate::GuardMode::Observe),
        }
    }

    /// Maps an audit-engine failure to a decision per the fail-closed policy.
    fn degrade(&self, rel_path: &str, reason: &str) -> GuardDecision {
        if self.observe || !self.fail_closed {
            GuardDecision::audit_degraded(rel_path, reason)
        } else {
            GuardDecision::audit_unavailable(rel_path, reason)
        }
    }
}

impl AuditClient for CliAuditClient {
    fn audit(
        &self,
        repo_root: &Path,
        rel_path: &Path,
        candidate_bytes: &[u8],
    ) -> Result<GuardDecision, GuardError> {
        let rel = rel_path.to_string_lossy().replace('\\', "/");
        let mut child = match Command::new(&self.binary)
            .arg("audit-file")
            .arg(repo_root)
            .arg("--path")
            .arg(&rel)
            .arg("--candidate")
            .arg("-")
            .arg("--mode")
            .arg("save-gate")
            .arg("--format")
            .arg("json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => return Ok(self.degrade(&rel, &format!("spawn failed: {e}"))),
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(candidate_bytes) {
                return Ok(self.degrade(&rel, &format!("stdin write failed: {e}")));
            }
        }

        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(e) => return Ok(self.degrade(&rel, &format!("wait failed: {e}"))),
        };

        let code = output.status.code().unwrap_or(-1);
        if code == 4 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(self.degrade(&rel, &format!("internal audit error: {stderr}")));
        }

        match serde_json::from_slice::<GuardDecision>(&output.stdout) {
            Ok(decision) => Ok(decision),
            Err(e) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Ok(self.degrade(
                    &rel,
                    &format!("unparseable output (exit {code}): {e}; stderr: {stderr}"),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_save_gate_json() {
        let json = r#"{
            "schema":"jankurai-save-gate/1","verdict":"block","exit_code":3,
            "path":"src/foo.rs","mode":"save-gate","candidate_score":78,
            "baseline_score":84,"summary":"1 new hard finding",
            "blocking":{"new_hard_findings":[{"severity":"high","category":"audit",
            "path":"src/foo.rs","problem":"p","agent_fix":"f","line":88,
            "rule_id":"HLT-029","check_id":"HLT-029:audit","evidence":["e"]}],
            "worsened_findings":[],"always_block_findings":[]},
            "advisory":{"new_soft_findings":[]},"preexisting_findings":[],
            "rerun_command":"jankurai audit-file ."
        }"#;
        let d: GuardDecision = serde_json::from_str(json).unwrap();
        assert_eq!(d.verdict, Verdict::Block);
        assert_eq!(d.blocking.new_hard_findings.len(), 1);
        assert_eq!(d.blocking.new_hard_findings[0].rule_id, "HLT-029");
    }
}
