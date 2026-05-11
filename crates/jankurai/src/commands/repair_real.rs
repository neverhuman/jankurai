use super::proof;
use super::repair::{
    increment_risk_summary, now_string, packet_eligibility, packet_risk, proof_lanes, push_unique,
    write_repair_run, AppliedEdit, BlockedPacket, RepairArgs, RepairRun, RiskSummary, SkippedEdit,
};
use super::repair_apply::{apply_planned_edit, packet_map, EditOutcome};
use super::repair_git;
use super::repair_pr;
use crate::commands::context_data::RepoCatalog;
use crate::commands::repair_plan::{PlannedEdit, RepairPlan};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Default, Clone)]
struct PlanAssessment {
    risk_summary: RiskSummary,
    blocked_packets: Vec<BlockedPacket>,
}

#[derive(Debug, Clone)]
struct FileSnapshot {
    path: String,
    existed: bool,
    bytes: Option<Vec<u8>>,
}

pub fn run_real_apply(
    args: RepairArgs,
    plan: RepairPlan,
    max_risk: crate::audit::rules::RepairRisk,
) -> Result<()> {
    if args.auto_pr && !args.git_commit {
        bail!("`--auto-pr` requires `--git-commit` in `--apply` mode");
    }
    if args.pr_draft_out.is_some() || args.pr_draft_md.is_some() {
        bail!("`--pr-draft-out` and `--pr-draft-md` are only supported for dry-run repair");
    }

    let catalog = RepoCatalog::load(&args.repo)?;
    let packets = packet_map(&plan);
    let assessment = assess_plan(&plan, max_risk);
    let draft = repair_pr::build_auto_pr_draft(
        &args,
        &plan,
        max_risk,
        args.out.as_deref(),
        args.md.as_deref(),
        None,
        None,
    )?;

    if draft.draft.status != "draft-only" {
        let run = build_run(
            &args,
            &plan,
            &assessment,
            "blocked",
            if args.auto_pr {
                "blocked"
            } else {
                "not-requested"
            },
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
            None,
        );
        return write_repair_run(&args, &run);
    }

    let mut git_state = None;
    let mut snapshots = Vec::new();
    if args.git_commit {
        let preflight = repair_git::preflight_git_repo(&args.repo, &args.remote, &args.base)
            .context("real repair git preflight failed")?;
        git_state = Some(
            repair_git::create_repair_branch(&args.repo, &preflight, &draft.draft.branch_name)
                .context("create repair branch")?,
        );
    } else {
        snapshots = snapshot_plan_files(&args.repo, &plan.planned_edits)?;
    }

    let mut applied_edits = Vec::new();
    let mut skipped_edits = Vec::new();
    let mut files_written = Vec::new();
    let mut terminal_error: Option<anyhow::Error> = None;

    for edit in &plan.planned_edits {
        match apply_planned_edit(&args.repo, &catalog, &packets, edit, max_risk) {
            Ok(EditOutcome::Applied(applied)) => {
                push_unique(&mut files_written, applied.path.clone());
                applied_edits.push(applied);
            }
            Ok(EditOutcome::Skipped(skipped)) => skipped_edits.push(skipped),
            Err(error) => {
                terminal_error = Some(error);
                break;
            }
        }
    }

    let mut proof_evidence_index = None;
    if terminal_error.is_none() && !files_written.is_empty() && skipped_edits.is_empty() {
        let receipt_dir = args.repo.join("target/jankurai/p13-real-proof-receipts");
        let evidence_index_path = args
            .repo
            .join("target/jankurai/p13-real-evidence-index.json");
        let proof_args = proof::ProveArgs {
            repo: args.repo.clone(),
            plan: None,
            changed: files_written.iter().map(PathBuf::from).collect(),
            changed_from: None,
            plan_out: args
                .repo
                .join("target/jankurai/p13-real-proof-plan.json")
                .display()
                .to_string(),
            plan_md: args
                .repo
                .join("target/jankurai/p13-real-proof-plan.md")
                .display()
                .to_string(),
            out_dir: receipt_dir.display().to_string(),
            evidence_index: evidence_index_path.display().to_string(),
            continue_on_error: false,
            allow_unsigned_commands: false,
        };
        if let Err(error) = proof::run_prove(proof_args) {
            terminal_error = Some(error.context("real apply proof verification failed"));
        } else {
            proof_evidence_index = Some(relative_repo_path(&args.repo, &evidence_index_path));
        }
    }

    if terminal_error.is_some() || !skipped_edits.is_empty() || applied_edits.is_empty() {
        let rollback_result = if let Some(state) = git_state.as_ref() {
            match repair_git::rollback_repair_branch(&args.repo, state) {
                Ok(receipt) => Some(receipt),
                Err(error) => {
                    if terminal_error.is_none() {
                        terminal_error = Some(error);
                    }
                    None
                }
            }
        } else {
            restore_snapshots(&args.repo, &snapshots)?;
            None
        };
        let run = build_run(
            &args,
            &plan,
            &assessment,
            if terminal_error.is_some() {
                "failed"
            } else {
                "blocked"
            },
            if args.auto_pr {
                "blocked"
            } else {
                "not-requested"
            },
            applied_edits,
            skipped_edits,
            files_written,
            proof_evidence_index,
            rollback_result,
            None,
        );
        write_repair_run(&args, &run)?;
        if let Some(error) = terminal_error {
            return Err(error);
        }
        return Ok(());
    }

    let mut git_mutation = None;
    let mut github_pr = None;
    if let Some(state) = git_state.as_ref() {
        let mut receipt = repair_git::commit_repair(
            &args.repo,
            state,
            &files_written,
            &draft.draft.commit_title,
            &draft.draft.pr_body,
        )?;
        if args.auto_pr {
            repair_git::push_branch(&args.repo, &args.remote, &state.head_branch)?;
            receipt.pushed = true;
            if args.github_pr {
                let body_file = repair_git::write_pr_body(&args.repo, &draft.draft.pr_body)?;
                let pr_receipt = repair_git::create_github_draft_pr(
                    &args.repo,
                    &args.remote,
                    &state.base_branch,
                    &state.head_branch,
                    &draft.draft.pr_title,
                    &body_file,
                );
                if pr_receipt.status != "created" {
                    let run = build_run(
                        &args,
                        &plan,
                        &assessment,
                        "failed",
                        "blocked",
                        applied_edits,
                        skipped_edits,
                        files_written,
                        proof_evidence_index,
                        Some(receipt),
                        Some(pr_receipt),
                    );
                    write_repair_run(&args, &run)?;
                    bail!("draft GitHub PR creation failed");
                }
                github_pr = Some(pr_receipt);
            }
        }
        git_mutation = Some(receipt);
    }

    let auto_pr_status = if args.auto_pr {
        if args.github_pr {
            "created"
        } else {
            "prepared"
        }
    } else {
        "not-requested"
    };

    let run = build_run(
        &args,
        &plan,
        &assessment,
        "complete",
        auto_pr_status,
        applied_edits,
        skipped_edits,
        files_written,
        proof_evidence_index,
        git_mutation,
        github_pr,
    );
    write_repair_run(&args, &run)
}

// Repair run receipts intentionally preserve every execution input and output collection.
#[allow(clippy::too_many_arguments)]
fn build_run(
    args: &RepairArgs,
    plan: &RepairPlan,
    assessment: &PlanAssessment,
    status: &str,
    auto_pr_status: &str,
    applied_edits: Vec<AppliedEdit>,
    skipped_edits: Vec<SkippedEdit>,
    files_written: Vec<String>,
    proof_evidence_index: Option<String>,
    git_mutation: Option<repair_git::GitMutationReceipt>,
    github_pr: Option<repair_git::GithubPrReceipt>,
) -> RepairRun {
    RepairRun {
        schema_version: "1.0.0".to_string(),
        repo: args.repo.display().to_string(),
        plan: args.plan.clone(),
        generated_at: now_string(),
        status: status.to_string(),
        execution_mode: "real-apply".to_string(),
        dry_run: false,
        auto_pr_requested: args.auto_pr,
        auto_pr_status: auto_pr_status.to_string(),
        max_risk: args.max_risk.clone(),
        planned_packets: plan.packets.len(),
        risk_summary: assessment.risk_summary.clone(),
        blocked_packets: assessment.blocked_packets.clone(),
        applied_edits,
        skipped_edits,
        files_written,
        proof_evidence_index,
        auto_pr_draft: None,
        git_mutation,
        github_pr,
        proof_lanes: proof_lanes(plan),
        notes: vec![
            "real apply mutates the working tree only after the plan is certified".to_string(),
            "git commit and GitHub draft PR creation remain gated behind explicit flags"
                .to_string(),
        ],
    }
}

fn assess_plan(plan: &RepairPlan, max_risk: crate::audit::rules::RepairRisk) -> PlanAssessment {
    let mut assessment = PlanAssessment::default();
    for packet in &plan.packets {
        let risk = packet_risk(packet);
        let eligibility = packet_eligibility(packet);
        increment_risk_summary(&mut assessment.risk_summary, risk);
        let mut reasons = Vec::new();
        if !risk.is_allowed_by(max_risk) {
            reasons.push(format!(
                "risk {} exceeds max {}",
                risk.as_str(),
                max_risk.as_str()
            ));
        }
        if packet.human_review_required {
            reasons.push("packet requires human review".to_string());
        }
        if !eligibility.allows_auto_pr() {
            reasons.push(format!("repair eligibility is {}", eligibility.as_str()));
        }
        if !reasons.is_empty() {
            assessment.blocked_packets.push(BlockedPacket {
                finding_fingerprint: packet.finding_fingerprint.clone(),
                rule_id: packet.rule_id.clone(),
                risk_level: risk.as_str().to_string(),
                repair_eligibility: eligibility.as_str().to_string(),
                reason: reasons.join("; "),
            });
        }
    }
    assessment
}

fn snapshot_plan_files(repo: &Path, edits: &[PlannedEdit]) -> Result<Vec<FileSnapshot>> {
    let mut snapshots = Vec::new();
    let mut seen = Vec::new();
    for edit in edits {
        let path = normalize_edit_path(&edit.path)?;
        if seen.contains(&path) {
            continue;
        }
        seen.push(path.clone());
        let file_path = repo.join(&path);
        let snapshot = match fs::read(&file_path) {
            Ok(bytes) => FileSnapshot {
                path,
                existed: true,
                bytes: Some(bytes),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => FileSnapshot {
                path,
                existed: false,
                bytes: None,
            },
            Err(error) => return Err(error.into()),
        };
        snapshots.push(snapshot);
    }
    Ok(snapshots)
}

fn restore_snapshots(repo: &Path, snapshots: &[FileSnapshot]) -> Result<()> {
    for snapshot in snapshots {
        let file_path = repo.join(&snapshot.path);
        if snapshot.existed {
            if let Some(bytes) = &snapshot.bytes {
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&file_path, bytes)?;
            }
        } else if file_path.exists() {
            fs::remove_file(&file_path)?;
        }
    }
    Ok(())
}

fn normalize_edit_path(path: &str) -> Result<String> {
    if path.trim().is_empty() {
        bail!("edit path must not be empty");
    }
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        bail!("absolute edit paths are not allowed: `{path}`");
    }
    let mut parts = Vec::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("path traversal is not allowed: `{path}`");
            }
        }
    }
    let normalized = parts.join("/");
    if normalized.is_empty() {
        bail!("edit path must not be empty");
    }
    Ok(normalized)
}

fn relative_repo_path(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .ok()
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
