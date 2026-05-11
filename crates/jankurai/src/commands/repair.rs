use crate::audit::rules::{self, RepairEligibility, RepairRisk};
use crate::commands::repair_git::{GitMutationReceipt, GithubPrReceipt};
use crate::commands::repair_plan::RepairPlan;
use crate::validation::{self, ArtifactSchema};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct RepairArgs {
    pub repo: PathBuf,
    pub plan: String,
    pub dry_run: bool,
    pub fixture_apply: bool,
    pub apply: bool,
    pub auto_pr: bool,
    pub git_commit: bool,
    pub github_pr: bool,
    pub remote: String,
    pub base: String,
    pub pr_draft_out: Option<String>,
    pub pr_draft_md: Option<String>,
    pub max_risk: String,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairRun {
    pub schema_version: String,
    pub repo: String,
    pub plan: String,
    pub generated_at: String,
    pub status: String,
    pub execution_mode: String,
    pub dry_run: bool,
    pub auto_pr_requested: bool,
    pub auto_pr_status: String,
    pub max_risk: String,
    pub planned_packets: usize,
    pub risk_summary: RiskSummary,
    pub blocked_packets: Vec<BlockedPacket>,
    pub applied_edits: Vec<AppliedEdit>,
    pub skipped_edits: Vec<SkippedEdit>,
    pub files_written: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_evidence_index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_pr_draft: Option<AutoPrDraftSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_mutation: Option<GitMutationReceipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_pr: Option<GithubPrReceipt>,
    pub proof_lanes: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutoPrDraftSummary {
    pub status: String,
    pub branch_name: String,
    pub commit_title: String,
    pub pr_title: String,
    pub planned_changed_paths: Vec<String>,
    pub proof_lanes: Vec<String>,
    pub artifact_links: Vec<String>,
    pub git_mutation_allowed: bool,
    pub github_mutation_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RiskSummary {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
    pub critical: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlockedPacket {
    pub finding_fingerprint: String,
    pub rule_id: String,
    pub risk_level: String,
    pub repair_eligibility: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppliedEdit {
    pub finding_fingerprint: String,
    pub path: String,
    pub apply_strategy: String,
    pub before_sha256: String,
    pub after_sha256: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedEdit {
    pub finding_fingerprint: String,
    pub path: String,
    pub reason: String,
}

pub fn run(args: RepairArgs) -> Result<()> {
    let max_risk = RepairRisk::parse(&args.max_risk)
        .with_context(|| format!("unknown --max-risk `{}`", args.max_risk))?;
    let plan_text = fs::read_to_string(&args.plan)
        .with_context(|| format!("read repair plan {}", args.plan))?;
    let plan: RepairPlan = serde_json::from_str(&plan_text)
        .with_context(|| format!("parse repair plan {}", args.plan))?;

    // Validate mode exclusivity.
    let selected_modes = [args.dry_run, args.fixture_apply, args.apply]
        .iter()
        .filter(|enabled| **enabled)
        .count();
    if selected_modes > 1 {
        bail!("choose exactly one repair execution mode: `--dry-run`, `--fixture-apply`, or `--apply`");
    }
    if args.auto_pr && args.fixture_apply {
        bail!("`--auto-pr` cannot be combined with `--fixture-apply`");
    }
    if args.git_commit && !args.apply {
        bail!("`--git-commit` requires `--apply`");
    }
    if args.github_pr && !args.git_commit {
        bail!("`--github-pr` requires `--git-commit`");
    }
    if args.github_pr && !args.auto_pr {
        bail!("`--github-pr` requires `--auto-pr`");
    }
    if !args.auto_pr && (args.pr_draft_out.is_some() || args.pr_draft_md.is_some()) {
        bail!("`--pr-draft-out` and `--pr-draft-md` require `--auto-pr`");
    }
    if !args.dry_run && !args.fixture_apply && !args.apply {
        bail!(
            "repair execution is dry-run only unless `--fixture-apply` or gated `--apply` is used"
        );
    }

    // Environment gates for real mutation.
    if args.apply && !env_flag("JANKURAI_ALLOW_REPAIR_APPLY") {
        bail!("real repair apply requires environment gate `JANKURAI_ALLOW_REPAIR_APPLY=1`");
    }
    if args.git_commit && !env_flag("JANKURAI_ALLOW_GIT_MUTATION") {
        bail!("git mutation requires environment gate `JANKURAI_ALLOW_GIT_MUTATION=1`");
    }
    if args.github_pr && !env_flag("JANKURAI_ALLOW_GITHUB_PR") {
        bail!("GitHub draft PR creation requires environment gate `JANKURAI_ALLOW_GITHUB_PR=1`");
    }

    // Dispatch to the appropriate execution mode.
    if args.fixture_apply {
        return crate::commands::repair_apply::run_fixture_apply(args, plan, max_risk);
    }
    if args.apply {
        return crate::commands::repair_real::run_real_apply(args, plan, max_risk);
    }

    // Dry-run path (unchanged logic).
    let mut risk_summary = RiskSummary::default();
    let mut blocked_packets = Vec::new();
    for packet in &plan.packets {
        let risk = packet_risk(packet);
        let eligibility = packet_eligibility(packet);
        increment_risk_summary(&mut risk_summary, risk);
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
        if args.auto_pr && !reasons.is_empty() {
            blocked_packets.push(BlockedPacket {
                finding_fingerprint: packet.finding_fingerprint.clone(),
                rule_id: packet.rule_id.clone(),
                risk_level: risk.as_str().to_string(),
                repair_eligibility: eligibility.as_str().to_string(),
                reason: reasons.join("; "),
            });
        }
    }
    let auto_pr_status = if !args.auto_pr {
        "not-requested"
    } else if blocked_packets.is_empty() {
        "eligible-dry-run-only"
    } else {
        "blocked"
    };
    let proof_lanes = proof_lanes(&plan);
    let auto_pr_draft = if args.auto_pr {
        let draft = crate::commands::repair_pr::build_auto_pr_draft(
            &args,
            &plan,
            max_risk,
            args.out.as_deref(),
            args.md.as_deref(),
            args.pr_draft_out.as_deref(),
            args.pr_draft_md.as_deref(),
        )?;
        if let Some(path) = args.pr_draft_out.as_deref() {
            validation::write_json(
                &args.repo,
                ArtifactSchema::RepairPrDraft,
                path,
                &draft.draft,
            )?;
        } else {
            validation::validate_serializable(
                &args.repo,
                ArtifactSchema::RepairPrDraft,
                &draft.draft,
            )?;
        }
        if let Some(path) = args.pr_draft_md.as_deref() {
            crate::render::write_markdown(path, &draft.markdown)?;
        }
        Some(draft.summary)
    } else {
        None
    };
    let run = RepairRun {
        schema_version: "1.0.0".to_string(),
        repo: args.repo.display().to_string(),
        plan: args.plan.clone(),
        generated_at: now_string(),
        status: "complete".to_string(),
        execution_mode: "dry-run".to_string(),
        dry_run: args.dry_run,
        auto_pr_requested: args.auto_pr,
        auto_pr_status: auto_pr_status.to_string(),
        max_risk: max_risk.as_str().to_string(),
        planned_packets: plan.packets.len(),
        risk_summary,
        blocked_packets,
        applied_edits: Vec::new(),
        skipped_edits: Vec::new(),
        files_written: Vec::new(),
        proof_evidence_index: None,
        auto_pr_draft,
        git_mutation: None,
        github_pr: None,
        proof_lanes,
        notes: vec![
            "repair execution is intentionally dry-run only in this workspace".to_string(),
            "the plan can be reviewed without mutating files".to_string(),
            "auto-pr requests are evaluated for eligibility but no branch, commit, or PR is created"
                .to_string(),
        ],
    };
    write_repair_run(&args, &run)?;
    Ok(())
}

pub(crate) fn write_repair_run(args: &RepairArgs, run: &RepairRun) -> Result<()> {
    if let Some(path) = args.out.as_deref() {
        validation::write_json(&args.repo, ArtifactSchema::RepairRun, path, run)?;
    } else {
        validation::validate_serializable(&args.repo, ArtifactSchema::RepairRun, run)?;
        println!("{}", serde_json::to_string_pretty(run)?);
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(run))?;
    }
    Ok(())
}

fn render_markdown(run: &RepairRun) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Repair Run");
    let _ = writeln!(out);
    let _ = writeln!(out, "- repo: `{}`", run.repo);
    let _ = writeln!(out, "- plan: `{}`", run.plan);
    let _ = writeln!(out, "- status: `{}`", run.status);
    let _ = writeln!(out, "- execution mode: `{}`", run.execution_mode);
    let _ = writeln!(out, "- dry run: `{}`", run.dry_run);
    let _ = writeln!(out, "- auto-pr requested: `{}`", run.auto_pr_requested);
    let _ = writeln!(out, "- auto-pr status: `{}`", run.auto_pr_status);
    let _ = writeln!(out, "- max risk: `{}`", run.max_risk);
    let _ = writeln!(out, "- planned packets: `{}`", run.planned_packets);
    let _ = writeln!(
        out,
        "- risk summary: `low={}, medium={}, high={}, critical={}`",
        run.risk_summary.low,
        run.risk_summary.medium,
        run.risk_summary.high,
        run.risk_summary.critical
    );
    let _ = writeln!(out, "- blocked packets: `{}`", run.blocked_packets.len());
    let _ = writeln!(out, "- applied edits: `{}`", run.applied_edits.len());
    let _ = writeln!(out, "- skipped edits: `{}`", run.skipped_edits.len());
    let _ = writeln!(out, "- files written: `{}`", run.files_written.join(", "));
    if let Some(path) = &run.proof_evidence_index {
        let _ = writeln!(out, "- proof evidence index: `{}`", path);
    }
    if let Some(draft) = &run.auto_pr_draft {
        let _ = writeln!(out, "- auto-pr draft status: `{}`", draft.status);
        let _ = writeln!(out, "- auto-pr draft branch: `{}`", draft.branch_name);
        let _ = writeln!(out, "- auto-pr draft title: `{}`", draft.pr_title);
    }
    if let Some(git) = &run.git_mutation {
        let _ = writeln!(out, "- git mutation status: `{}`", git.status);
        let _ = writeln!(out, "- git base branch: `{}`", git.base_branch);
        let _ = writeln!(out, "- git head branch: `{}`", git.head_branch);
        if let Some(commit_sha) = &git.commit_sha {
            let _ = writeln!(out, "- git commit sha: `{}`", commit_sha);
        }
        let _ = writeln!(out, "- git pushed: `{}`", git.pushed);
        let _ = writeln!(out, "- rollback command: `{}`", git.rollback_command);
    }
    if let Some(pr) = &run.github_pr {
        let _ = writeln!(out, "- github pr status: `{}`", pr.status);
        if let Some(url) = &pr.url {
            let _ = writeln!(out, "- github pr url: `{}`", url);
        }
    }
    let _ = writeln!(out, "- proof lanes: `{}`", run.proof_lanes.join(", "));
    let _ = writeln!(out, "- notes: `{}`", run.notes.join(", "));
    out
}

pub(crate) fn packet_risk(packet: &crate::commands::repair_plan::RepairPacket) -> RepairRisk {
    RepairRisk::parse(&packet.risk_level)
        .or_else(|| rules::lookup(&packet.rule_id).map(|rule| rule.repair_risk))
        .unwrap_or_else(|| risk_from_severity(&packet.severity))
}

pub(crate) fn packet_eligibility(
    packet: &crate::commands::repair_plan::RepairPacket,
) -> RepairEligibility {
    match packet.repair_eligibility.as_str() {
        "auto-safe" => RepairEligibility::AutoSafe,
        "agent-assisted" => RepairEligibility::AgentAssisted,
        "human-required" => RepairEligibility::HumanRequired,
        "never-auto" => RepairEligibility::NeverAuto,
        _ => rules::lookup(&packet.rule_id)
            .map(|rule| rule.repair_eligibility)
            .unwrap_or(RepairEligibility::HumanRequired),
    }
}

pub(crate) fn risk_from_severity(severity: &str) -> RepairRisk {
    match severity {
        "low" => RepairRisk::Low,
        "medium" => RepairRisk::Medium,
        "critical" => RepairRisk::Critical,
        _ => RepairRisk::High,
    }
}

pub(crate) fn increment_risk_summary(summary: &mut RiskSummary, risk: RepairRisk) {
    match risk {
        RepairRisk::Low => summary.low += 1,
        RepairRisk::Medium => summary.medium += 1,
        RepairRisk::High => summary.high += 1,
        RepairRisk::Critical => summary.critical += 1,
    }
}

pub(crate) fn proof_lanes(plan: &RepairPlan) -> Vec<String> {
    let mut out = Vec::new();
    for lane in &plan.proof_lanes {
        push_unique(&mut out, lane.clone());
    }
    if out.is_empty() {
        for packet in &plan.packets {
            push_unique(&mut out, packet.lane.clone());
        }
    }
    out
}

pub(crate) fn push_unique(values: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if !value.is_empty() && !values.contains(&value) {
        values.push(value);
    }
}

pub(crate) fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes")
        })
        .unwrap_or(false)
}
