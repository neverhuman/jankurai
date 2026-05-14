//! The test audit client. [`MockAuditClient`] returns a scripted decision for
//! every call so unit and integration tests of the guard never need the real
//! `jankurai` binary on `PATH`.

use super::{
    default_schema, AdvisoryFindings, AuditClient, BlockingFindings, GuardDecision, GuardFinding,
    Verdict,
};
use crate::GuardError;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A test audit client that returns a scripted decision for every call and
/// counts how many times it was invoked.
#[derive(Debug, Clone)]
pub struct MockAuditClient {
    decision: GuardDecision,
    calls: Arc<AtomicUsize>,
}

impl MockAuditClient {
    /// Builds a mock that always returns `decision`.
    pub fn new(decision: GuardDecision) -> Self {
        Self {
            decision,
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Builds a mock that always passes.
    pub fn always_pass() -> Self {
        Self::new(GuardDecision {
            schema: default_schema(),
            verdict: Verdict::Pass,
            exit_code: 0,
            path: String::new(),
            mode: "save-gate".to_string(),
            candidate_score: Some(100),
            baseline_score: Some(100),
            summary: "no findings".to_string(),
            blocking: BlockingFindings::default(),
            advisory: AdvisoryFindings::default(),
            preexisting_findings: Vec::new(),
            rerun_command: String::new(),
        })
    }

    /// Builds a mock that always blocks with one high-severity finding.
    pub fn always_block() -> Self {
        let finding = GuardFinding {
            severity: "high".to_string(),
            category: "audit".to_string(),
            path: "src/foo.rs".to_string(),
            problem: "scripted block for tests".to_string(),
            agent_fix: "resolve the finding".to_string(),
            line: Some(1),
            rule_id: "TEST-BLOCK".to_string(),
            check_id: "TEST-BLOCK:audit".to_string(),
            evidence: vec!["test evidence".to_string()],
        };
        Self::new(GuardDecision {
            schema: default_schema(),
            verdict: Verdict::Block,
            exit_code: 3,
            path: "src/foo.rs".to_string(),
            mode: "save-gate".to_string(),
            candidate_score: Some(40),
            baseline_score: Some(90),
            summary: "1 new hard finding".to_string(),
            blocking: BlockingFindings {
                new_hard_findings: vec![finding],
                ..BlockingFindings::default()
            },
            advisory: AdvisoryFindings::default(),
            preexisting_findings: Vec::new(),
            rerun_command: "jankurai audit-file . --path src/foo.rs --candidate - --mode save-gate"
                .to_string(),
        })
    }

    /// Returns how many times [`AuditClient::audit`] has been called.
    pub fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl AuditClient for MockAuditClient {
    fn audit(
        &self,
        _repo_root: &Path,
        rel_path: &Path,
        _candidate_bytes: &[u8],
    ) -> Result<GuardDecision, GuardError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut decision = self.decision.clone();
        decision.path = rel_path.to_string_lossy().replace('\\', "/");
        Ok(decision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_block_reports_path_and_counts() {
        let mock = MockAuditClient::always_block();
        let d = mock
            .audit(Path::new("."), Path::new("a/b.rs"), b"x")
            .unwrap();
        assert!(d.verdict.is_block());
        assert_eq!(d.path, "a/b.rs");
        assert_eq!(mock.call_count(), 1);
    }
}
