use super::proof;
use super::repair::{
    increment_risk_summary, now_string, packet_eligibility, packet_risk, proof_lanes, push_unique,
    AppliedEdit, BlockedPacket, RepairArgs, RepairRun, RiskSummary, SkippedEdit,
};
use crate::audit::rules::{RepairEligibility, RepairRisk};
use crate::commands::context_data::RepoCatalog;
use crate::commands::repair_plan::{PlannedEdit, RepairPacket, RepairPlan};
use crate::validation::{self, ArtifactSchema};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Deserialize)]
struct FixtureMarker {
    fixture: bool,
}

#[derive(Debug)]
pub(crate) enum EditOutcome {
    Applied(AppliedEdit),
    Skipped(SkippedEdit),
}

pub fn run_fixture_apply(args: RepairArgs, plan: RepairPlan, max_risk: RepairRisk) -> Result<()> {
    read_fixture_marker(&args.repo)?;
    let catalog = RepoCatalog::load(&args.repo)?;
    let packets = packet_map(&plan);
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
        if !reasons.is_empty() {
            blocked_packets.push(BlockedPacket {
                finding_fingerprint: packet.finding_fingerprint.clone(),
                rule_id: packet.rule_id.clone(),
                risk_level: risk.as_str().to_string(),
                repair_eligibility: eligibility.as_str().to_string(),
                reason: reasons.join("; "),
            });
        }
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

    let proof_evidence_index = if terminal_error.is_none() && !files_written.is_empty() {
        let receipt_dir = args.repo.join("target/jankurai/p13-fixture-proof-receipts");
        let evidence_index_path = args
            .repo
            .join("target/jankurai/p13-fixture-evidence-index.json");
        let proof_args = proof::ProveArgs {
            repo: args.repo.clone(),
            plan: None,
            changed: files_written.iter().map(PathBuf::from).collect(),
            changed_from: None,
            plan_out: args
                .repo
                .join("target/jankurai/p13-fixture-proof-plan.json")
                .display()
                .to_string(),
            plan_md: args
                .repo
                .join("target/jankurai/p13-fixture-proof-plan.md")
                .display()
                .to_string(),
            out_dir: receipt_dir.display().to_string(),
            evidence_index: evidence_index_path.display().to_string(),
            continue_on_error: false,
            allow_unsigned_commands: false,
        };
        if let Err(error) = proof::run_prove(proof_args) {
            terminal_error = Some(error.context("fixture proof verification failed"));
        }
        Some(relative_repo_path(&args.repo, &evidence_index_path))
    } else {
        None
    };

    let status = if terminal_error.is_some() {
        "failed"
    } else if !skipped_edits.is_empty() || applied_edits.is_empty() {
        "blocked"
    } else {
        "complete"
    };
    let notes = fixture_notes(
        &applied_edits,
        &skipped_edits,
        proof_evidence_index.as_deref(),
        terminal_error.is_some(),
    );
    let run = RepairRun {
        schema_version: "1.0.0".to_string(),
        repo: args.repo.display().to_string(),
        plan: args.plan.clone(),
        generated_at: now_string(),
        status: status.to_string(),
        execution_mode: "fixture-apply".to_string(),
        dry_run: false,
        auto_pr_requested: false,
        auto_pr_status: "not-requested".to_string(),
        max_risk: max_risk.as_str().to_string(),
        planned_packets: plan.packets.len(),
        risk_summary,
        blocked_packets,
        applied_edits,
        skipped_edits,
        files_written,
        proof_evidence_index,
        auto_pr_draft: None,
        git_mutation: None,
        github_pr: None,
        proof_lanes: proof_lanes(&plan),
        notes,
    };

    if let Some(path) = args.out.as_deref() {
        validation::write_json(&args.repo, ArtifactSchema::RepairRun, path, &run)?;
    } else {
        validation::validate_serializable(&args.repo, ArtifactSchema::RepairRun, &run)?;
        println!("{}", serde_json::to_string_pretty(&run)?);
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&run))?;
    }

    if let Some(error) = terminal_error {
        return Err(error);
    }

    Ok(())
}

pub(crate) fn apply_planned_edit(
    repo: &Path,
    catalog: &RepoCatalog,
    packets: &HashMap<String, RepairPacket>,
    edit: &PlannedEdit,
    max_risk: RepairRisk,
) -> Result<EditOutcome> {
    let path = normalize_edit_path(&edit.path)?;
    let packet = match packets.get(&edit.finding_fingerprint) {
        Some(packet) => packet,
        None => {
            return Ok(EditOutcome::Skipped(SkippedEdit {
                finding_fingerprint: edit.finding_fingerprint.clone(),
                path,
                reason: "no packet matches finding_fingerprint".to_string(),
            }));
        }
    };
    if normalize_edit_path(&packet.finding_path)? != path {
        return Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path,
            reason: "planned edit path does not match packet finding_path".to_string(),
        }));
    }
    if !path_allowed(&path, &packet.allowed_paths) {
        return Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path,
            reason: "path is outside packet allowed_paths".to_string(),
        }));
    }
    if path_forbidden(&path, &packet.forbidden_paths) {
        return Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path,
            reason: "path is in packet forbidden_paths".to_string(),
        }));
    }
    if is_repo_forbidden_path(&path) {
        return Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path,
            reason: "path is in a forbidden repository zone".to_string(),
        }));
    }
    if catalog
        .generated_zones
        .iter()
        .any(|zone| path_matches(&path, &zone.path))
    {
        return Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path,
            reason: "path is in a generated zone".to_string(),
        }));
    }

    let risk = packet_risk(packet);
    let eligibility = packet_eligibility(packet);
    if !risk.is_allowed_by(max_risk) {
        return Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path,
            reason: "packet risk exceeds the configured max risk".to_string(),
        }));
    }
    if !matches!(
        eligibility,
        RepairEligibility::AutoSafe | RepairEligibility::AgentAssisted
    ) || packet.human_review_required
    {
        return Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path,
            reason: "packet is not eligible for fixture apply".to_string(),
        }));
    }

    match edit.apply_strategy.as_str() {
        "append-text" => apply_append_text(repo, edit, &path),
        "replace-exact" => apply_replace_exact(repo, edit, &path),
        "create-file" => apply_create_file(repo, edit, &path),
        "none" => Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path,
            reason: "apply_strategy is none".to_string(),
        })),
        "review-only" | "regenerate" => Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path,
            reason: format!(
                "apply_strategy `{}` is not writable in fixture mode",
                edit.apply_strategy
            ),
        })),
        other => Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path,
            reason: format!("unknown apply_strategy `{other}`"),
        })),
    }
}

fn apply_append_text(repo: &Path, edit: &PlannedEdit, path: &str) -> Result<EditOutcome> {
    let append_text = match edit.append_text.as_deref() {
        Some(text) => text,
        None => {
            return Ok(EditOutcome::Skipped(SkippedEdit {
                finding_fingerprint: edit.finding_fingerprint.clone(),
                path: path.to_string(),
                reason: "append-text strategy requires append_text".to_string(),
            }));
        }
    };
    let file_path = repo.join(path);
    let mut content = match fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(EditOutcome::Skipped(SkippedEdit {
                finding_fingerprint: edit.finding_fingerprint.clone(),
                path: path.to_string(),
                reason: "target file does not exist for append-text".to_string(),
            }));
        }
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            return Ok(EditOutcome::Skipped(SkippedEdit {
                finding_fingerprint: edit.finding_fingerprint.clone(),
                path: path.to_string(),
                reason: "target file is not valid UTF-8".to_string(),
            }));
        }
        Err(error) => return Err(error.into()),
    };
    let before_sha256 = sha256_text(&content);
    content.push_str(append_text);
    write_text(&file_path, &content)?;
    let after_sha256 = sha256_text(&content);
    Ok(EditOutcome::Applied(AppliedEdit {
        finding_fingerprint: edit.finding_fingerprint.clone(),
        path: path.to_string(),
        apply_strategy: edit.apply_strategy.clone(),
        before_sha256,
        after_sha256,
        status: "applied".to_string(),
    }))
}

fn apply_replace_exact(repo: &Path, edit: &PlannedEdit, path: &str) -> Result<EditOutcome> {
    let match_text = match edit.match_text.as_deref() {
        Some(text) => text,
        None => {
            return Ok(EditOutcome::Skipped(SkippedEdit {
                finding_fingerprint: edit.finding_fingerprint.clone(),
                path: path.to_string(),
                reason: "replace-exact strategy requires match_text".to_string(),
            }));
        }
    };
    let replacement_text = match edit.replacement_text.as_deref() {
        Some(text) => text,
        None => {
            return Ok(EditOutcome::Skipped(SkippedEdit {
                finding_fingerprint: edit.finding_fingerprint.clone(),
                path: path.to_string(),
                reason: "replace-exact strategy requires replacement_text".to_string(),
            }));
        }
    };
    let file_path = repo.join(path);
    let content = match fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(EditOutcome::Skipped(SkippedEdit {
                finding_fingerprint: edit.finding_fingerprint.clone(),
                path: path.to_string(),
                reason: "target file does not exist for replace-exact".to_string(),
            }));
        }
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            return Ok(EditOutcome::Skipped(SkippedEdit {
                finding_fingerprint: edit.finding_fingerprint.clone(),
                path: path.to_string(),
                reason: "target file is not valid UTF-8".to_string(),
            }));
        }
        Err(error) => return Err(error.into()),
    };
    let before_sha256 = sha256_text(&content);
    let occurrences = content.matches(match_text).count();
    if occurrences != 1 {
        return Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path: path.to_string(),
            reason: format!("replace-exact expected exactly one match, found {occurrences}"),
        }));
    }
    let updated = content.replacen(match_text, replacement_text, 1);
    write_text(&file_path, &updated)?;
    let after_sha256 = sha256_text(&updated);
    Ok(EditOutcome::Applied(AppliedEdit {
        finding_fingerprint: edit.finding_fingerprint.clone(),
        path: path.to_string(),
        apply_strategy: edit.apply_strategy.clone(),
        before_sha256,
        after_sha256,
        status: "applied".to_string(),
    }))
}

fn apply_create_file(repo: &Path, edit: &PlannedEdit, path: &str) -> Result<EditOutcome> {
    let create_text = match edit.create_text.as_deref() {
        Some(text) => text,
        None => {
            return Ok(EditOutcome::Skipped(SkippedEdit {
                finding_fingerprint: edit.finding_fingerprint.clone(),
                path: path.to_string(),
                reason: "create-file strategy requires create_text".to_string(),
            }));
        }
    };
    let file_path = repo.join(path);
    if file_path.exists() {
        return Ok(EditOutcome::Skipped(SkippedEdit {
            finding_fingerprint: edit.finding_fingerprint.clone(),
            path: path.to_string(),
            reason: "target file already exists for create-file".to_string(),
        }));
    }
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_text(&file_path, create_text)?;
    Ok(EditOutcome::Applied(AppliedEdit {
        finding_fingerprint: edit.finding_fingerprint.clone(),
        path: path.to_string(),
        apply_strategy: edit.apply_strategy.clone(),
        before_sha256: "missing".to_string(),
        after_sha256: sha256_text(create_text),
        status: "applied".to_string(),
    }))
}

fn read_fixture_marker(repo: &Path) -> Result<FixtureMarker> {
    let path = repo.join("agent/repair-fixture.toml");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read fixture marker {}", path.display()))?;
    let marker: FixtureMarker = toml::from_str(&text)
        .with_context(|| format!("parse fixture marker {}", path.display()))?;
    if !marker.fixture {
        bail!("fixture marker must set `fixture = true`");
    }
    Ok(marker)
}

pub(crate) fn packet_map(plan: &RepairPlan) -> HashMap<String, RepairPacket> {
    let mut map = HashMap::new();
    for packet in &plan.packets {
        map.insert(packet.finding_fingerprint.clone(), packet.clone());
    }
    map
}

fn normalize_edit_path(path: &str) -> Result<String> {
    if path.trim().is_empty() {
        bail!("edit path must not be empty");
    }
    let mut parts = Vec::new();
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        bail!("absolute edit paths are not allowed: `{path}`");
    }
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

fn is_repo_forbidden_path(path: &str) -> bool {
    matches_prefix(path, "reference/")
        || matches_prefix(path, "paper/")
        || matches_prefix(path, "target/")
        || path == "reference"
        || path == "paper"
        || path == "target"
}

fn matches_prefix(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn path_matches(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

fn sha256_text(text: &str) -> String {
    sha256_bytes(text.as_bytes())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{:x}", digest)
}

fn relative_repo_path(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .ok()
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn fixture_notes(
    applied_edits: &[AppliedEdit],
    skipped_edits: &[SkippedEdit],
    proof_evidence_index: Option<&str>,
    failed: bool,
) -> Vec<String> {
    let mut notes = vec![
        "fixture repair execution applies only explicitly allowed patch strategies".to_string(),
        "proof commands are derived from the fixture repo test map and proof lanes".to_string(),
    ];
    notes.push(format!("applied edits: {}", applied_edits.len()));
    notes.push(format!("skipped edits: {}", skipped_edits.len()));
    if let Some(path) = proof_evidence_index {
        notes.push(format!("proof evidence index: `{path}`"));
    } else {
        notes.push("proof did not run because no files were written".to_string());
    }
    if failed {
        notes.push("fixture apply ended in failure after recording receipts".to_string());
    }
    notes
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
    let _ = writeln!(out, "- proof lanes: `{}`", run.proof_lanes.join(", "));
    let _ = writeln!(out, "- notes: `{}`", run.notes.join(", "));
    out
}
