use crate::model::Report;
use crate::report::proof;
use anyhow::Result;
use serde_json::Value;
use std::fs;

pub fn write_json(path: &str, content: &str) -> Result<()> {
    if path != "-" {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, content)?;
    } else {
        print!("{content}");
    }
    Ok(())
}

pub fn write_markdown(path: &str, content: &str) -> Result<()> {
    if path != "-" {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, content)?;
    } else {
        print!("{content}");
    }
    Ok(())
}

pub fn render_markdown(report: &Report) -> String {
    let mut out = String::new();
    use std::fmt::Write;
    let _ = writeln!(out, "# jankurai Repo Score");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Standard: `{}`", report.standard);
    let _ = writeln!(out, "- Auditor: `{}`", report.auditor_version);
    let _ = writeln!(out, "- Schema: `{}`", report.schema_version);
    let _ = writeln!(out, "- Paper edition: `{}`", report.paper_edition);
    let _ = writeln!(out, "- Target stack ID: `{}`", report.target_stack_id);
    let _ = writeln!(out, "- Target stack: `{}`", report.target_stack);
    let _ = writeln!(out, "- Repo: `{}`", report.repo);
    if let Some(run_id) = &report.run_id {
        let _ = writeln!(out, "- Run ID: `{}`", run_id);
    }
    if let Some(started_at) = &report.started_at {
        let _ = writeln!(out, "- Started at: `{}`", started_at);
    }
    if let Some(elapsed_ms) = report.elapsed_ms {
        let _ = writeln!(out, "- Elapsed: `{}` ms", elapsed_ms);
    }
    let _ = writeln!(out, "- Scope: `{}`", report.scope.mode);
    if !report.scope.paths.is_empty() {
        let _ = writeln!(out, "- Changed: `{}`", report.scope.paths.join(", "));
    }
    if report.scope.mode == "changed-fast" {
        let _ = writeln!(
            out,
            "- Advisory: `changed-fast scans only changed files plus required control files; run the full audit before merge or release.`"
        );
    }
    proof::append_proof_receipts(&mut out, report);
    let _ = writeln!(out, "- Raw score: `{}`", report.raw_score);
    let _ = writeln!(out, "- Final score: `{}`", report.score);
    if let Some(decision) = &report.decision {
        let _ = writeln!(out, "- Decision: `{}`", decision.status);
        let _ = writeln!(out, "- Minimum score: `{}`", decision.minimum_score);
    }
    let _ = writeln!(
        out,
        "- Caps applied: `{}`",
        if report.caps_applied.is_empty() {
            "none".into()
        } else {
            report.caps_applied.join(", ")
        }
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "## Hard Rule Caps");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Rule | Max Score | Applied |");
    let _ = writeln!(out, "| --- | ---: | --- |");
    for rule in &report.hard_rules {
        let mark = if report.caps_applied.iter().any(|c| c == &rule.id) {
            "yes"
        } else {
            "no"
        };
        let _ = writeln!(out, "| `{}` | {} | {} |", rule.id, rule.max_score, mark);
    }
    if let Some(copy_code) = &report.copy_code {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Copy-Code Redundancy");
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "- Status: `{}` hard=`{}` warning=`{}` files=`{}`",
            copy_code.status,
            copy_code.summary.hard_classes,
            copy_code.summary.warning_classes,
            copy_code.summary.files_considered
        );
        let _ = writeln!(
            out,
            "- Policy: min-lines=`{}` min-tokens=`{}` max-findings=`{}` include-tests=`{}` strict=`{}`",
            copy_code.policy.min_lines,
            copy_code.policy.min_tokens,
            copy_code.policy.max_findings,
            copy_code.policy.include_tests,
            copy_code.policy.strict
        );
        let _ = writeln!(
            out,
            "- Duplicate volume: lines=`{}` tokens=`{}` bytes=`{}`",
            copy_code.summary.duplicate_lines,
            copy_code.summary.duplicate_tokens,
            copy_code.summary.duplicate_bytes
        );
        if !copy_code.notes.is_empty() {
            let _ = writeln!(out);
            let _ = writeln!(out, "- Notes:");
            for note in &copy_code.notes {
                let _ = writeln!(out, "  - {note}");
            }
        }
        if !copy_code.classes.is_empty() {
            let _ = writeln!(out);
            let _ = writeln!(
                out,
                "| Kind | Severity | Language | Lines | Tokens | Instances | Reason |"
            );
            let _ = writeln!(out, "| --- | --- | --- | ---: | ---: | --- | --- |");
            for class in &copy_code.classes {
                let instances = class
                    .instances
                    .iter()
                    .map(|instance| {
                        format!(
                            "{}:{}-{}",
                            instance.path, instance.start_line, instance.end_line
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(
                    out,
                    "| `{:?}` | `{:?}` | `{}` | {} | {} | `{}` | `{}` |",
                    class.kind,
                    class.severity,
                    class.language,
                    class.duplicate_lines,
                    class.duplicate_tokens,
                    instances,
                    class.reason
                );
            }
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Dimensions");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Dimension | Weight | Score | Weighted | Evidence |");
    let _ = writeln!(out, "| --- | ---: | ---: | ---: | --- |");
    for dim in &report.dimensions {
        let evidence = dim
            .evidence
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        let _ = writeln!(
            out,
            "| {} | {} | {} | {:.2} | {} |",
            dim.name, dim.weight, dim.score, dim.weighted_points, evidence
        );
    }
    let _ = writeln!(out);
    let profile = &report.profile_structure;
    let _ = writeln!(out, "## Reference Profile Structure");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- Applicable cells: `{}` canonical=`{}` noncanonical=`{}` guidance missing=`{}`",
        profile.applicable_count,
        profile.canonical_count,
        profile.noncanonical_count,
        profile.guidance_missing_count
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Cell | Status | Canonical | Detected | Aliases | Guidance | Owner | Proof lane | Agent fix |"
    );
    let _ = writeln!(
        out,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for cell in &profile.cells {
        let detected = if cell.detected_paths.is_empty() {
            "-".into()
        } else {
            cell.detected_paths.join(", ")
        };
        let aliases = if cell.aliases.is_empty() {
            "-".into()
        } else {
            cell.aliases.join(", ")
        };
        let _ = writeln!(
            out,
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |",
            cell.id,
            cell.status,
            cell.canonical_path,
            detected,
            aliases,
            cell.guidance_status,
            cell.owner,
            cell.proof_lane,
            cell.agent_fix
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Rendered UX QA");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Web surface: `{}`", report.ux_qa.web_surface);
    let _ = writeln!(
        out,
        "- Layered UX lane: `{}`",
        report.ux_qa.has_rendered_ux_lane
    );
    let _ = writeln!(
        out,
        "- Missing: `{}`",
        if report.ux_qa.missing_categories.is_empty() {
            "none".into()
        } else {
            report.ux_qa.missing_categories.join(", ")
        }
    );
    if let Some(tuiwright) = report
        .ux_qa
        .evidence
        .get("tuiwright")
        .and_then(Value::as_object)
    {
        let flows = tuiwright
            .get("flow_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let files = tuiwright
            .get("test_files")
            .and_then(Value::as_array)
            .map(|items| items.len())
            .unwrap_or(0);
        let assertions = tuiwright
            .get("assertion_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let actions = tuiwright
            .get("action_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let artifact_counts = tuiwright
            .get("artifact_counts")
            .and_then(Value::as_object)
            .map(|counts| {
                if counts.is_empty() {
                    "none".into()
                } else {
                    counts
                        .iter()
                        .map(|(kind, count)| format!("{kind}={}", count.as_u64().unwrap_or(0)))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            })
            .unwrap_or_else(|| "none".into());
        let _ = writeln!(
            out,
            "- Tuiwright TUI flows: `{}` flow(s) across `{}` file(s); assertions=`{}` actions=`{}` artifacts=`{}`",
            flows, files, assertions, actions, artifact_counts
        );
    }
    if let Some(art) = &report.ux_qa.artifact {
        let _ = writeln!(out);
        let _ = writeln!(out, "### Ingested UX QA report (`{}`)", art.path);
        let _ = writeln!(out, "- Report count: `{}`", art.report_count);
        let _ = writeln!(out, "- Worst decision: `{}`", art.worst_decision);
        let _ = writeln!(out, "- Total violations: `{}`", art.total_violations);
        let _ = writeln!(
            out,
            "- Summary errors / warnings: `{}` / `{}`",
            art.summary_errors, art.summary_warnings
        );
        let artifact_counts = if art.artifact_counts_by_kind.is_empty() {
            "none".into()
        } else {
            art.artifact_counts_by_kind
                .iter()
                .map(|(kind, count)| format!("{kind}={count}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(out, "- Artifact counts: `{}`", artifact_counts);
        let _ = writeln!(
            out,
            "- Artifact fingerprints: `{}`",
            art.artifact_fingerprint_count
        );
        let _ = writeln!(
            out,
            "- Visual baseline counts: missing=`{}` changed=`{}` review=`{}` block=`{}`",
            art.visual_baseline_missing,
            art.visual_baseline_changed,
            art.visual_baseline_review,
            art.visual_baseline_block
        );
        let _ = writeln!(
            out,
            "- Missing required states: `{}` report(s) `{}`",
            art.reports_missing_required_states,
            if art.missing_state_names.is_empty() {
                "none".into()
            } else {
                art.missing_state_names.join(", ")
            }
        );
        let _ = writeln!(
            out,
            "- Missing required artifacts: `{}` report(s) `{}`",
            art.reports_missing_required_artifacts,
            if art.missing_artifact_kinds.is_empty() {
                "none".into()
            } else {
                art.missing_artifact_kinds.join(", ")
            }
        );
        let _ = writeln!(
            out,
            "- Accessibility violations / incomplete / passes: `{}` / `{}` / `{}`",
            art.accessibility_violation_total,
            art.accessibility_incomplete_total,
            art.accessibility_pass_total
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Tool Adoption");
    let _ = writeln!(out);
    let ta = &report.tool_adoption;
    let _ = writeln!(
        out,
        "- Control plane present: `{}`",
        ta.control_plane_present
    );
    let _ = writeln!(out, "- Applicable tools: `{}`", ta.applicable_count);
    let _ = writeln!(out, "- Configured: `{}`", ta.configured_count);
    let _ = writeln!(out, "- CI evidence: `{}`", ta.ci_evidence_count);
    let _ = writeln!(out, "- Artifact verified: `{}`", ta.artifact_verified_count);
    let _ = writeln!(out, "- Replaced count: `{}`", ta.replaced_count);
    let _ = writeln!(
        out,
        "- Missing CI evidence: `{}`",
        if ta.missing.is_empty() {
            "none".into()
        } else {
            ta.missing.join(", ")
        }
    );
    if !ta.items.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "| Tool | Category | Mode | Status | Replaced | Artifacts |"
        );
        let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
        for item in &ta.items {
            let artifacts = if item.artifact_paths.is_empty() {
                "none".into()
            } else {
                item.artifact_paths.join(", ")
            };
            let replaced = if item.replaced_tools.is_empty() {
                "none".into()
            } else {
                item.replaced_tools.join(", ")
            };
            let _ = writeln!(
                out,
                "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |",
                item.id, item.category, item.mode, item.status, replaced, artifacts
            );
        }
    }
    if let Some(art) = &report.security_evidence.artifact {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Security evidence (ingested)");
        let _ = writeln!(out);
        let _ = writeln!(out, "- Source: `{}`", art.path);
        let _ = writeln!(
            out,
            "- Envelope exit code: `{}` · elapsed: `{}` ms · strict: `{}`",
            art.envelope_exit_code, art.elapsed_ms, art.wrapper_strict
        );
        let _ = writeln!(
            out,
            "- Commands — ran: `{}`, skipped: `{}`, failed: `{}`",
            art.commands_ran, art.commands_skipped, art.commands_failed
        );
        if let Some(ts) = &art.generated_at {
            let _ = writeln!(out, "- Generated at: `{}`", ts);
        }
        if let Some(gh) = &art.git_head {
            let _ = writeln!(out, "- Git HEAD (envelope): `{}`", gh);
        }
    }
    if let Some(art) = &report.boundaries.artifact {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Boundary manifest (ingested)");
        let _ = writeln!(out);
        let _ = writeln!(out, "- Path: `{}`", art.path);
        if let Some(v) = &art.stack_version {
            let _ = writeln!(out, "- Stack: `{}` · version: `{}`", art.stack_id, v);
        } else {
            let _ = writeln!(out, "- Stack: `{}`", art.stack_id);
        }
        let _ = writeln!(
            out,
            "- Queue path counts — adapter: `{}`, event_contract: `{}`, generated_type: `{}`, client_marker: `{}`, streaming_exception: `{}`",
            art.adapter_path_count,
            art.event_contract_path_count,
            art.generated_type_path_count,
            art.client_marker_count,
            art.streaming_exception_count
        );
        let _ = writeln!(out, "- Content fingerprint: `{}`", art.content_fingerprint);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Boundary Reclassifications");
    let _ = writeln!(out);
    if report.boundaries.reclassifications.is_empty() {
        let _ = writeln!(
            out,
            "No audited runtime boundary reclassifications declared."
        );
    } else {
        let _ = writeln!(
            out,
            "| Boundary | Status | Files | Lines | Reclassified Caps | Rerun |"
        );
        let _ = writeln!(out, "| --- | --- | ---: | ---: | --- | --- |");
        for boundary in &report.boundaries.reclassifications {
            let caps = if boundary.reclassified_caps.is_empty() {
                "none".into()
            } else {
                boundary.reclassified_caps.join(", ")
            };
            let _ = writeln!(
                out,
                "| `{}` | `{}` | {} | {} | `{}` | `{}` |",
                boundary.id,
                boundary.status,
                boundary.covered_file_count,
                boundary.covered_line_count,
                caps,
                boundary.rerun_command
            );
            if !boundary.missing_checks.is_empty() || !boundary.failed_checks.is_empty() {
                let problems = boundary
                    .missing_checks
                    .iter()
                    .chain(boundary.failed_checks.iter())
                    .take(4)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ");
                let _ = writeln!(out, "<br>Checks: {}", problems.replace('|', "\\|"));
            }
        }
    }
    if let Some(summary) = &report.vibe_coverage {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Vibe Coding Coverage");
        let _ = writeln!(out);
        let _ = writeln!(out, "- Source: `{}`", summary.source_path);
        let _ = writeln!(out, "- Issues: `{}`", summary.issue_count);
        let _ = writeln!(out, "- Source refs: `{}`", summary.source_ref_count);
        let _ = writeln!(
            out,
            "- Unmapped source rows: `{}`",
            summary.unmapped_source_rows
        );
        let _ = writeln!(
            out,
            "- Coverage: detector-backed=`{}` partial=`{}` none=`{}`",
            summary
                .coverage_counts
                .get("detector-backed")
                .copied()
                .unwrap_or(0),
            summary.coverage_counts.get("partial").copied().unwrap_or(0),
            summary.coverage_counts.get("none").copied().unwrap_or(0)
        );
        if !summary.top_gaps.is_empty() {
            let _ = writeln!(out);
            let _ = writeln!(out, "| ID | Coverage | Priority | Next action |");
            let _ = writeln!(out, "| --- | --- | --- | --- |");
            for gap in &summary.top_gaps {
                let _ = writeln!(
                    out,
                    "| `{}` | `{}` | `{}` | {} |",
                    gap.id,
                    gap.coverage,
                    gap.priority,
                    gap.next_action.replace('|', "\\|")
                );
            }
        }
    }
    if let Some(summary) = &report.coverage_evidence {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Coverage Evidence");
        let _ = writeln!(out);
        let _ = writeln!(out, "- Artifact: `{}`", summary.artifact);
        let _ = writeln!(out, "- Status: `{}`", summary.status);
        let _ = writeln!(
            out,
            "- Sources: total=`{}` present=`{}`",
            summary.sources_total, summary.sources_present
        );
        let _ = writeln!(
            out,
            "- Findings: hard=`{}` soft=`{}`",
            summary.hard_findings, summary.soft_findings
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Findings");
    let _ = writeln!(out);
    if report.findings.is_empty() {
        let _ = writeln!(out, "No findings.");
    } else {
        for (idx, finding) in report.findings.iter().enumerate() {
            let loc = if let Some(line) = finding.line {
                format!("{}:{}", finding.path, line)
            } else {
                finding.path.clone()
            };
            let _ = writeln!(
                out,
                "{}. `{}` `{}` `{}`",
                idx + 1,
                finding.severity,
                finding.category,
                loc
            );
            if let Some(rule) = &finding.rule_id {
                let _ = writeln!(out, "   Rule: `{}`", rule);
            }
            let _ = writeln!(
                out,
                "   Check: `{}` `{}` confidence `{:.2}`",
                finding.check_id, finding.hardness, finding.confidence
            );
            if finding.tlr.is_some() || finding.lane.is_some() || finding.owner.is_some() {
                let _ = writeln!(
                    out,
                    "   Route: TLR `{}`, lane `{}`, owner `{}`",
                    finding.tlr.as_deref().unwrap_or("unknown"),
                    finding.lane.as_deref().unwrap_or("unknown"),
                    finding.owner.as_deref().unwrap_or("unmapped"),
                );
            }
            if let Some(url) = &finding.docs_url {
                let _ = writeln!(out, "   Docs: `{}`", url);
            }
            if let Some(term) = &finding.matched_term {
                let _ = writeln!(out, "   Matched term: `{}`", term);
            }
            let _ = writeln!(
                out,
                "   Reason: {}",
                finding.reason.as_deref().unwrap_or(&finding.problem)
            );
            let _ = writeln!(out, "   Fix: {}", finding.agent_fix);
            let _ = writeln!(out, "   Rerun: `{}`", finding.rerun_command);
            let _ = writeln!(out, "   Fingerprint: `{}`", finding.fingerprint);
            if !finding.evidence.is_empty() {
                let _ = writeln!(out, "   Evidence: {}", finding.evidence.join(", "));
            }
        }
    }
    let _ = writeln!(out);
    if let Some(policy) = &report.policy {
        let _ = writeln!(out, "## Policy");
        let _ = writeln!(out);
        let _ = writeln!(out, "- Policy file: `{}`", policy.path);
        let _ = writeln!(out, "- Minimum score: `{}`", policy.minimum_score);
        let _ = writeln!(out, "- Fail on: `{}`", policy.fail_on.join(", "));
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Agent Fix Queue");
    let _ = writeln!(out);
    if report.agent_fix_queue.is_empty() {
        let _ = writeln!(out, "No queued fixes.");
    } else {
        for (idx, item) in report.agent_fix_queue.iter().enumerate() {
            let rule = item
                .rule_id
                .as_ref()
                .map(|s| format!(" `{}`", s))
                .unwrap_or_default();
            let route = match (&item.tlr, &item.lane) {
                (Some(tlr), Some(lane)) => format!(" `{}`/`{}`", tlr, lane),
                _ => String::new(),
            };
            let _ = writeln!(
                out,
                "{}. `{}`{} `{}` - {}",
                idx + 1,
                item.priority,
                rule,
                item.path,
                item.task
            );
            let _ = writeln!(out, "   Route:{}", route);
        }
    }
    out
}
