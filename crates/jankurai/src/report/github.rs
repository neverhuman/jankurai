use crate::model::Report;

/// Shorten `sha256:…` digests for GitHub step summaries (first 12 hex digits of the digest).
fn truncate_content_fingerprint_for_summary(fingerprint: &str) -> String {
    const PREFIX: &str = "sha256:";
    let Some(rest) = fingerprint.strip_prefix(PREFIX) else {
        return fingerprint.chars().take(PREFIX.len() + 12).collect();
    };
    let hex_prefix: String = rest
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(12)
        .collect();
    if hex_prefix.is_empty() {
        format!("{PREFIX}{}", &rest.chars().take(12).collect::<String>())
    } else {
        format!("{PREFIX}{hex_prefix}")
    }
}

pub fn render_step_summary(report: &Report) -> String {
    let mut out = String::new();
    use std::fmt::Write;
    let decision = report
        .decision
        .as_ref()
        .map(|decision| decision.status.as_str())
        .unwrap_or("unknown");
    let _ = writeln!(out, "### jankurai");
    let _ = writeln!(out);
    let _ = writeln!(out, "- score: `{}`", report.score);
    let _ = writeln!(out, "- raw score: `{}`", report.raw_score);
    let _ = writeln!(out, "- decision: `{}`", decision);
    let _ = writeln!(out, "- findings: `{}`", report.findings.len());
    if report.ux_qa.artifact.is_some()
        || report.security_evidence.artifact.is_some()
        || report.boundaries.artifact.is_some()
    {
        let _ = writeln!(out);
        let _ = writeln!(out, "#### lane artifacts");
        if let Some(art) = &report.ux_qa.artifact {
            let _ = writeln!(
                out,
                "- ux-qa `{}`: reports={} worst={} violations={} missing_states={} missing_artifacts={} fingerprints={} visual_baseline=missing:{}/changed:{}/review:{}/block:{} a11y_violations={}",
                art.path,
                art.report_count,
                art.worst_decision,
                art.total_violations,
                art.reports_missing_required_states,
                art.reports_missing_required_artifacts,
                art.artifact_fingerprint_count,
                art.visual_baseline_missing,
                art.visual_baseline_changed,
                art.visual_baseline_review,
                art.visual_baseline_block,
                art.accessibility_violation_total
            );
        }
        if let Some(art) = &report.security_evidence.artifact {
            let _ = writeln!(
                out,
                "- security `{}`: exit={} strict={} ran={}/skip={}/fail={}",
                art.path,
                art.envelope_exit_code,
                art.wrapper_strict,
                art.commands_ran,
                art.commands_skipped,
                art.commands_failed
            );
        }
        if let Some(art) = &report.boundaries.artifact {
            let fp = truncate_content_fingerprint_for_summary(&art.content_fingerprint);
            let _ = writeln!(
                out,
                "- boundaries `{}`: stack=`{}` adapters=`{}` event_contracts=`{}` generated_types=`{}` client_markers=`{}` streaming_exceptions=`{}` fp=`{}`",
                art.path,
                art.stack_id,
                art.adapter_path_count,
                art.event_contract_path_count,
                art.generated_type_path_count,
                art.client_marker_count,
                art.streaming_exception_count,
                fp,
            );
        }
        if !report.boundaries.reclassifications.is_empty() {
            let passed = report
                .boundaries
                .reclassifications
                .iter()
                .filter(|boundary| boundary.status == "passed")
                .count();
            let rejected = report.boundaries.reclassifications.len() - passed;
            let _ = writeln!(
                out,
                "- boundary reclassifications: passed=`{}` rejected=`{}`",
                passed, rejected
            );
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "#### agent_fix_queue");
    if report.agent_fix_queue.is_empty() {
        let _ = writeln!(out, "No queued fixes.");
    } else {
        for item in &report.agent_fix_queue {
            let _ = writeln!(out, "- [{}] `{}`: {}", item.priority, item.path, item.task);
        }
    }
    out
}
