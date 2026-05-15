use crate::audit::rules::{self, RepairEligibility, RepairRisk};
use crate::commands::context_data::{push_unique, RepoCatalog};
use crate::commands::repair::now_string;
use crate::commands::score::join_or_none;
use crate::validation::{self, ArtifactSchema};
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RepairPlanArgs {
    pub repo: PathBuf,
    pub from: String,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct RepairPlan {
    pub schema_version: String,
    pub source_report: String,
    pub generated_at: String,
    pub target_stack_id: String,
    #[serde(default)]
    pub plan_mode: String,
    #[serde(default)]
    pub planned_edits: Vec<PlannedEdit>,
    #[serde(default)]
    pub planned_commands: Vec<String>,
    #[serde(default)]
    pub proof_lanes: Vec<String>,
    #[serde(default)]
    pub rollback_guidance: Vec<String>,
    #[serde(default)]
    pub human_approval_requirements: Vec<String>,
    pub packets: Vec<RepairPacket>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct PlannedEdit {
    pub path: String,
    pub operation: String,
    pub reason: String,
    pub finding_fingerprint: String,
    pub rule_id: String,
    pub apply_strategy: String,
    pub risk_level: String,
    pub repair_eligibility: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct RepairPacket {
    pub finding_fingerprint: String,
    pub finding_path: String,
    pub rule_id: String,
    pub check_id: String,
    pub severity: String,
    pub owner: String,
    pub lane: String,
    pub problem: String,
    pub why: String,
    pub permission_profile: String,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub expected_patch_shape: String,
    pub required_proof: Vec<String>,
    pub stop_conditions: Vec<String>,
    #[serde(default)]
    pub repair_eligibility: String,
    #[serde(default)]
    pub risk_level: String,
    #[serde(default)]
    pub eligibility_reason: String,
    pub human_review_required: bool,
    pub rollback_guidance: String,
}

pub fn run(args: RepairPlanArgs) -> Result<()> {
    let plan = build_repair_plan(&args.repo, &args.from)?;
    match args.out.as_deref() {
        Some(path) => {
            validation::write_json(&args.repo, ArtifactSchema::RepairPlan, path, &plan)?;
        }
        None => {
            validation::validate_serializable(&args.repo, ArtifactSchema::RepairPlan, &plan)?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&plan))?;
    }
    Ok(())
}

pub fn build_repair_plan(repo: &Path, report_path: &str) -> Result<RepairPlan> {
    let catalog = RepoCatalog::load(repo)?;
    let text = fs::read_to_string(report_path)
        .with_context(|| format!("read repair source {}", report_path))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("parse repair source {}", report_path))?;
    let findings = value
        .get("findings")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut packets = Vec::new();
    for finding in findings {
        packets.push(packet_from_finding(&catalog, &finding));
    }
    let planned_edits = packets.iter().map(planned_edit_from_packet).collect();
    let mut planned_commands = Vec::new();
    let mut proof_lanes = Vec::new();
    let mut rollback_guidance = Vec::new();
    let mut human_approval_requirements = Vec::new();
    for packet in &packets {
        for command in &packet.required_proof {
            push_unique(&mut planned_commands, command.clone());
        }
        push_unique(&mut proof_lanes, packet.lane.clone());
        push_unique(&mut rollback_guidance, packet.rollback_guidance.clone());
        if packet.human_review_required {
            push_unique(
                &mut human_approval_requirements,
                format!(
                    "{} {} requires approval: {}",
                    packet.rule_id, packet.finding_fingerprint, packet.eligibility_reason
                ),
            );
        }
    }
    Ok(RepairPlan {
        schema_version: "1.0.0".to_string(),
        source_report: report_path.to_string(),
        generated_at: now_string(),
        target_stack_id: crate::model::TARGET_STACK_ID.to_string(),
        plan_mode: "dry-run".to_string(),
        planned_edits,
        planned_commands,
        proof_lanes,
        rollback_guidance,
        human_approval_requirements,
        packets,
    })
}

fn packet_from_finding(catalog: &RepoCatalog, finding: &serde_json::Value) -> RepairPacket {
    let finding_path = str_field(finding, "path");
    let rule_id = str_field(finding, "rule_id");
    let severity = str_field(finding, "severity");
    let owner = str_field(finding, "owner");
    let lane = str_field(finding, "lane");
    let problem = str_field(finding, "problem");
    let why = finding
        .get("reason")
        .and_then(|value| value.as_str())
        .unwrap_or(&problem)
        .to_string();
    let agent_fix = finding
        .get("agent_fix")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let permission_profile = infer_permission_profile(&severity, &rule_id, &finding_path, &owner);
    let (repair_eligibility, risk_level, mut eligibility_reason) =
        repair_policy_for_finding(&rule_id, &severity);
    let owner_route = catalog.owner_for_path(&finding_path).unwrap_or(&owner);
    let mut allowed_paths = repair_allowed_paths(catalog, &finding_path, owner_route);
    let mut blocked_required_paths = Vec::new();
    for candidate in inferred_required_paths(&problem, &why, &agent_fix) {
        if let Some(scope) = safe_required_path_scope(
            catalog,
            &finding_path,
            owner_route,
            &rule_id,
            &permission_profile,
            &candidate,
        ) {
            push_unique(&mut allowed_paths, scope);
        } else {
            push_unique(&mut blocked_required_paths, candidate);
        }
    }
    if allowed_paths.is_empty() {
        push_unique(&mut allowed_paths, finding_path.clone());
    }
    if let Some(first_blocked) = blocked_required_paths.first() {
        let blocked_reason = format!("required fix path outside allowed_paths: {first_blocked}");
        if eligibility_reason.is_empty() {
            eligibility_reason = blocked_reason;
        } else {
            eligibility_reason = format!("{eligibility_reason}; {blocked_reason}");
        }
    }
    let forbidden_paths = repair_forbidden_paths(catalog);
    let expected_patch_shape = expected_patch_shape(&rule_id, &severity, &finding_path);
    let mut required_proof = string_or_array_field(finding, "rerun_command");
    if required_proof.is_empty() {
        required_proof.extend(catalog.commands_for_paths(&allowed_paths));
    }
    if required_proof.is_empty() {
        required_proof.push("just fast".to_string());
    }
    let mut stop_conditions = vec![
        "stop if the fix broadens permission scope or touches a generated zone".to_string(),
        "stop if the repair requires a migration, secret rotation, or external service change"
            .to_string(),
    ];
    if let Some(first_blocked) = blocked_required_paths.first() {
        push_unique(
            &mut stop_conditions,
            format!("stop if required fix path outside allowed_paths: {first_blocked}"),
        );
    }
    if permission_profile == "security-investigation" {
        push_unique(
            &mut stop_conditions,
            "stop and hand off any secret, credential, or token exposure",
        );
    }
    if permission_profile == "generated-regeneration" {
        push_unique(
            &mut stop_conditions,
            "stop if the source contract or generator is not identified first",
        );
    }
    let human_review_required = human_review_required(
        &severity,
        &rule_id,
        &finding_path,
        &permission_profile,
        &repair_eligibility,
        &risk_level,
    );
    let rollback_guidance = rollback_guidance(&permission_profile, &finding_path, &rule_id);
    RepairPacket {
        finding_fingerprint: str_field(finding, "fingerprint"),
        finding_path,
        rule_id,
        check_id: str_field(finding, "check_id"),
        severity,
        owner,
        lane,
        problem,
        why,
        permission_profile,
        allowed_paths,
        forbidden_paths,
        expected_patch_shape,
        required_proof,
        stop_conditions,
        repair_eligibility,
        risk_level,
        eligibility_reason,
        human_review_required,
        rollback_guidance,
    }
}

fn planned_edit_from_packet(packet: &RepairPacket) -> PlannedEdit {
    PlannedEdit {
        path: packet.finding_path.clone(),
        operation: planned_operation(packet).to_string(),
        reason: packet.expected_patch_shape.clone(),
        finding_fingerprint: packet.finding_fingerprint.clone(),
        rule_id: packet.rule_id.clone(),
        apply_strategy: "none".to_string(),
        risk_level: packet.risk_level.clone(),
        repair_eligibility: packet.repair_eligibility.clone(),
        match_text: None,
        replacement_text: None,
        append_text: None,
        create_text: None,
    }
}

fn planned_operation(packet: &RepairPacket) -> &'static str {
    if packet.finding_path.is_empty() {
        "none"
    } else if packet.rule_id == "HLT-002-GENERATED-MUTATION"
        || packet.permission_profile == "generated-regeneration"
    {
        "regenerate"
    } else if packet.repair_eligibility == RepairEligibility::NeverAuto.as_str() {
        "review-only"
    } else {
        "modify"
    }
}

fn render_markdown(plan: &RepairPlan) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Repair Plan");
    let _ = writeln!(out);
    let _ = writeln!(out, "- source report: `{}`", plan.source_report);
    let _ = writeln!(out, "- generated at: `{}`", plan.generated_at);
    let _ = writeln!(out, "- target stack: `{}`", plan.target_stack_id);
    let _ = writeln!(out, "- mode: `{}`", plan.plan_mode);
    let _ = writeln!(
        out,
        "- planned commands: `{}`",
        join_or_none(&plan.planned_commands)
    );
    let _ = writeln!(out, "- proof lanes: `{}`", join_or_none(&plan.proof_lanes));
    for packet in &plan.packets {
        let _ = writeln!(out);
        let _ = writeln!(out, "## {} {}", packet.rule_id, packet.finding_path);
        let _ = writeln!(out, "- fingerprint: `{}`", packet.finding_fingerprint);
        let _ = writeln!(out, "- severity: `{}`", packet.severity);
        let _ = writeln!(out, "- owner: `{}`", packet.owner);
        let _ = writeln!(out, "- lane: `{}`", packet.lane);
        let _ = writeln!(out, "- profile: `{}`", packet.permission_profile);
        let _ = writeln!(out, "- eligibility: `{}`", packet.repair_eligibility);
        let _ = writeln!(out, "- risk: `{}`", packet.risk_level);
        let _ = writeln!(out, "- eligibility reason: {}", packet.eligibility_reason);
        let _ = writeln!(out, "- problem: {}", packet.problem);
        let _ = writeln!(out, "- why: {}", packet.why);
        let _ = writeln!(out, "- allowed: `{}`", join_or_none(&packet.allowed_paths));
        let _ = writeln!(
            out,
            "- forbidden: `{}`",
            join_or_none(&packet.forbidden_paths)
        );
        let _ = writeln!(out, "- patch shape: {}", packet.expected_patch_shape);
        let _ = writeln!(out, "- proof: `{}`", join_or_none(&packet.required_proof));
        let _ = writeln!(out, "- stop: `{}`", join_or_none(&packet.stop_conditions));
        let _ = writeln!(out, "- human review: `{}`", packet.human_review_required);
        let _ = writeln!(out, "- rollback: {}", packet.rollback_guidance);
    }
    out
}

fn infer_permission_profile(severity: &str, rule_id: &str, path: &str, owner: &str) -> String {
    if matches!(
        rule_id,
        "HLT-010-SECRET-SPRAWL" | "HLT-011-PROMPT-INJECTION" | "HLT-012-OVERBROAD-AGENCY"
    ) || severity == "critical"
    {
        "security-investigation".to_string()
    } else if rule_id == "HLT-002-GENERATED-MUTATION" || path.starts_with("agent/") {
        "generated-regeneration".to_string()
    } else if path.starts_with("docs/") || path.starts_with("tips/") || owner == "paper" {
        "docs-only".to_string()
    } else if path.starts_with(".github/") || path == "Justfile" {
        "release".to_string()
    } else {
        "code-edit".to_string()
    }
}

fn repair_allowed_paths(catalog: &RepoCatalog, path: &str, owner: &str) -> Vec<String> {
    let mut out = Vec::new();
    for prefix in catalog.prefixes_for_owner(owner) {
        push_unique(&mut out, prefix);
    }
    if let Some(prefix) = parent_prefix(path) {
        push_unique(&mut out, prefix);
    }
    push_unique(&mut out, path);
    out
}

fn inferred_required_paths(problem: &str, why: &str, agent_fix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for text in [problem, why, agent_fix] {
        for candidate in path_candidates(text) {
            if seen.insert(candidate.clone()) {
                out.push(candidate);
            }
        }
    }
    out
}

fn path_candidates(text: &str) -> Vec<String> {
    static PATH_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?x)
            (?P<path>
                (?:\.{1,2}/)?(?:[A-Za-z0-9._-]+/)+[A-Za-z0-9._@*?-]+(?:/[A-Za-z0-9._@*?-]+)*
                | rust-toolchain\.toml
                | \.tool-versions
                | Justfile
                | CODEOWNERS
                | AGENTS\.md
                | CLAUDE\.md
                | GEMINI\.md
            )
        ",
        )
        .expect("repair plan path regex is valid")
    });
    PATH_RE
        .captures_iter(text)
        .filter_map(|capture| capture.name("path").map(|path| path.as_str().to_string()))
        .filter_map(|path| normalize_required_path(&path))
        .collect()
}

fn normalize_required_path(path: &str) -> Option<String> {
    let cleaned = path
        .trim()
        .trim_matches(|c: char| {
            matches!(
                c,
                '`' | '"' | '\'' | ',' | ';' | ':' | ')' | '(' | '[' | ']' | '{' | '}'
            )
        })
        .trim();
    if cleaned.is_empty() {
        return None;
    }
    if let Some(idx) = cleaned
        .char_indices()
        .find_map(|(idx, ch)| matches!(ch, '*' | '?' | '{' | '[').then_some(idx))
    {
        let prefix = cleaned[..idx].trim_end_matches('/');
        if prefix.is_empty() {
            return None;
        }
        if cleaned[..idx].ends_with('/') {
            return Some(format!("{prefix}/"));
        }
        return parent_prefix(prefix).or_else(|| Some(prefix.to_string()));
    }
    Some(cleaned.trim_end_matches('/').to_string())
}

fn safe_required_path_scope(
    catalog: &RepoCatalog,
    finding_path: &str,
    owner: &str,
    rule_id: &str,
    permission_profile: &str,
    candidate: &str,
) -> Option<String> {
    let scope = normalize_required_path(candidate)?;
    let finding_owner = catalog.owner_for_path(finding_path).unwrap_or(owner);
    if catalog.owner_for_path(&scope) == Some(finding_owner) {
        return Some(scope);
    }
    if repo_policy_allows_required_path(finding_path, rule_id, permission_profile, &scope) {
        return Some(scope);
    }
    None
}

fn repo_policy_allows_required_path(
    finding_path: &str,
    rule_id: &str,
    permission_profile: &str,
    candidate: &str,
) -> bool {
    if !ci_local_parity_scope(finding_path, rule_id, permission_profile) {
        return false;
    }
    matches!(
        candidate,
        "scripts/ci-local.sh"
            | "scripts/ci-doctor.sh"
            | "rust-toolchain.toml"
            | ".tool-versions"
            | "Justfile"
    ) || candidate.starts_with("scripts/")
        || candidate.starts_with("ops/ci/")
        || candidate.starts_with("ops/git-hooks/")
        || candidate.starts_with(".github/workflows/")
}

fn ci_local_parity_scope(finding_path: &str, rule_id: &str, _permission_profile: &str) -> bool {
    finding_path.starts_with(".github/workflows/")
        || finding_path.starts_with("ops/")
        || matches!(
            rule_id,
            "HLT-020-CI-HARDENING-GAP" | "HLT-042-CI-LOCAL-PARITY"
        )
}

fn repair_forbidden_paths(catalog: &RepoCatalog) -> Vec<String> {
    let mut out = vec!["reference/".to_string(), "target/".to_string()];
    for path in catalog.forbidden_generated_paths() {
        push_unique(&mut out, path);
    }
    out
}

fn expected_patch_shape(rule_id: &str, severity: &str, path: &str) -> String {
    match rule_id {
        "HLT-010-SECRET-SPRAWL" => {
            "remove secrets, rotate credentials, and add secret scanning".to_string()
        }
        "HLT-011-PROMPT-INJECTION" => {
            "separate trusted policy from untrusted input and strip bypass language".to_string()
        }
        "HLT-012-OVERBROAD-AGENCY" => {
            "narrow the tool profile and route through least-privilege approval".to_string()
        }
        "HLT-002-GENERATED-MUTATION" => {
            "edit the source contract and regenerate the protected artifact".to_string()
        }
        "HLT-006-DIRECT-DB-WRONG-LAYER" => {
            "move SQL into the adapter or migration layer and keep domain code pure".to_string()
        }
        "HLT-021-DESTRUCTIVE-MIGRATION" => {
            "add rollback/backfill/lock or staged-deploy notes (or an approved `jankurai:migration-safe` marker) and rerun db-migration-analyze".to_string()
        }
        "HLT-007-HANDWRITTEN-CONTRACT" => {
            "replace handwritten mirrors with source contract or generated client output"
                .to_string()
        }
        "HLT-013-RENDERED-UX-GAP" => {
            "add rendered UX proof, state coverage, and accessibility receipts".to_string()
        }
        "HLT-017-OPAQUE-OBSERVABILITY" => {
            "add typed errors, traces, and repairable boundary evidence".to_string()
        }
        "HLT-019-STREAMING-RUNTIME-DRIFT" => {
            "move broker clients behind an adapter and keep contracts generated".to_string()
        }
        _ if path.starts_with("docs/") || path.starts_with("tips/") => {
            "make the document precise, short, and aligned with the canonical standard".to_string()
        }
        _ if severity == "critical" => {
            "narrow to the smallest safe fix and require human review".to_string()
        }
        _ => "scoped fix with targeted proof and no authority expansion".to_string(),
    }
}

fn human_review_required(
    severity: &str,
    rule_id: &str,
    path: &str,
    permission_profile: &str,
    repair_eligibility: &str,
    risk_level: &str,
) -> bool {
    let _ = (
        severity,
        rule_id,
        path,
        permission_profile,
        repair_eligibility,
        risk_level,
    );
    false
}

fn repair_policy_for_finding(rule_id: &str, severity: &str) -> (String, String, String) {
    if let Some(rule) = rules::lookup(rule_id) {
        return (
            rule.repair_eligibility.as_str().to_string(),
            rule.repair_risk.as_str().to_string(),
            rule.repair_reason.to_string(),
        );
    }
    let risk = risk_from_severity(severity);
    (
        RepairEligibility::HumanRequired.as_str().to_string(),
        risk.as_str().to_string(),
        "unknown rule requires human review".to_string(),
    )
}

fn risk_from_severity(severity: &str) -> RepairRisk {
    match severity {
        "low" => RepairRisk::Low,
        "medium" => RepairRisk::Medium,
        "critical" => RepairRisk::Critical,
        _ => RepairRisk::High,
    }
}

fn rollback_guidance(permission_profile: &str, path: &str, rule_id: &str) -> String {
    if rule_id == "HLT-002-GENERATED-MUTATION" || permission_profile == "generated-regeneration" {
        "revert the source contract or template, then regenerate the output".to_string()
    } else if permission_profile == "security-investigation" {
        "revert the scoped policy change and confirm secret scan evidence again".to_string()
    } else if path.starts_with("docs/") || path.starts_with("tips/") {
        "restore the previous text and rerun the narrow proof lane".to_string()
    } else {
        "revert the scoped files and rerun the required proof before retrying".to_string()
    }
}

fn parent_prefix(path: &str) -> Option<String> {
    path.rsplit_once('/')
        .map(|(prefix, _)| format!("{prefix}/"))
}

fn str_field(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

fn string_or_array_field(value: &serde_json::Value, key: &str) -> Vec<String> {
    if let Some(text) = value.get(key).and_then(|value| value.as_str()) {
        return vec![text.to_string()];
    }
    value
        .get(key)
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|item| item.to_string()))
                .collect()
        })
        .unwrap_or_default()
}
