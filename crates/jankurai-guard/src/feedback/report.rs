//! Failure-report writing. On a block the guard writes an agent-friendly
//! markdown report and a machine-readable JSON sidecar under
//! `.jankurai/guard/failures/<ts>.{md,json}`, and overwrites
//! `.jankurai/guard/LAST_FAILURE.md` so an agent can always find the most
//! recent failure at a fixed path.
//!
//! The formatting functions are pure (data in, `String` out) so they can be
//! golden-tested without touching the filesystem.

use crate::audit_client::{GuardDecision, GuardFinding};
use crate::feedback::now_stamp;
use crate::GuardError;
use std::fs;
use std::path::{Path, PathBuf};

/// Writes the markdown report, the JSON sidecar and the `LAST_FAILURE.md`
/// pointer for `decision`. Returns the path of the markdown report.
pub fn write_failure_report(
    repo_root: &Path,
    decision: &GuardDecision,
) -> Result<PathBuf, GuardError> {
    let failures_dir = repo_root.join(".jankurai").join("guard").join("failures");
    fs::create_dir_all(&failures_dir)?;

    let stamp = now_stamp();
    let md_path = failures_dir.join(format!("{stamp}.md"));
    let json_path = failures_dir.join(format!("{stamp}.json"));

    let md = format_report_md(decision);
    fs::write(&md_path, &md)?;

    let json = serde_json::to_string_pretty(decision)
        .map_err(|e| GuardError::State(format!("serialize decision: {e}")))?;
    fs::write(&json_path, json)?;

    let last = repo_root
        .join(".jankurai")
        .join("guard")
        .join("LAST_FAILURE.md");
    fs::write(&last, &md)?;

    Ok(md_path)
}

/// Formats a [`GuardDecision`] as an agent-friendly markdown report. Pure.
pub fn format_report_md(decision: &GuardDecision) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# JANKURAI GUARD: BLOCKED {}\n\n",
        path_or_unknown(&decision.path)
    ));

    if !decision.summary.is_empty() {
        out.push_str(&format!("{}\n\n", decision.summary));
    }
    if let (Some(c), Some(b)) = (decision.candidate_score, decision.baseline_score) {
        out.push_str(&format!("Candidate score {c} vs baseline {b}.\n\n"));
    }

    out.push_str("## Blocking findings\n\n");
    let blocking: Vec<&GuardFinding> = decision.blocking.all().collect();
    if blocking.is_empty() {
        out.push_str("_No individual findings were attached to this block._\n\n");
    } else {
        for finding in &blocking {
            out.push_str(&format_finding_block(finding));
        }
    }

    out.push_str("## Pre-existing issues (not blocking)\n\n");
    if decision.preexisting_findings.is_empty() {
        out.push_str("_None._\n\n");
    } else {
        for finding in &decision.preexisting_findings {
            out.push_str(&format!(
                "- `{}` {} — {}\n",
                finding.rule_id,
                line_suffix(finding),
                finding.problem
            ));
        }
        out.push('\n');
    }

    let rerun = if decision.rerun_command.is_empty() {
        "jankurai audit-file . --path <path> --candidate - --mode save-gate"
    } else {
        &decision.rerun_command
    };
    out.push_str(&format!("Re-run after fixing: `{rerun}`\n"));
    out
}

/// Formats one blocking finding as a markdown block. Pure.
fn format_finding_block(finding: &GuardFinding) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "[BLOCK] {}  {}\n",
        rule_or_unknown(&finding.rule_id),
        finding.problem
    ));
    if let Some(line) = finding.line {
        out.push_str(&format!("line {line}\n"));
    }
    if !finding.agent_fix.is_empty() {
        out.push_str(&format!("fix: {}\n", finding.agent_fix));
    }
    if !finding.evidence.is_empty() {
        for line in &finding.evidence {
            out.push_str(&format!("  > {line}\n"));
        }
    }
    out.push('\n');
    out
}

/// Returns `L<n>` when a finding has a line, else an empty string.
fn line_suffix(finding: &GuardFinding) -> String {
    match finding.line {
        Some(n) => format!("L{n}"),
        None => String::new(),
    }
}

/// Returns the path, or a sentinel label when the decision omitted it.
fn path_or_unknown(path: &str) -> &str {
    if path.is_empty() {
        "<unknown path>"
    } else {
        path
    }
}

/// Returns the rule id, or a sentinel label when the finding omitted it.
fn rule_or_unknown(rule_id: &str) -> &str {
    if rule_id.is_empty() {
        "GUARD-UNSPECIFIED"
    } else {
        rule_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit_client::MockAuditClient;
    use crate::AuditClient;

    #[test]
    fn report_has_required_sections() {
        let decision = MockAuditClient::always_block()
            .audit(Path::new("."), Path::new("src/foo.rs"), b"x")
            .unwrap();
        let md = format_report_md(&decision);
        assert!(md.starts_with("# JANKURAI GUARD: BLOCKED src/foo.rs"));
        assert!(md.contains("## Blocking findings"));
        assert!(md.contains("[BLOCK]"));
        assert!(md.contains("## Pre-existing issues (not blocking)"));
        assert!(md.contains("Re-run after fixing:"));
    }
}
