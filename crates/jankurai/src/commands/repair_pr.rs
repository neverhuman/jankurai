use super::repair::{
    now_string, packet_eligibility, packet_risk, push_unique, AutoPrDraftSummary, RepairArgs,
};
use crate::audit::rules::RepairRisk;
use crate::commands::context_data::RepoCatalog;
use crate::commands::repair_plan::{PlannedEdit, RepairPlan};
use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Component, Path};

#[derive(Debug, Clone, Serialize)]
pub struct RepairPrDraft {
    pub schema_version: String,
    pub repo: String,
    pub source_plan: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_repair_run: Option<String>,
    pub generated_at: String,
    pub status: String,
    pub execution_mode: String,
    pub max_risk: String,
    pub branch_name: String,
    pub commit_title: String,
    pub pr_title: String,
    pub pr_body: String,
    pub planned_changed_paths: Vec<String>,
    pub eligible_packets: Vec<DraftPacket>,
    pub blocked_packets: Vec<DraftBlockedPacket>,
    pub proof_lanes: Vec<String>,
    pub artifact_links: Vec<String>,
    pub residual_risk: Vec<String>,
    pub safety_notes: Vec<String>,
    pub git_mutation_allowed: bool,
    pub github_mutation_allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftPacket {
    pub finding_fingerprint: String,
    pub path: String,
    pub rule_id: String,
    pub operation: String,
    pub apply_strategy: String,
    pub risk_level: String,
    pub repair_eligibility: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftBlockedPacket {
    pub finding_fingerprint: String,
    pub path: String,
    pub rule_id: String,
    pub operation: String,
    pub apply_strategy: String,
    pub risk_level: String,
    pub repair_eligibility: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct AutoPrDraftResult {
    pub draft: RepairPrDraft,
    pub summary: AutoPrDraftSummary,
    pub markdown: String,
}

pub fn build_auto_pr_draft(
    args: &RepairArgs,
    plan: &RepairPlan,
    max_risk: RepairRisk,
    repair_run_json: Option<&str>,
    repair_run_md: Option<&str>,
    pr_draft_json: Option<&str>,
    pr_draft_md: Option<&str>,
) -> Result<AutoPrDraftResult> {
    let catalog = RepoCatalog::load(&args.repo)?;
    let edit_map = planned_edit_map(plan);
    let mut eligible_packets = Vec::new();
    let mut blocked_packets = Vec::new();
    let mut planned_changed_paths = Vec::new();
    let mut residual_risk = Vec::new();

    for packet in &plan.packets {
        let edit = match edit_map.get(&packet.finding_fingerprint) {
            Some(edit) => *edit,
            None => {
                let path = normalize_path(&packet.finding_path)?;
                blocked_packets.push(DraftBlockedPacket {
                    finding_fingerprint: packet.finding_fingerprint.clone(),
                    path,
                    rule_id: packet.rule_id.clone(),
                    operation: "none".to_string(),
                    apply_strategy: "none".to_string(),
                    risk_level: packet_risk(packet).as_str().to_string(),
                    repair_eligibility: packet_eligibility(packet).as_str().to_string(),
                    reason: "no planned edit matches finding_fingerprint".to_string(),
                });
                continue;
            }
        };

        let path = normalize_path(&edit.path)?;
        let packet_risk = packet_risk(packet);
        let packet_eligibility = packet_eligibility(packet);
        let mut reasons = Vec::new();
        if !packet_risk.is_allowed_by(max_risk) {
            reasons.push(format!(
                "risk {} exceeds max {}",
                packet_risk.as_str(),
                max_risk.as_str()
            ));
        }
        if !packet_eligibility.allows_auto_pr() {
            reasons.push(format!(
                "repair eligibility is {}",
                packet_eligibility.as_str()
            ));
        }
        if path_is_forbidden(&path) {
            reasons.push("path is in a forbidden repository zone".to_string());
        }
        if path_matches_generated_zone(&catalog, &path) {
            reasons.push("path is in a generated zone".to_string());
        }
        if !path_allowed(&path, &packet.allowed_paths) {
            reasons.push("path is outside allowed_paths".to_string());
        }
        if path_forbidden(&path, &packet.forbidden_paths) {
            reasons.push("path is in forbidden_paths".to_string());
        }
        if !matches!(
            edit.apply_strategy.as_str(),
            "append-text" | "replace-exact" | "create-file"
        ) {
            reasons.push(format!(
                "apply_strategy `{}` is not draft-writable",
                edit.apply_strategy
            ));
        }
        if packet.human_review_required {
            reasons.push("human review is required".to_string());
        }

        if reasons.is_empty() {
            push_unique(&mut planned_changed_paths, path.clone());
            eligible_packets.push(DraftPacket {
                finding_fingerprint: packet.finding_fingerprint.clone(),
                path,
                rule_id: packet.rule_id.clone(),
                operation: edit.operation.clone(),
                apply_strategy: edit.apply_strategy.clone(),
                risk_level: packet_risk.as_str().to_string(),
                repair_eligibility: packet_eligibility.as_str().to_string(),
            });
        } else {
            residual_risk.extend(reasons.iter().cloned());
            blocked_packets.push(DraftBlockedPacket {
                finding_fingerprint: packet.finding_fingerprint.clone(),
                path,
                rule_id: packet.rule_id.clone(),
                operation: edit.operation.clone(),
                apply_strategy: edit.apply_strategy.clone(),
                risk_level: packet_risk.as_str().to_string(),
                repair_eligibility: packet_eligibility.as_str().to_string(),
                reason: reasons.join("; "),
            });
        }
    }

    let status = if eligible_packets.is_empty() || !blocked_packets.is_empty() {
        "blocked"
    } else {
        "draft-only"
    };
    if eligible_packets.is_empty() {
        push_unique(
            &mut residual_risk,
            "no eligible packets remain for draft-only auto-pr".to_string(),
        );
    } else {
        push_unique(
            &mut residual_risk,
            "draft generation remains advisory and does not mutate the repository".to_string(),
        );
    }

    let branch_name = branch_name(plan, &eligible_packets, &blocked_packets);
    let commit_title = normalize_title(&format!(
        "Repair eligible findings ({}, {} blocked)",
        eligible_packets.len(),
        blocked_packets.len()
    ));
    let pr_title = normalize_title(&format!(
        "Jankurai repair draft: {} eligible, {} blocked",
        eligible_packets.len(),
        blocked_packets.len()
    ));
    let artifact_links = artifact_links(
        &plan.source_report,
        repair_run_json,
        repair_run_md,
        pr_draft_json,
        pr_draft_md,
    );
    let draft = RepairPrDraft {
        schema_version: "1.0.0".to_string(),
        repo: args.repo.display().to_string(),
        source_plan: plan.source_report.clone(),
        source_repair_run: repair_run_json.map(|path| path.to_string()),
        generated_at: now_string(),
        status: status.to_string(),
        execution_mode: "dry-run".to_string(),
        max_risk: max_risk.as_str().to_string(),
        branch_name: branch_name.clone(),
        commit_title: commit_title.clone(),
        pr_title: pr_title.clone(),
        pr_body: String::new(),
        planned_changed_paths: planned_changed_paths.clone(),
        eligible_packets,
        blocked_packets,
        proof_lanes: super::repair::proof_lanes(plan),
        artifact_links: artifact_links.clone(),
        residual_risk: dedupe_strings(residual_risk),
        safety_notes: safety_notes(),
        git_mutation_allowed: false,
        github_mutation_allowed: false,
    };
    let pr_body = render_markdown(&draft);
    let draft = RepairPrDraft {
        pr_body: pr_body.clone(),
        ..draft
    };
    let summary = AutoPrDraftSummary {
        status: draft.status.clone(),
        branch_name: draft.branch_name.clone(),
        commit_title: draft.commit_title.clone(),
        pr_title: draft.pr_title.clone(),
        planned_changed_paths: draft.planned_changed_paths.clone(),
        proof_lanes: draft.proof_lanes.clone(),
        artifact_links: draft.artifact_links.clone(),
        git_mutation_allowed: draft.git_mutation_allowed,
        github_mutation_allowed: draft.github_mutation_allowed,
    };
    Ok(AutoPrDraftResult {
        draft,
        summary,
        markdown: pr_body,
    })
}

fn planned_edit_map(plan: &RepairPlan) -> HashMap<String, &PlannedEdit> {
    let mut map = HashMap::new();
    for edit in &plan.planned_edits {
        map.insert(edit.finding_fingerprint.clone(), edit);
    }
    map
}

fn artifact_links(
    source_plan: &str,
    repair_run_json: Option<&str>,
    repair_run_md: Option<&str>,
    pr_draft_json: Option<&str>,
    pr_draft_md: Option<&str>,
) -> Vec<String> {
    let mut out = Vec::new();
    push_unique(&mut out, source_plan.to_string());
    if let Some(path) = repair_run_json {
        push_unique(&mut out, path.to_string());
    }
    if let Some(path) = repair_run_md {
        push_unique(&mut out, path.to_string());
    }
    if let Some(path) = pr_draft_json {
        push_unique(&mut out, path.to_string());
    }
    if let Some(path) = pr_draft_md {
        push_unique(&mut out, path.to_string());
    }
    out
}

fn safety_notes() -> Vec<String> {
    vec![
        "draft generation is non-mutating".to_string(),
        "git branch creation, commits, pushes, and GitHub PRs remain disabled".to_string(),
        "proof commands are not executed during draft generation".to_string(),
        "fixture-apply remains mutually exclusive with auto-pr".to_string(),
        "generated zones, reference, paper, and target remain read-only".to_string(),
    ]
}

fn branch_name(
    plan: &RepairPlan,
    eligible_packets: &[DraftPacket],
    blocked_packets: &[DraftBlockedPacket],
) -> String {
    let mut seed = String::new();
    seed.push_str(&plan.source_report);
    seed.push('|');
    seed.push_str(&plan.target_stack_id);
    seed.push('|');
    for edit in &plan.planned_edits {
        seed.push_str(&edit.finding_fingerprint);
        seed.push('|');
        seed.push_str(&edit.path);
        seed.push('|');
        seed.push_str(&edit.apply_strategy);
        seed.push('|');
    }
    seed.push('|');
    for packet in eligible_packets {
        seed.push_str(&packet.finding_fingerprint);
        seed.push('|');
        seed.push_str(&packet.path);
        seed.push('|');
        seed.push_str(&packet.rule_id);
        seed.push('|');
    }
    seed.push('|');
    for packet in blocked_packets {
        seed.push_str(&packet.finding_fingerprint);
        seed.push('|');
        seed.push_str(&packet.path);
        seed.push('|');
        seed.push_str(&packet.reason);
        seed.push('|');
    }
    let digest = sha256_hex(&seed);
    format!("jankurai/repair/{}", &digest[..12])
}

fn normalize_title(value: &str) -> String {
    let collapsed = value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut title = collapsed.replace(['\t', '\r', '\n'], " ");
    while title.contains("  ") {
        title = title.replace("  ", " ");
    }
    title.trim().chars().take(120).collect()
}

fn normalize_path(path: &str) -> Result<String> {
    if path.trim().is_empty() {
        anyhow::bail!("path must not be empty");
    }
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        anyhow::bail!("absolute paths are not allowed: `{path}`");
    }
    let mut parts = Vec::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!("path traversal is not allowed: `{path}`");
            }
        }
    }
    let normalized = parts.join("/");
    if normalized.is_empty() {
        anyhow::bail!("path must not be empty");
    }
    Ok(normalized)
}

fn path_allowed(path: &str, allowed_paths: &[String]) -> bool {
    allowed_paths
        .iter()
        .any(|allowed| path_matches(path, allowed))
}

fn path_forbidden(path: &str, forbidden_paths: &[String]) -> bool {
    forbidden_paths
        .iter()
        .any(|forbidden| path_matches(path, forbidden))
}

fn path_is_forbidden(path: &str) -> bool {
    matches_prefix(path, "reference/")
        || matches_prefix(path, "paper/")
        || matches_prefix(path, "target/")
        || path == "reference"
        || path == "paper"
        || path == "target"
}

fn path_matches(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn matches_prefix(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn path_matches_generated_zone(catalog: &RepoCatalog, path: &str) -> bool {
    catalog
        .generated_zones
        .iter()
        .any(|zone| path_matches(path, &zone.path))
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        push_unique(&mut out, value);
    }
    out
}

fn sha256_hex(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    format!("{:x}", digest)
}

fn render_markdown(draft: &RepairPrDraft) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# Jankurai Repair PR Draft");
    let _ = writeln!(out);
    let _ = writeln!(out, "- repo: `{}`", draft.repo);
    let _ = writeln!(out, "- source plan: `{}`", draft.source_plan);
    if let Some(path) = &draft.source_repair_run {
        let _ = writeln!(out, "- source repair run: `{}`", path);
    }
    let _ = writeln!(out, "- generated at: `{}`", draft.generated_at);
    let _ = writeln!(out, "- status: `{}`", draft.status);
    let _ = writeln!(out, "- execution mode: `{}`", draft.execution_mode);
    let _ = writeln!(out, "- max risk: `{}`", draft.max_risk);
    let _ = writeln!(out, "- branch name: `{}`", draft.branch_name);
    let _ = writeln!(out, "- commit title: `{}`", draft.commit_title);
    let _ = writeln!(out, "- pr title: `{}`", draft.pr_title);
    let _ = writeln!(
        out,
        "- planned changed paths: `{}`",
        draft.planned_changed_paths.join(", ")
    );
    let _ = writeln!(
        out,
        "- eligible packets: `{}`",
        draft.eligible_packets.len()
    );
    let _ = writeln!(out, "- blocked packets: `{}`", draft.blocked_packets.len());
    let _ = writeln!(out, "- proof lanes: `{}`", draft.proof_lanes.join(", "));
    let _ = writeln!(
        out,
        "- artifact links: `{}`",
        draft.artifact_links.join(", ")
    );
    let _ = writeln!(out, "- residual risk: `{}`", draft.residual_risk.join(", "));
    let _ = writeln!(out, "- safety notes: `{}`", draft.safety_notes.join(", "));
    let _ = writeln!(
        out,
        "- git mutation allowed: `{}`",
        draft.git_mutation_allowed
    );
    let _ = writeln!(
        out,
        "- github mutation allowed: `{}`",
        draft.github_mutation_allowed
    );

    if !draft.eligible_packets.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Eligible Packets");
        for packet in &draft.eligible_packets {
            let _ = writeln!(
                out,
                "- `{}` `{}` -> `{}`",
                packet.rule_id, packet.path, packet.apply_strategy
            );
        }
    }

    if !draft.blocked_packets.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Blocked Packets");
        for packet in &draft.blocked_packets {
            let _ = writeln!(
                out,
                "- `{}` `{}` -> {}",
                packet.rule_id, packet.path, packet.reason
            );
        }
    }

    out
}
