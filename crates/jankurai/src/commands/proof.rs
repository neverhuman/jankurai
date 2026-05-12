use crate::commands::context_data::{push_unique, RepoCatalog};
use crate::commands::repair::now_string;
use crate::commands::score::join_or_none;
use crate::model::{
    ArtifactDigest, ManifestFingerprints, ProofReceipt, RuleCoverage, STANDARD_VERSION,
};
use crate::validation::{self, ArtifactSchema};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ProofPlanArgs {
    pub repo: PathBuf,
    pub changed: Vec<PathBuf>,
    pub changed_from: Option<String>,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProveArgs {
    pub repo: PathBuf,
    pub plan: Option<String>,
    pub changed: Vec<PathBuf>,
    pub changed_from: Option<String>,
    pub plan_out: String,
    pub plan_md: String,
    pub out_dir: String,
    pub evidence_index: String,
    pub continue_on_error: bool,
    /// When set with `JANKURAI_ALLOW_UNSIGNED_PROOF_COMMANDS=1`, run commands not in proof-lanes/test-map.
    pub allow_unsigned_commands: bool,
}

#[derive(Debug, Clone)]
pub struct ProofVerifyArgs {
    pub repo: PathBuf,
    pub plan: String,
    pub evidence_index: String,
    pub out: String,
    pub md: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPlan {
    pub schema_version: String,
    pub standard_version: String,
    pub repo_root: String,
    pub git_head: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<String>,
    pub changed_paths: Vec<String>,
    pub matched_owner_map: Vec<String>,
    pub matched_test_map: Vec<String>,
    pub required_lanes: Vec<String>,
    pub optional_lanes: Vec<String>,
    pub skipped_lanes: Vec<String>,
    pub commands: Vec<String>,
    pub expected_artifacts: Vec<String>,
    pub risk_notes: Vec<String>,
    pub human_approval_requirements: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route_decisions: Vec<RouteDecision>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub planned_runs: Vec<PlannedRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_lane_entries: Vec<SkippedLaneEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    pub changed_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub match_kind: String,
    pub specificity: usize,
    pub decision: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub residual_risk: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedLaneEntry {
    pub lane: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedRun {
    pub lane: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub changed_paths: Vec<String>,
    pub artifacts: Vec<String>,
    pub residual_risk: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEvidenceIndex {
    pub schema_version: String,
    pub generated_at: String,
    pub repo_root: String,
    pub git_head: String,
    pub plan_path: String,
    pub plan_digest: String,
    pub manifest_fingerprints: ManifestFingerprints,
    pub receipt_dir: String,
    pub log_dir: String,
    pub commands: Vec<String>,
    pub receipts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_digests: Vec<ArtifactDigest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log_digests: Vec<ArtifactDigest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_digests: Vec<ArtifactDigest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_digests: Vec<ArtifactDigest>,
    pub logs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coverage_verdicts: Vec<RuleCoverage>,
    pub failed_receipts: Vec<String>,
    pub skipped_lanes: Vec<String>,
    pub risk_notes: Vec<String>,
    pub human_approval_requirements: Vec<String>,
    pub changed_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ux_qa_report_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ux_qa_report_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_evidence_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_score_json_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage_audit_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sarif_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_step_summary_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_queue_jsonl_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundaries_manifest_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofVerification {
    pub schema_version: String,
    pub standard_version: String,
    pub generated_at: String,
    pub repo_root: String,
    pub plan_path: String,
    pub evidence_index_path: String,
    pub plan_digest: String,
    pub manifest_fingerprints: ManifestFingerprints,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_digests: Vec<ArtifactDigest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log_digests: Vec<ArtifactDigest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_digests: Vec<ArtifactDigest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_digests: Vec<ArtifactDigest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coverage_verdicts: Vec<RuleCoverage>,
    pub verdict: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<String>,
}

pub fn run_lane(args: ProofPlanArgs) -> Result<()> {
    let plan = build_proof_plan(&args.repo, &args.changed, args.changed_from.as_deref())?;
    write_plan(&args.repo, &plan, args.out.as_deref(), args.md.as_deref())?;
    Ok(())
}

pub fn run_proof(args: ProofPlanArgs) -> Result<()> {
    run_lane(args)
}

pub fn run_prove(args: ProveArgs) -> Result<()> {
    let has_changed_input = !args.changed.is_empty() || args.changed_from.is_some();
    if args.plan.is_some() && has_changed_input {
        anyhow::bail!("use either --plan or --changed/--changed-from, not both");
    }

    let (plan, plan_path_str) = if let Some(plan_path) = args.plan.as_deref() {
        (
            load_proof_plan(&args.repo, plan_path)?,
            plan_path.to_string(),
        )
    } else if has_changed_input {
        if args.plan_out == "-" {
            anyhow::bail!("--plan-out must be a file path when prove builds a plan");
        }
        let plan = build_proof_plan(&args.repo, &args.changed, args.changed_from.as_deref())?;
        write_plan(
            &args.repo,
            &plan,
            Some(args.plan_out.as_str()),
            Some(args.plan_md.as_str()),
        )?;
        let persisted_plan = load_proof_plan(&args.repo, args.plan_out.as_str())?;
        (persisted_plan, args.plan_out.clone())
    } else {
        anyhow::bail!("provide --plan, --changed, or --changed-from");
    };

    execute_proof_plan(args, plan, plan_path_str)
}

pub fn run_proof_verify(args: ProofVerifyArgs) -> Result<()> {
    let plan = load_proof_plan(&args.repo, &args.plan)?;
    let evidence = load_proof_evidence_index(&args.repo, &args.evidence_index)?;
    let verification =
        verify_proof_evidence(&args.repo, &args.plan, &args.evidence_index, plan, evidence)?;
    validation::write_json(
        &args.repo,
        ArtifactSchema::ProofVerification,
        &args.out,
        &verification,
    )?;
    if let Some(parent) = Path::new(&args.md).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    crate::render::write_markdown(&args.md, &render_verification_markdown(&verification))?;
    Ok(())
}

fn load_proof_plan(repo: &Path, plan_path: &str) -> Result<ProofPlan> {
    let plan_text =
        fs::read_to_string(plan_path).with_context(|| format!("read proof plan {plan_path}"))?;
    let plan_json: Value = serde_json::from_str(&plan_text)
        .with_context(|| format!("parse proof plan {plan_path}"))?;
    validation::validate_value(repo, ArtifactSchema::ProofPlan, &plan_json)?;
    Ok(serde_json::from_value(plan_json)?)
}

fn load_proof_evidence_index(repo: &Path, evidence_path: &str) -> Result<ProofEvidenceIndex> {
    let text = fs::read_to_string(evidence_path)
        .with_context(|| format!("read proof evidence index {evidence_path}"))?;
    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("parse proof evidence index {evidence_path}"))?;
    validation::validate_value(repo, ArtifactSchema::EvidenceIndex, &json)?;
    Ok(serde_json::from_value(json)?)
}

fn execute_proof_plan(args: ProveArgs, plan: ProofPlan, plan_path_str: String) -> Result<()> {
    let receipt_dir = PathBuf::from(&args.out_dir);
    let log_dir = args.repo.join("target/jankurai/logs");
    fs::create_dir_all(&receipt_dir)?;
    fs::create_dir_all(&log_dir)?;
    let evidence_index_path = PathBuf::from(&args.evidence_index);
    if let Some(parent) = evidence_index_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let plan_digest = sha256_file(Path::new(&plan_path_str))?;
    let manifest_fingerprints = repo_manifest_fingerprints(&args.repo);
    let runs = if plan.planned_runs.is_empty() {
        fallback_runs_from_plan(&plan)
    } else {
        plan.planned_runs.clone()
    };

    ensure_planned_commands_allowed(&args.repo, &runs, args.allow_unsigned_commands)?;

    let mut receipts = Vec::new();
    let mut receipt_paths = Vec::new();
    let mut command_digests = Vec::new();
    let mut log_digests = Vec::new();
    let mut artifact_digests = Vec::new();
    let mut receipt_digests = Vec::new();
    let mut coverage_verdicts = Vec::new();
    let mut failed_receipts = Vec::new();
    let mut failure: Option<anyhow::Error> = None;

    for (index, run) in runs.iter().enumerate() {
        command_digests.push(ArtifactDigest {
            path: format!("{}::{}", run.lane, run.command),
            sha256: sha256_string(&run.command),
        });
        let receipt = execute_run(
            &args.repo,
            &receipt_dir,
            &log_dir,
            index,
            run,
            plan_path_str.as_str(),
            plan_digest.as_str(),
        )?;
        let receipt_name = receipt_file_name(index, &run.lane, &run.command);
        let receipt_path = receipt_dir.join(receipt_name);
        validation::write_json(
            &args.repo,
            ArtifactSchema::ProofReceipt,
            receipt_path.to_string_lossy().as_ref(),
            &receipt,
        )?;
        receipt_paths.push(display_relative(&args.repo, &receipt_path));
        let receipt_sha256 = sha256_file(&receipt_path)?;
        receipt_digests.push(ArtifactDigest {
            path: display_relative(&args.repo, &receipt_path),
            sha256: receipt_sha256,
        });
        if let Some(log_path) = receipt.log_path.clone() {
            if let Some(log_digest) = receipt.log_sha256.clone() {
                log_digests.push(ArtifactDigest {
                    path: log_path.clone(),
                    sha256: log_digest,
                });
                artifact_digests.push(ArtifactDigest {
                    path: log_path,
                    sha256: receipt.log_sha256.clone().unwrap_or_default(),
                });
            }
        }
        artifact_digests.push(ArtifactDigest {
            path: display_relative(&args.repo, &receipt_path),
            sha256: sha256_file(&receipt_path)?,
        });
        coverage_verdicts.extend(receipt.rules_covered.clone());
        if receipt.exit_code != 0 {
            failed_receipts.push(display_relative(&args.repo, receipt_path.as_path()));
            if args.continue_on_error {
                receipts.push(receipt);
                continue;
            }
            failure = Some(anyhow::anyhow!(
                "proof command `{}` failed in lane `{}` with exit {}",
                run.command,
                run.lane,
                receipt.exit_code
            ));
            receipts.push(receipt);
            break;
        }
        receipts.push(receipt);
    }

    let evidence = ProofEvidenceIndex {
        schema_version: "1.2.0".to_string(),
        generated_at: now_string(),
        repo_root: args.repo.display().to_string(),
        git_head: git_head(&args.repo).unwrap_or_else(|_| "unknown".to_string()),
        plan_path: plan_path_str.clone(),
        plan_digest,
        manifest_fingerprints,
        receipt_dir: receipt_dir.display().to_string(),
        log_dir: log_dir.display().to_string(),
        commands: runs.iter().map(|run| run.command.clone()).collect(),
        receipts: receipt_paths,
        command_digests,
        log_digests,
        artifact_digests,
        receipt_digests,
        logs: receipts
            .iter()
            .filter_map(|receipt| receipt.log_path.clone())
            .collect(),
        coverage_verdicts,
        failed_receipts,
        skipped_lanes: plan.skipped_lanes.clone(),
        risk_notes: plan.risk_notes.clone(),
        human_approval_requirements: plan.human_approval_requirements.clone(),
        changed_paths: plan.changed_paths.clone(),
        ux_qa_report_path: optional_repo_relative_existing(
            &args.repo,
            "target/jankurai/ux-qa.json",
        ),
        ux_qa_report_digest: sha256_file_if_exists(&args.repo, "target/jankurai/ux-qa.json"),
        security_evidence_path: optional_repo_relative_existing(
            &args.repo,
            "target/jankurai/security/evidence.json",
        ),
        repo_score_json_path: optional_repo_relative_existing(&args.repo, "agent/repo-score.json"),
        coverage_audit_path: optional_repo_relative_existing(
            &args.repo,
            "target/jankurai/coverage/coverage-audit.json",
        ),
        sarif_path: optional_repo_relative_existing(&args.repo, "target/jankurai/jankurai.sarif"),
        github_step_summary_path: optional_repo_relative_existing(
            &args.repo,
            "target/jankurai/summary.md",
        ),
        repair_queue_jsonl_path: optional_repo_relative_existing(
            &args.repo,
            "target/jankurai/repair-queue.jsonl",
        ),
        boundaries_manifest_path: optional_repo_relative_existing(
            &args.repo,
            "agent/boundaries.toml",
        ),
    };
    write_evidence_index(&args.repo, &evidence_index_path, &evidence)?;

    if runs.is_empty() {
        anyhow::bail!(
            "proof plan contains no runnable proof commands; update agent/test-map.json \
             or provide a persisted plan with planned_runs"
        );
    }

    if let Some(error) = failure {
        return Err(error);
    }

    if receipts.iter().any(|receipt| receipt.exit_code != 0) {
        anyhow::bail!("one or more proof commands failed");
    }

    Ok(())
}

pub fn build_proof_plan(
    repo: &Path,
    changed: &[PathBuf],
    changed_from: Option<&str>,
) -> Result<ProofPlan> {
    let catalog = RepoCatalog::load(repo)?;
    let changed_paths = normalize_changed_paths(repo, changed, changed_from)?;
    if changed_paths.is_empty() {
        anyhow::bail!("provide at least one --changed path or --changed-from ref");
    }

    let mut matched_owner_map = Vec::new();
    let mut matched_test_map = Vec::new();
    let mut risk_notes = Vec::new();
    let mut human_approval_requirements = BTreeSet::new();
    let mut required_lanes = Vec::new();
    let mut optional_lanes = catalog.proof_lane_names();
    let mut skipped_lanes = BTreeSet::new();
    let mut route_decisions = Vec::new();
    let mut planned_runs: BTreeMap<String, PlannedRun> = BTreeMap::new();
    let lane_names_by_command = lane_names_by_command(&catalog);

    for path in &changed_paths {
        let owner_route = catalog.owner_route_for_path(path);
        let test_route = catalog.test_route_for_path(path);
        if let Some(route) = owner_route.as_ref() {
            push_unique(&mut matched_owner_map, route.prefix.clone());
        } else {
            risk_notes.push(format!("path `{path}` has no owner-map route"));
            human_approval_requirements
                .insert("review unmapped path coverage before merge".to_string());
            skipped_lanes.insert("full".to_string());
        }
        if let Some((route, spec)) = test_route.as_ref() {
            push_unique(&mut matched_test_map, route.prefix.clone());
            let lane_label = lane_names_by_command
                .get(&spec.command)
                .cloned()
                .unwrap_or_else(|| format!("test-map:{}", route.prefix));
            if lane_names_by_command.contains_key(&spec.command) {
                push_unique(&mut required_lanes, lane_label.clone());
            } else {
                push_unique(&mut required_lanes, lane_label.clone());
                risk_notes.push(format!(
                    "test command `{}` for `{}` is not backed by a named proof lane",
                    spec.command, path
                ));
                human_approval_requirements
                    .insert("approve proof commands that are not named lanes".to_string());
                skipped_lanes.insert("full".to_string());
            }
            let owner = catalog.owner_for_path(path).map(|owner| owner.to_string());
            let route_note = route_risk_note(path);
            let entry = planned_runs
                .entry(spec.command.clone())
                .or_insert_with(|| PlannedRun {
                    lane: lane_label.clone(),
                    command: spec.command.clone(),
                    owner: owner.clone(),
                    changed_paths: vec![path.clone()],
                    artifacts: vec![
                        "target/jankurai/logs/*.log".to_string(),
                        "target/jankurai/proof-receipts/*.json".to_string(),
                    ],
                    residual_risk: vec![route_note.clone()],
                    skipped_reason: None,
                });
            push_unique(&mut entry.changed_paths, path.clone());
            push_unique(&mut entry.residual_risk, route_note.clone());
            if entry.owner.is_none() {
                entry.owner = owner;
            }
            route_decisions.push(RouteDecision {
                changed_path: path.clone(),
                owner_route: owner_route.as_ref().map(|route| route.prefix.clone()),
                test_route: Some(route.prefix.clone()),
                lane: Some(lane_label),
                command: Some(spec.command.clone()),
                match_kind: route.match_kind.clone(),
                specificity: route.specificity,
                decision: if owner_route.is_some()
                    && lane_names_by_command.contains_key(&spec.command)
                {
                    "pass".to_string()
                } else {
                    "blocked".to_string()
                },
                reason: if owner_route.is_none() {
                    format!(
                        "test-map prefix `{}` matched but owner-map coverage is missing",
                        route.prefix
                    )
                } else if lane_names_by_command.contains_key(&spec.command) {
                    format!(
                        "test-map prefix `{}` matched and is backed by a named proof lane",
                        route.prefix
                    )
                } else {
                    format!(
                        "test-map prefix `{}` matched but command `{}` is not a named proof lane",
                        route.prefix, spec.command
                    )
                },
                residual_risk: vec![route_note],
            });
            continue;
        }
        risk_notes.push(format!("path `{path}` has no test-map proof route"));
        human_approval_requirements.insert("approve proof coverage for unmapped paths".to_string());
        skipped_lanes.insert("full".to_string());
        route_decisions.push(RouteDecision {
            changed_path: path.clone(),
            owner_route: owner_route.as_ref().map(|route| route.prefix.clone()),
            test_route: None,
            lane: None,
            command: None,
            match_kind: owner_route
                .as_ref()
                .map(|route| route.match_kind.clone())
                .unwrap_or_else(|| "none".to_string()),
            specificity: owner_route
                .as_ref()
                .map(|route| route.specificity)
                .unwrap_or(0),
            decision: "blocked".to_string(),
            reason: if owner_route.is_some() {
                "changed path has owner coverage but no test-map proof route".to_string()
            } else {
                "changed path lacks owner and test-map routes".to_string()
            },
            residual_risk: vec![route_risk_note(path)],
        });
    }

    let used_real_lanes: BTreeSet<String> = required_lanes
        .iter()
        .filter(|lane| {
            catalog
                .proof_lanes
                .iter()
                .any(|candidate| candidate.name == **lane)
        })
        .cloned()
        .collect();
    for lane in &catalog.proof_lanes {
        if !used_real_lanes.contains(&lane.name) {
            skipped_lanes.insert(lane.name.clone());
        }
    }
    optional_lanes.retain(|lane| !required_lanes.iter().any(|required| required == lane));
    let mut commands = Vec::new();
    let mut planned_runs = planned_runs.into_values().collect::<Vec<_>>();
    planned_runs.sort_by(|left, right| {
        left.lane
            .cmp(&right.lane)
            .then(left.command.cmp(&right.command))
    });
    for run in &planned_runs {
        push_unique(&mut commands, run.command.clone());
    }
    let expected_artifacts = vec![
        "target/jankurai/proof-plan.json".to_string(),
        "target/jankurai/proof-plan.md".to_string(),
        "target/jankurai/proof-receipts/*.json".to_string(),
        "target/jankurai/logs/*.log".to_string(),
        "target/jankurai/evidence-index.json".to_string(),
    ];
    let skipped_lanes_vec: Vec<String> = skipped_lanes.iter().cloned().collect();
    let skipped_lane_entries: Vec<SkippedLaneEntry> = skipped_lanes_vec
        .iter()
        .map(|lane| SkippedLaneEntry {
            lane: lane.clone(),
            reason: skipped_lane_reason(lane.as_str(), &risk_notes),
        })
        .collect();
    let git_head = git_head(repo).unwrap_or_else(|_| "unknown".to_string());
    Ok(ProofPlan {
        schema_version: "1.0.0".to_string(),
        standard_version: STANDARD_VERSION.to_string(),
        repo_root: repo.display().to_string(),
        git_head,
        base_ref: changed_from.map(|value| value.to_string()),
        changed_paths,
        matched_owner_map,
        matched_test_map,
        required_lanes,
        optional_lanes,
        skipped_lanes: skipped_lanes_vec,
        commands,
        expected_artifacts,
        risk_notes,
        human_approval_requirements: human_approval_requirements.into_iter().collect(),
        route_decisions,
        planned_runs,
        skipped_lane_entries,
    })
}

fn ensure_planned_commands_allowed(
    repo: &Path,
    runs: &[PlannedRun],
    allow_unsigned: bool,
) -> Result<()> {
    let env_ok = std::env::var("JANKURAI_ALLOW_UNSIGNED_PROOF_COMMANDS")
        .map(|v| v == "1")
        .unwrap_or(false);
    if allow_unsigned && env_ok {
        return Ok(());
    }
    let catalog = RepoCatalog::load(repo)?;
    let allow = catalog.allowed_proof_commands();
    for run in runs {
        let normalized = RepoCatalog::normalize_proof_command(&run.command);
        if !allow.contains(&normalized) {
            anyhow::bail!(
                "proof command not in agent proof-lanes or test-map allowlist: `{}`\n\
                 hint: entries must match after trimming and collapsing whitespace; \
                 or pass --allow-unsigned-commands with JANKURAI_ALLOW_UNSIGNED_PROOF_COMMANDS=1",
                run.command
            );
        }
    }
    Ok(())
}

fn skipped_lane_reason(lane: &str, risk_notes: &[String]) -> String {
    if lane == "full" {
        if risk_notes
            .iter()
            .any(|n| n.contains("no test-map proof route"))
        {
            "changed path lacks test-map proof route".into()
        } else if risk_notes.iter().any(|n| n.contains("no owner-map route")) {
            "changed path lacks owner-map route".into()
        } else if risk_notes
            .iter()
            .any(|n| n.contains("not backed by a named proof lane"))
        {
            "test-map command not listed in proof-lanes.toml".into()
        } else {
            "merge-grade proof lane blocked for current routing".into()
        }
    } else {
        "not required for current changed paths".into()
    }
}

fn proof_run_id(
    plan_path: &str,
    index: usize,
    lane: &str,
    command: &str,
    started_secs: u64,
) -> String {
    let raw = format!("{plan_path}|{index}|{lane}|{command}|{started_secs}");
    let digest = Sha256::digest(raw.as_bytes());
    digest
        .iter()
        .take(8)
        .map(|b| format!("{:02x}", b))
        .collect()
}

fn write_plan(repo: &Path, plan: &ProofPlan, out: Option<&str>, md: Option<&str>) -> Result<()> {
    if let Some(path) = out {
        validation::write_json(repo, ArtifactSchema::ProofPlan, path, plan)?;
    } else {
        println!("{}", serde_json::to_string_pretty(plan)?);
    }
    if let Some(path) = md {
        crate::render::write_markdown(path, &render_markdown(plan))?;
    }
    Ok(())
}

fn write_evidence_index(repo: &Path, path: &Path, evidence: &ProofEvidenceIndex) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    validation::write_json(
        repo,
        ArtifactSchema::EvidenceIndex,
        path.to_string_lossy().as_ref(),
        evidence,
    )?;
    Ok(())
}

fn verify_proof_evidence(
    repo: &Path,
    plan_path: &str,
    evidence_path: &str,
    plan: ProofPlan,
    evidence: ProofEvidenceIndex,
) -> Result<ProofVerification> {
    let mut issues = Vec::new();
    let current_plan_digest = sha256_file(Path::new(plan_path))?;
    if evidence.plan_digest != current_plan_digest {
        issues.push(format!(
            "plan digest mismatch: evidence `{}` vs current `{}`",
            evidence.plan_digest, current_plan_digest
        ));
    }
    if evidence.changed_paths != plan.changed_paths {
        issues.push("evidence changed_paths do not match plan changed_paths".to_string());
    }

    let current_manifest_fingerprints = repo_manifest_fingerprints(repo);
    push_manifest_mismatch(
        &mut issues,
        "owner_map",
        current_manifest_fingerprints.owner_map.as_deref(),
        evidence.manifest_fingerprints.owner_map.as_deref(),
    );
    push_manifest_mismatch(
        &mut issues,
        "test_map",
        current_manifest_fingerprints.test_map.as_deref(),
        evidence.manifest_fingerprints.test_map.as_deref(),
    );
    push_manifest_mismatch(
        &mut issues,
        "generated_zones",
        current_manifest_fingerprints.generated_zones.as_deref(),
        evidence.manifest_fingerprints.generated_zones.as_deref(),
    );
    push_manifest_mismatch(
        &mut issues,
        "boundaries",
        current_manifest_fingerprints.boundaries.as_deref(),
        evidence.manifest_fingerprints.boundaries.as_deref(),
    );
    push_manifest_mismatch(
        &mut issues,
        "proof_lanes",
        current_manifest_fingerprints.proof_lanes.as_deref(),
        evidence.manifest_fingerprints.proof_lanes.as_deref(),
    );
    push_manifest_mismatch(
        &mut issues,
        "standard_version",
        current_manifest_fingerprints.standard_version.as_deref(),
        evidence.manifest_fingerprints.standard_version.as_deref(),
    );

    let receipt_digests = evidence.receipt_digests.clone();
    let mut command_digests = Vec::new();
    let mut log_digests = Vec::new();
    let mut artifact_digests = Vec::new();
    let mut coverage_verdicts = Vec::new();

    for receipt_rel in &evidence.receipts {
        let receipt_path = repo.join(receipt_rel);
        if !receipt_path.is_file() {
            issues.push(format!("missing proof receipt `{receipt_rel}`"));
            continue;
        }
        let receipt_text = fs::read_to_string(&receipt_path)
            .with_context(|| format!("read proof receipt {}", receipt_path.display()))?;
        let receipt_json: Value = serde_json::from_str(&receipt_text)
            .with_context(|| format!("parse proof receipt {}", receipt_path.display()))?;
        validation::validate_value(repo, ArtifactSchema::ProofReceipt, &receipt_json)?;
        let receipt: ProofReceipt = serde_json::from_value(receipt_json.clone())?;

        let receipt_digest = sha256_file(&receipt_path)?;
        artifact_digests.push(ArtifactDigest {
            path: receipt_rel.clone(),
            sha256: receipt_digest.clone(),
        });
        if let Some(expected) = receipt_digests
            .iter()
            .find(|digest| digest.path == *receipt_rel)
        {
            if expected.sha256 != receipt_digest {
                issues.push(format!(
                    "receipt digest mismatch for `{receipt_rel}`: evidence `{}` vs actual `{}`",
                    expected.sha256, receipt_digest
                ));
            }
        } else {
            issues.push(format!("missing receipt digest entry for `{receipt_rel}`"));
        }

        let command_digest = sha256_string(&receipt.command);
        command_digests.push(ArtifactDigest {
            path: format!("{}::{}", receipt.lane, receipt.command),
            sha256: command_digest.clone(),
        });
        if receipt.command_digest.as_deref() != Some(command_digest.as_str()) {
            issues.push(format!("command digest mismatch for `{}`", receipt.lane));
        }

        if let Some(log_rel) = receipt.log_path.as_deref() {
            let log_path = repo.join(log_rel);
            if !log_path.is_file() {
                issues.push(format!("missing proof log `{log_rel}`"));
            } else {
                let log_digest = sha256_file(&log_path)?;
                log_digests.push(ArtifactDigest {
                    path: log_rel.to_string(),
                    sha256: log_digest.clone(),
                });
                artifact_digests.push(ArtifactDigest {
                    path: log_rel.to_string(),
                    sha256: log_digest.clone(),
                });
                if receipt.log_sha256.as_deref() != Some(log_digest.as_str()) {
                    issues.push(format!("log digest mismatch for `{log_rel}`"));
                }
            }
        } else {
            issues.push(format!("receipt `{receipt_rel}` missing log path"));
        }

        coverage_verdicts.extend(receipt.rules_covered.clone());
        if receipt.exit_code != 0 {
            issues.push(format!(
                "receipt `{receipt_rel}` exited with status {}",
                receipt.exit_code
            ));
        }
        if receipt.plan_digest.as_deref() != Some(current_plan_digest.as_str()) {
            issues.push(format!("receipt `{receipt_rel}` plan digest mismatch"));
        }
    }

    let verdict = classify_verdict(&issues);
    Ok(ProofVerification {
        schema_version: "1.0.0".to_string(),
        standard_version: STANDARD_VERSION.to_string(),
        generated_at: now_string(),
        repo_root: repo.display().to_string(),
        plan_path: plan_path.to_string(),
        evidence_index_path: evidence_path.to_string(),
        plan_digest: current_plan_digest,
        manifest_fingerprints: current_manifest_fingerprints,
        command_digests: if command_digests.is_empty() {
            evidence.command_digests.clone()
        } else {
            command_digests
        },
        log_digests: if log_digests.is_empty() {
            evidence.log_digests.clone()
        } else {
            log_digests
        },
        receipt_digests,
        artifact_digests: if artifact_digests.is_empty() {
            evidence.artifact_digests.clone()
        } else {
            artifact_digests
        },
        coverage_verdicts: if coverage_verdicts.is_empty() {
            evidence.coverage_verdicts.clone()
        } else {
            coverage_verdicts
        },
        verdict,
        issues,
    })
}

fn push_manifest_mismatch(
    issues: &mut Vec<String>,
    name: &str,
    current: Option<&str>,
    evidence: Option<&str>,
) {
    if current != evidence {
        issues.push(format!(
            "manifest fingerprint mismatch for {name}: evidence `{}` vs current `{}`",
            evidence.unwrap_or("missing"),
            current.unwrap_or("missing")
        ));
    }
}

fn classify_verdict(issues: &[String]) -> String {
    if issues.is_empty() {
        return "pass".to_string();
    }
    if issues
        .iter()
        .any(|issue| issue.contains("missing") || issue.contains("missing required"))
    {
        return "blocked".to_string();
    }
    if issues
        .iter()
        .any(|issue| issue.contains("mismatch") || issue.contains("stale"))
    {
        return "stale".to_string();
    }
    if issues
        .iter()
        .any(|issue| issue.contains("exited with status"))
    {
        return "incomplete".to_string();
    }
    "advisory".to_string()
}

fn render_markdown(plan: &ProofPlan) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Proof Plan");
    let _ = writeln!(out);
    let _ = writeln!(out, "- repo: `{}`", plan.repo_root);
    let _ = writeln!(out, "- git head: `{}`", plan.git_head);
    if let Some(base_ref) = &plan.base_ref {
        let _ = writeln!(out, "- base ref: `{}`", base_ref);
    }
    let _ = writeln!(out, "- changed: `{}`", join_or_none(&plan.changed_paths));
    let _ = writeln!(
        out,
        "- owner map: `{}`",
        join_or_none(&plan.matched_owner_map)
    );
    let _ = writeln!(
        out,
        "- test map: `{}`",
        join_or_none(&plan.matched_test_map)
    );
    let _ = writeln!(
        out,
        "- required lanes: `{}`",
        join_or_none(&plan.required_lanes)
    );
    let _ = writeln!(
        out,
        "- optional lanes: `{}`",
        join_or_none(&plan.optional_lanes)
    );
    let _ = writeln!(
        out,
        "- skipped lanes: `{}`",
        join_or_none(&plan.skipped_lanes)
    );
    if !plan.skipped_lane_entries.is_empty() {
        let _ = writeln!(out, "- skipped lane reasons:");
        for entry in &plan.skipped_lane_entries {
            let _ = writeln!(out, "  - `{}`: {}", entry.lane, entry.reason);
        }
    }
    let _ = writeln!(out, "- commands: `{}`", join_or_none(&plan.commands));
    let _ = writeln!(
        out,
        "- expected artifacts: `{}`",
        join_or_none(&plan.expected_artifacts)
    );
    let _ = writeln!(out, "- risk notes: `{}`", join_or_none(&plan.risk_notes));
    let _ = writeln!(
        out,
        "- human approval: `{}`",
        join_or_none(&plan.human_approval_requirements)
    );
    if !plan.route_decisions.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Route Decisions");
        for decision in &plan.route_decisions {
            let _ = writeln!(out, "- path: `{}`", decision.changed_path);
            if let Some(owner_route) = &decision.owner_route {
                let _ = writeln!(out, "  - owner route: `{}`", owner_route);
            }
            if let Some(test_route) = &decision.test_route {
                let _ = writeln!(out, "  - test route: `{}`", test_route);
            }
            if let Some(lane) = &decision.lane {
                let _ = writeln!(out, "  - lane: `{}`", lane);
            }
            if let Some(command) = &decision.command {
                let _ = writeln!(out, "  - command: `{}`", command);
            }
            let _ = writeln!(
                out,
                "  - match: `{}` specificity `{}` decision `{}`",
                decision.match_kind, decision.specificity, decision.decision
            );
            let _ = writeln!(out, "  - reason: `{}`", decision.reason);
            if !decision.residual_risk.is_empty() {
                let _ = writeln!(
                    out,
                    "  - residual risk: `{}`",
                    join_or_none(&decision.residual_risk)
                );
            }
        }
    }
    for run in &plan.planned_runs {
        let _ = writeln!(out);
        let _ = writeln!(out, "## {}", run.lane);
        let _ = writeln!(out, "- command: `{}`", run.command);
        let _ = writeln!(
            out,
            "- owner: `{}`",
            run.owner.as_deref().unwrap_or("unknown")
        );
        let _ = writeln!(out, "- changed: `{}`", join_or_none(&run.changed_paths));
        let _ = writeln!(out, "- artifacts: `{}`", join_or_none(&run.artifacts));
        let _ = writeln!(
            out,
            "- residual risk: `{}`",
            join_or_none(&run.residual_risk)
        );
        if let Some(reason) = &run.skipped_reason {
            let _ = writeln!(out, "- skipped reason: `{}`", reason);
        }
    }
    out
}

fn render_verification_markdown(verification: &ProofVerification) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Proof Verification");
    let _ = writeln!(out);
    let _ = writeln!(out, "- verdict: `{}`", verification.verdict);
    let _ = writeln!(out, "- plan: `{}`", verification.plan_path);
    let _ = writeln!(
        out,
        "- evidence index: `{}`",
        verification.evidence_index_path
    );
    let _ = writeln!(out, "- plan digest: `{}`", verification.plan_digest);
    let _ = writeln!(
        out,
        "- manifest fingerprints: owner=`{}` test=`{}` generated=`{}` boundaries=`{}` proof-lanes=`{}` standard-version=`{}`",
        verification
            .manifest_fingerprints
            .owner_map
            .as_deref()
            .unwrap_or("missing"),
        verification
            .manifest_fingerprints
            .test_map
            .as_deref()
            .unwrap_or("missing"),
        verification
            .manifest_fingerprints
            .generated_zones
            .as_deref()
            .unwrap_or("missing"),
        verification
            .manifest_fingerprints
            .boundaries
            .as_deref()
            .unwrap_or("missing"),
        verification
            .manifest_fingerprints
            .proof_lanes
            .as_deref()
            .unwrap_or("missing"),
        verification
            .manifest_fingerprints
            .standard_version
            .as_deref()
            .unwrap_or("missing")
    );
    if !verification.issues.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Issues");
        for issue in &verification.issues {
            let _ = writeln!(out, "- {}", issue);
        }
    }
    if !verification.coverage_verdicts.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Coverage");
        for verdict in &verification.coverage_verdicts {
            match verdict {
                RuleCoverage::Rich { rule_id, status } => {
                    let _ = writeln!(out, "- `{}`: `{}`", rule_id, status);
                }
                RuleCoverage::Simple(rule_id) => {
                    let _ = writeln!(out, "- `{}`", rule_id);
                }
            }
        }
    }
    out
}

fn fallback_runs_from_plan(plan: &ProofPlan) -> Vec<PlannedRun> {
    plan.commands
        .iter()
        .enumerate()
        .map(|(index, command)| PlannedRun {
            lane: format!("command-{index}"),
            command: command.clone(),
            owner: None,
            changed_paths: plan.changed_paths.clone(),
            artifacts: vec![
                "target/jankurai/logs/*.log".to_string(),
                "target/jankurai/proof-receipts/*.json".to_string(),
            ],
            residual_risk: plan.risk_notes.clone(),
            skipped_reason: None,
        })
        .collect()
}

fn execute_run(
    repo: &Path,
    receipt_dir: &Path,
    log_dir: &Path,
    index: usize,
    run: &PlannedRun,
    plan_path: &str,
    plan_digest: &str,
) -> Result<ProofReceipt> {
    let started = Instant::now();
    let started_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let run_id = proof_run_id(plan_path, index, &run.lane, &run.command, started_secs);
    let started_at = now_string();
    let command_output = Command::new("bash")
        .arg("-lc")
        .arg(&run.command)
        .current_dir(repo)
        .output()
        .with_context(|| format!("run proof command `{}`", run.command))?;
    let log_file = log_dir.join(log_file_name(index, &run.lane, &run.command));
    let mut log_text = String::new();
    log_text.push_str(&format!("lane: {}\n", run.lane));
    log_text.push_str(&format!("command: {}\n", run.command));
    log_text.push_str(&format!("status: {}\n", command_output.status));
    log_text.push('\n');
    log_text.push_str(&String::from_utf8_lossy(&command_output.stdout));
    if !command_output.stdout.is_empty() && !command_output.stdout.ends_with(b"\n") {
        log_text.push('\n');
    }
    if !command_output.stderr.is_empty() {
        log_text.push_str("\n[stderr]\n");
        log_text.push_str(&String::from_utf8_lossy(&command_output.stderr));
        if !command_output.stderr.ends_with(b"\n") {
            log_text.push('\n');
        }
    }
    fs::write(&log_file, log_text)?;
    let log_sha256 = sha256_file(&log_file)?;
    let exit_code = command_output.status.code().unwrap_or(-1);
    let stdout_stderr_bytes = fs::metadata(&log_file).map(|m| m.len()).ok();
    let retryable = if exit_code != 0 { Some(true) } else { None };
    let receipt_path = display_relative(
        repo,
        &receipt_dir.join(receipt_file_name(index, &run.lane, &run.command)),
    );
    Ok(ProofReceipt {
        schema_version: Some(crate::model::SCHEMA_VERSION.into()),
        standard_version: Some(crate::model::STANDARD_VERSION.into()),
        auditor_version: Some(crate::model::AUDITOR_VERSION.into()),
        receipt_id: Some(format!("proof-{run_id}")),
        lane: run.lane.clone(),
        command: run.command.clone(),
        exit_code,
        elapsed_ms: started.elapsed().as_millis(),
        artifacts: vec![display_relative(repo, &log_file)],
        changed_paths: run.changed_paths.clone(),
        owner: run.owner.clone(),
        skipped_reason: run.skipped_reason.clone(),
        residual_risk: run.residual_risk.clone(),
        log_path: Some(display_relative(repo, &log_file)),
        receipt_path: Some(receipt_path),
        generated_at: Some(started_at.clone()),
        started_at: Some(started_at),
        finished_at: Some(now_string()),
        repo: Some(repo.display().to_string()),
        repo_root: Some(repo.display().to_string()),
        git_head: git_head(repo).ok(),
        dirty_worktree: Some(crate::commands::witness::git_dirty_for_receipt(repo)),
        run_id: Some(run_id),
        plan_path: Some(plan_path.to_string()),
        plan_digest: Some(plan_digest.to_string()),
        command_digest: Some(sha256_string(&run.command)),
        log_sha256: Some(log_sha256.clone()),
        artifact_digests: vec![ArtifactDigest {
            path: display_relative(repo, &log_file),
            sha256: log_sha256,
        }],
        rules_covered: rules_covered_for_run(run),
        retryable,
        stdout_stderr_bytes,
        extensions: serde_json::Map::new(),
    })
}

fn lane_names_by_command(catalog: &RepoCatalog) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for lane in &catalog.proof_lanes {
        map.entry(lane.command.clone())
            .or_insert_with(|| lane.name.clone());
    }
    map
}

fn normalize_changed_paths(
    repo: &Path,
    changed: &[PathBuf],
    changed_from: Option<&str>,
) -> Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    for path in changed {
        let rel = normalize_changed_path(repo, path)?;
        insert_changed_path(&mut paths, rel, path)?;
    }
    if let Some(base_ref) = changed_from {
        for path in crate::audit::changed_paths_from_git(repo, base_ref)? {
            let rel = normalize_changed_path(repo, &path)?;
            insert_changed_path(&mut paths, rel, path.as_path())?;
        }
    }
    Ok(paths.into_iter().collect())
}

fn normalize_changed_path(root: &Path, path: &Path) -> Result<String> {
    if path.is_absolute() {
        if path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
            || !path.starts_with(root)
        {
            anyhow::bail!(
                "changed path `{}` resolves outside repository root `{}`",
                path.display(),
                root.display()
            );
        }
        let rel = path.strip_prefix(root).with_context(|| {
            format!(
                "changed path `{}` resolves outside repository root `{}`",
                path.display(),
                root.display()
            )
        })?;
        return Ok(rel.to_string_lossy().replace('\\', "/"));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!(
            "changed path `{}` resolves outside repository root `{}`",
            path.display(),
            root.display()
        );
    }
    let candidate = root.join(path);
    let rel = candidate.strip_prefix(root).with_context(|| {
        format!(
            "changed path `{}` resolves outside repository root `{}`",
            path.display(),
            root.display()
        )
    })?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

fn insert_changed_path(paths: &mut BTreeSet<String>, rel: String, original: &Path) -> Result<()> {
    let normalized = rel
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string();
    if normalized.is_empty() || normalized == "." {
        anyhow::bail!(
            "changed path `{}` resolves to the repository root; pass explicit changed files \
             or a non-root subdirectory",
            original.display()
        );
    }
    paths.insert(normalized);
    Ok(())
}

fn rules_covered_for_run(run: &PlannedRun) -> Vec<RuleCoverage> {
    let mut rules = Vec::new();
    match run.lane.as_str() {
        "fast" | "audit" => {
            push_rule(&mut rules, "HLT-003-OWNERLESS-PATH");
            push_rule(&mut rules, "HLT-004-UNMAPPED-PROOF");
        }
        "contract" => {
            push_rule(&mut rules, "HLT-002-GENERATED-MUTATION");
            push_rule(&mut rules, "HLT-007-HANDWRITTEN-CONTRACT");
        }
        "db" => {
            push_rule(&mut rules, "HLT-006-DIRECT-DB-WRONG-LAYER");
            push_rule(&mut rules, "HLT-019-STREAMING-RUNTIME-DRIFT");
        }
        "db-migration-analyze" => {
            push_rule(&mut rules, "HLT-021-DESTRUCTIVE-MIGRATION");
        }
        "web" | "ux-qa" => {
            push_rule(&mut rules, "HLT-013-RENDERED-UX-GAP");
            push_rule(&mut rules, "HLT-014-A11Y-GAP");
        }
        "security" => {
            push_rule(&mut rules, "HLT-009-GENERATED-SECURITY");
            push_rule(&mut rules, "HLT-010-SECRET-SPRAWL");
            push_rule(&mut rules, "HLT-011-PROMPT-INJECTION");
            push_rule(&mut rules, "HLT-012-OVERBROAD-AGENCY");
            push_rule(&mut rules, "HLT-016-SUPPLY-CHAIN-DRIFT");
            push_rule(&mut rules, "HLT-020-CI-HARDENING-GAP");
        }
        "observability" => {
            push_rule(&mut rules, "HLT-017-OPAQUE-OBSERVABILITY");
        }
        _ => {}
    }
    rules
}

fn push_rule(rules: &mut Vec<RuleCoverage>, rule_id: &str) {
    if crate::audit::rules::lookup(rule_id).is_none() {
        return;
    }
    if rules
        .iter()
        .any(|coverage| rule_coverage_id(coverage) == rule_id)
    {
        return;
    }
    rules.push(RuleCoverage::Rich {
        rule_id: rule_id.to_string(),
        status: "covered".to_string(),
    });
}

fn rule_coverage_id(coverage: &RuleCoverage) -> &str {
    match coverage {
        RuleCoverage::Rich { rule_id, .. } => rule_id.as_str(),
        RuleCoverage::Simple(rule_id) => rule_id.as_str(),
    }
}

fn receipt_file_name(index: usize, lane: &str, command: &str) -> String {
    format!(
        "{:02}-{}-{}.json",
        index + 1,
        slugify(lane),
        short_hash(command)
    )
}

fn log_file_name(index: usize, lane: &str, command: &str) -> String {
    format!(
        "{:02}-{}-{}.log",
        index + 1,
        slugify(lane),
        short_hash(command)
    )
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn short_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest
        .iter()
        .take(6)
        .map(|byte| format!("{:02x}", byte))
        .collect()
}

fn display_relative(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn optional_repo_relative_existing(repo: &Path, rel_posix: &str) -> Option<String> {
    let path = repo.join(rel_posix);
    if path.is_file() {
        Some(display_relative(repo, path.as_path()))
    } else {
        None
    }
}

fn sha256_file_if_exists(repo: &Path, rel_posix: &str) -> Option<String> {
    let path = repo.join(rel_posix);
    let bytes = fs::read(&path).ok()?;
    Some(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn sha256_string(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

fn repo_manifest_fingerprints(repo: &Path) -> ManifestFingerprints {
    ManifestFingerprints {
        owner_map: sha256_file_if_exists(repo, "agent/owner-map.json"),
        test_map: sha256_file_if_exists(repo, "agent/test-map.json"),
        generated_zones: sha256_file_if_exists(repo, "agent/generated-zones.toml"),
        boundaries: sha256_file_if_exists(repo, "agent/boundaries.toml"),
        proof_lanes: sha256_file_if_exists(repo, "agent/proof-lanes.toml"),
        standard_version: sha256_file_if_exists(repo, "agent/standard-version.toml"),
    }
}

fn route_risk_note(path: &str) -> String {
    format!("route derived from `{path}`")
}

fn git_head(repo: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("resolve git HEAD in {}", repo.display()))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        anyhow::bail!("unable to resolve git HEAD in {}", repo.display());
    }
}

