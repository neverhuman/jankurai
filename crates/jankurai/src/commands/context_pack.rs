use crate::commands::context_data::{push_unique, GeneratedZone, RepoCatalog};
use crate::validation::{self, ArtifactSchema};
use anyhow::Result;
use jankurai_proofbind::{build_proofbind, ProofBindMode, ProofBindRequest};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ContextPackArgs {
    pub repo: PathBuf,
    pub task: String,
    pub changed: Vec<PathBuf>,
    pub max_tokens: usize,
    pub agent: String,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextPack {
    pub schema_version: String,
    pub task: String,
    pub owner: String,
    pub permission_profile: String,
    pub changed_paths: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub generated_zones: Vec<String>,
    pub read_first_files: Vec<String>,
    pub relevant_docs: Vec<String>,
    pub proof_lanes: Vec<String>,
    pub commands: Vec<String>,
    pub likely_rules: Vec<String>,
    pub scope_decisions: Vec<ContextScopeDecision>,
    pub human_approval_required: bool,
    pub human_approval_reasons: Vec<String>,
    pub max_context_files: usize,
    pub stop_conditions: Vec<String>,
    pub residual_risk: Vec<String>,
    pub token_budget: usize,
    pub estimated_tokens: usize,
    pub agent: String,
    pub included_files: Vec<ContextFileEntry>,
    pub excluded_files: Vec<ContextExcludedFile>,
    pub source_trust_summary: BTreeMap<String, usize>,
    pub raw_output_policy: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextFileEntry {
    pub path: String,
    pub estimated_tokens: usize,
    pub reason: String,
    pub source_trust: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextExcludedFile {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextScopeDecision {
    pub path: String,
    pub owner: String,
    pub owner_route: String,
    pub test_command: String,
    pub proof_lane: String,
    pub generated_zone: bool,
    pub generated_source: String,
    pub generated_command: String,
    pub decision: String,
    pub reason: String,
}

pub fn run(args: ContextPackArgs) -> Result<()> {
    let pack = build_context_pack_with_options(
        &args.repo,
        &args.task,
        &args.changed,
        args.max_tokens,
        &args.agent,
    )?;
    match args.out.as_deref() {
        Some(path) => {
            validation::write_json(&args.repo, ArtifactSchema::ContextPack, path, &pack)?;
        }
        None => {
            validation::validate_serializable(&args.repo, ArtifactSchema::ContextPack, &pack)?;
            println!("{}", serde_json::to_string_pretty(&pack)?);
        }
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&pack))?;
    }
    Ok(())
}

pub fn build_context_pack(repo: &Path, task: &str, changed: &[PathBuf]) -> Result<ContextPack> {
    build_context_pack_with_options(repo, task, changed, 6000, "generic")
}

pub fn build_context_pack_with_options(
    repo: &Path,
    task: &str,
    changed: &[PathBuf],
    max_tokens: usize,
    agent: &str,
) -> Result<ContextPack> {
    let catalog = RepoCatalog::load(repo)?;
    let changed_paths = normalize_paths(changed);
    let task_lc = task.to_ascii_lowercase();
    let mut allowed_paths = Vec::new();

    if !changed_paths.is_empty() {
        for path in &changed_paths {
            push_unique(&mut allowed_paths, path.clone());
            for prefix in prefixes_for_path(&catalog, path) {
                push_unique(&mut allowed_paths, prefix);
            }
        }
    }

    if allowed_paths.is_empty() {
        for path in infer_roots(&task_lc) {
            push_unique(&mut allowed_paths, path);
        }
    }

    let owner = infer_owner(&catalog, &task_lc, &changed_paths, &allowed_paths);
    if owner == "mixed" {
        for prefix in catalog.prefixes_for_owner("agent") {
            push_unique(&mut allowed_paths, prefix);
        }
    } else if owner != "unmapped" {
        for prefix in catalog.prefixes_for_owner(&owner) {
            push_unique(&mut allowed_paths, prefix);
        }
    }

    let permission_profile = infer_permission_profile(&task_lc, &allowed_paths, &owner);
    let generated_zones = catalog.generated_paths();
    let forbidden_paths = build_forbidden_paths(&catalog);
    let read_first_files = build_read_first_files(&task_lc);
    let relevant_docs = build_relevant_docs(&task_lc);
    let mut proof_lanes = build_proof_lanes(&task_lc, &allowed_paths, &permission_profile);
    let mut commands = catalog.commands_for_paths(&allowed_paths);
    for lane in &proof_lanes {
        let lane_cmds = catalog.proof_lane_commands(&[lane.as_str()]);
        for cmd in lane_cmds {
            push_unique(&mut commands, cmd);
        }
    }
    if commands.is_empty() {
        for cmd in fallback_commands(&permission_profile) {
            push_unique(&mut commands, cmd);
        }
    }
    let mut likely_rules =
        build_likely_rules(&task_lc, &permission_profile, &owner, &allowed_paths);
    let mut proofbind_stop_conditions = Vec::new();
    let mut proofbind_residual_risk = Vec::new();
    apply_proofbind_context(
        repo,
        &changed_paths,
        &mut proof_lanes,
        &mut commands,
        &mut likely_rules,
        &mut proofbind_stop_conditions,
        &mut proofbind_residual_risk,
    );
    let scope_paths = if changed_paths.is_empty() {
        allowed_paths.clone()
    } else {
        changed_paths.clone()
    };
    let scope_decisions = build_scope_decisions(&catalog, &scope_paths);
    let human_approval_reasons = build_human_approval_reasons(&scope_decisions);
    let human_approval_required = !human_approval_reasons.is_empty();
    let mut stop_conditions = build_stop_conditions(
        &permission_profile,
        &allowed_paths,
        &generated_zones,
        &scope_decisions,
    );
    for item in proofbind_stop_conditions {
        push_unique(&mut stop_conditions, item);
    }
    let mut residual_risk = vec![
        "heuristic routing can miss a cross-owner edit".to_string(),
        "confirm the source contract before editing generated output".to_string(),
    ];
    for item in proofbind_residual_risk {
        push_unique(&mut residual_risk, item);
    }
    let token_budget = max_tokens.max(1);
    let (included_files, excluded_files, estimated_tokens, source_trust_summary) =
        build_context_files(
            repo,
            &catalog,
            &read_first_files,
            &allowed_paths,
            &scope_paths,
            token_budget,
        );
    Ok(ContextPack {
        schema_version: "1.2.0".to_string(),
        task: task.to_string(),
        owner,
        permission_profile,
        changed_paths,
        allowed_paths,
        forbidden_paths,
        generated_zones,
        read_first_files,
        relevant_docs,
        proof_lanes,
        commands,
        likely_rules,
        scope_decisions,
        human_approval_required,
        human_approval_reasons,
        max_context_files: 12,
        stop_conditions,
        residual_risk,
        token_budget,
        estimated_tokens,
        agent: agent.to_string(),
        included_files,
        excluded_files,
        source_trust_summary,
        raw_output_policy:
            "include trusted policy and repo code first; summarize untrusted input and generated artifacts when budget is tight"
                .to_string(),
    })
}

fn apply_proofbind_context(
    repo: &Path,
    changed_paths: &[String],
    proof_lanes: &mut Vec<String>,
    commands: &mut Vec<String>,
    likely_rules: &mut Vec<String>,
    stop_conditions: &mut Vec<String>,
    residual_risk: &mut Vec<String>,
) {
    if changed_paths.is_empty() {
        return;
    }
    let output = build_proofbind(ProofBindRequest {
        repo_root: repo.to_path_buf(),
        changed_paths: changed_paths.iter().map(PathBuf::from).collect(),
        changed_from: None,
        mode: ProofBindMode::Advisory,
        proof_receipts: None,
    });
    let Ok(output) = output else {
        return;
    };
    if output.witness.surfaces.is_empty() {
        return;
    }
    push_unique(proof_lanes, "proofbind");
    let mut command = "cargo run -p jankurai -- proofbind verify .".to_string();
    for path in changed_paths.iter().take(12) {
        command.push_str(" --changed ");
        command.push_str(path);
    }
    push_unique(commands, command);
    for surface in &output.witness.surfaces {
        for lane in &surface.required_lanes {
            push_unique(proof_lanes, lane.clone());
        }
        for rule in &surface.required_rules {
            push_unique(likely_rules, rule.clone());
        }
        push_unique(
            residual_risk,
            format!(
                "proofbind surface `{}` type=`{}` severity=`{}` owner=`{}`",
                surface.path, surface.surface_type, surface.severity, surface.owner
            ),
        );
    }
    for obligation in output
        .obligations
        .obligations
        .iter()
        .filter(|item| !item.satisfied)
    {
        push_unique(
            stop_conditions,
            format!(
                "stop if proofbind obligation `{}` remains missing for `{}`",
                obligation.surface_type, obligation.path
            ),
        );
    }
}

fn render_markdown(pack: &ContextPack) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Context Pack");
    let _ = writeln!(out);
    let _ = writeln!(out, "- task: `{}`", pack.task);
    let _ = writeln!(out, "- owner: `{}`", pack.owner);
    let _ = writeln!(out, "- permission profile: `{}`", pack.permission_profile);
    let _ = writeln!(out, "- changed: `{}`", join_or_none(&pack.changed_paths));
    let _ = writeln!(out, "- allowed: `{}`", join_or_none(&pack.allowed_paths));
    let _ = writeln!(
        out,
        "- forbidden: `{}`",
        join_or_none(&pack.forbidden_paths)
    );
    let _ = writeln!(
        out,
        "- generated: `{}`",
        join_or_none(&pack.generated_zones)
    );
    let _ = writeln!(
        out,
        "- read first: `{}`",
        join_or_none(&pack.read_first_files)
    );
    let _ = writeln!(out, "- docs: `{}`", join_or_none(&pack.relevant_docs));
    let _ = writeln!(out, "- proof lanes: `{}`", join_or_none(&pack.proof_lanes));
    let _ = writeln!(out, "- commands: `{}`", join_or_none(&pack.commands));
    let _ = writeln!(
        out,
        "- likely rules: `{}`",
        join_or_none(&pack.likely_rules)
    );
    let _ = writeln!(
        out,
        "- human approval required: `{}`",
        pack.human_approval_required
    );
    let _ = writeln!(
        out,
        "- human approval reasons: `{}`",
        join_or_none(&pack.human_approval_reasons)
    );
    let _ = writeln!(out, "- max context files: `{}`", pack.max_context_files);
    let _ = writeln!(out, "- stop: `{}`", join_or_none(&pack.stop_conditions));
    let _ = writeln!(
        out,
        "- residual risk: `{}`",
        join_or_none(&pack.residual_risk)
    );
    let _ = writeln!(out, "- token budget: `{}`", pack.token_budget);
    let _ = writeln!(out, "- estimated tokens: `{}`", pack.estimated_tokens);
    let _ = writeln!(out, "- agent: `{}`", pack.agent);
    let _ = writeln!(out, "- raw output policy: `{}`", pack.raw_output_policy);
    let _ = writeln!(out);
    let _ = writeln!(out, "## Included files");
    if pack.included_files.is_empty() {
        let _ = writeln!(out, "- none");
    } else {
        for file in &pack.included_files {
            let _ = writeln!(
                out,
                "- `{}` tokens=`{}` trust=`{}` reason=`{}`",
                file.path, file.estimated_tokens, file.source_trust, file.reason
            );
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Scope decisions");
    if pack.scope_decisions.is_empty() {
        let _ = writeln!(out, "- none");
    } else {
        for decision in &pack.scope_decisions {
            let _ = writeln!(
                out,
                "- `{}`: `{}` owner=`{}` owner_route=`{}` proof_lane=`{}` generated=`{}` source=`{}` command=`{}` reason=`{}`",
                decision.path,
                decision.decision,
                decision.owner,
                decision.owner_route,
                decision.proof_lane,
                decision.generated_zone,
                decision.generated_source,
                decision.generated_command,
                decision.reason
            );
        }
    }
    out
}

fn build_context_files(
    repo: &Path,
    catalog: &RepoCatalog,
    read_first_files: &[String],
    allowed_paths: &[String],
    scope_paths: &[String],
    token_budget: usize,
) -> (
    Vec<ContextFileEntry>,
    Vec<ContextExcludedFile>,
    usize,
    BTreeMap<String, usize>,
) {
    let mut candidates = Vec::new();
    for path in read_first_files {
        push_unique(&mut candidates, path.clone());
    }
    for path in scope_paths {
        push_unique(&mut candidates, path.clone());
    }
    for path in allowed_paths.iter().take(12) {
        push_unique(&mut candidates, path.clone());
    }

    let mut included = Vec::new();
    let mut excluded = Vec::new();
    let mut estimated_total = 0usize;
    let mut trust_summary = BTreeMap::new();
    for path in candidates {
        let estimate = estimate_tokens_for_path(repo, &path);
        let trust = source_trust_for_path(catalog, &path);
        let reason = reason_for_context_file(catalog, &path);
        if estimated_total.saturating_add(estimate) <= token_budget {
            estimated_total += estimate;
            *trust_summary.entry(trust.clone()).or_insert(0) += 1;
            included.push(ContextFileEntry {
                path,
                estimated_tokens: estimate,
                reason,
                source_trust: trust,
            });
        } else {
            excluded.push(ContextExcludedFile {
                path,
                reason: format!("excluded by --max-tokens budget {token_budget}"),
            });
        }
    }
    (included, excluded, estimated_total, trust_summary)
}

fn estimate_tokens_for_path(repo: &Path, path: &str) -> usize {
    let full = repo.join(path);
    if full.is_file() {
        std::fs::read_to_string(full)
            .map(|text| estimate_tokens(&text))
            .unwrap_or(128)
    } else {
        128
    }
}

fn estimate_tokens(text: &str) -> usize {
    // Conservative deterministic estimate: roughly four bytes per token plus a line overhead.
    (text.len() / 4).saturating_add(text.lines().count()).max(1)
}

fn source_trust_for_path(catalog: &RepoCatalog, path: &str) -> String {
    if path.starts_with("reference/") {
        "untrusted-input".into()
    } else if path == "AGENTS.md"
        || path.starts_with("agent/")
        || path.starts_with(".agents/")
        || path.starts_with(".codex/")
    {
        "trusted-policy".into()
    } else if generated_zone_for_path(path, &catalog.generated_zones).is_some() {
        "generated-artifact".into()
    } else if path.starts_with("target/jankurai/") {
        "proof-evidence".into()
    } else if path.starts_with("docs/") || path.starts_with("paper/") || path == "README.md" {
        "docs".into()
    } else {
        "repo-code".into()
    }
}

fn reason_for_context_file(catalog: &RepoCatalog, path: &str) -> String {
    if path == "AGENTS.md" || path.starts_with("agent/") {
        "policy bootstrap".into()
    } else if generated_zone_for_path(path, &catalog.generated_zones).is_some() {
        "generated-zone routing".into()
    } else if catalog.test_route_for_path(path).is_some() {
        "owner/test route context".into()
    } else {
        "task-relevant context".into()
    }
}

fn infer_owner(
    catalog: &RepoCatalog,
    task_lc: &str,
    changed_paths: &[String],
    allowed_paths: &[String],
) -> String {
    let mut owners = Vec::new();
    for path in changed_paths.iter().chain(allowed_paths.iter()) {
        if let Some(owner) = catalog.owner_for_path(path) {
            push_unique(&mut owners, owner.to_string());
        }
    }
    if owners.len() == 1 {
        return owners[0].clone();
    }
    if owners.len() > 1 {
        return "mixed".to_string();
    }
    if task_lc.contains("agent")
        || task_lc.contains("context")
        || task_lc.contains("repair")
        || task_lc.contains("adapter")
    {
        "agent".to_string()
    } else if task_lc.contains("ux")
        || task_lc.contains("web")
        || task_lc.contains("storybook")
        || task_lc.contains("playwright")
    {
        "tools".to_string()
    } else if task_lc.contains("security") || task_lc.contains("secret") {
        "ops".to_string()
    } else if task_lc.contains("paper") || task_lc.contains("moonshot") {
        "paper".to_string()
    } else if task_lc.contains("docs") {
        "standard".to_string()
    } else if task_lc.is_empty() {
        "unmapped".to_string()
    } else {
        "mixed".to_string()
    }
}

fn infer_permission_profile(task_lc: &str, allowed_paths: &[String], owner: &str) -> String {
    if task_lc.contains("security") || owner == "ops" {
        "security-investigation".to_string()
    } else if task_lc.contains("paper")
        || task_lc.contains("docs")
        || allowed_paths.iter().any(|p| p.starts_with("paper/"))
        || allowed_paths
            .iter()
            .all(|path| path.starts_with("docs/") || path == "AGENTS.md")
    {
        "docs-only".to_string()
    } else if task_lc.contains("generated") || allowed_paths.iter().any(|p| p.contains(".toml")) {
        "generated-regeneration".to_string()
    } else if task_lc.contains("release") {
        "release".to_string()
    } else if task_lc.is_empty() {
        "read-only".to_string()
    } else {
        "code-edit".to_string()
    }
}

fn build_forbidden_paths(catalog: &RepoCatalog) -> Vec<String> {
    let mut out = vec!["reference/".to_string(), "target/".to_string()];
    for path in catalog.forbidden_generated_paths() {
        push_unique(&mut out, path);
    }
    out
}

fn build_read_first_files(task_lc: &str) -> Vec<String> {
    let mut out = vec![
        "AGENTS.md".to_string(),
        "agent/JANKURAI_STANDARD.md".to_string(),
        "docs/agent-native-standard.md".to_string(),
        "docs/moonshot.md".to_string(),
    ];
    if task_lc.contains("agent") || task_lc.contains("repair") || task_lc.contains("context") {
        push_unique(&mut out, "docs/boundary-oracle.md");
        push_unique(&mut out, "docs/install.md");
        push_unique(&mut out, "docs/ide-integrations.md");
        push_unique(&mut out, "tips/phases/08-agent-context-repair.md");
    }
    if task_lc.contains("reference") || task_lc.contains("platform") || task_lc.contains("golden") {
        push_unique(&mut out, "tips/phases/09-reference-product-platform.md");
        push_unique(&mut out, "docs/streaming.md");
    }
    if task_lc.contains("security") {
        push_unique(&mut out, "docs/boundary-oracle.md");
    }
    out
}

fn build_relevant_docs(task_lc: &str) -> Vec<String> {
    let mut out = vec![
        "docs/agent-native-standard.md".to_string(),
        "docs/moonshot.md".to_string(),
    ];
    if task_lc.contains("agent") || task_lc.contains("repair") || task_lc.contains("context") {
        push_unique(&mut out, "docs/boundary-oracle.md");
        push_unique(&mut out, "docs/install.md");
        push_unique(&mut out, "docs/ide-integrations.md");
    }
    if task_lc.contains("reference") || task_lc.contains("platform") || task_lc.contains("golden") {
        push_unique(&mut out, "docs/streaming.md");
        push_unique(&mut out, "tips/phases/09-reference-product-platform.md");
    }
    if task_lc.contains("security") {
        push_unique(&mut out, "docs/boundary-oracle.md");
    }
    if task_lc.contains("docs") {
        push_unique(&mut out, "docs/ide-integrations.md");
    }
    out
}

fn build_proof_lanes(
    task_lc: &str,
    allowed_paths: &[String],
    permission_profile: &str,
) -> Vec<String> {
    let mut lanes = vec!["fast".to_string(), "audit".to_string()];
    if permission_profile == "security-investigation" || task_lc.contains("security") {
        push_unique(&mut lanes, "security");
    }
    if task_lc.contains("paper") || allowed_paths.iter().any(|p| p.starts_with("paper/")) {
        push_unique(&mut lanes, "paper");
    }
    if task_lc.contains("reference")
        || task_lc.contains("platform")
        || task_lc.contains("golden")
        || task_lc.contains("ux")
        || task_lc.contains("web")
        || task_lc.contains("db")
        || task_lc.contains("contract")
    {
        push_unique(&mut lanes, "full");
    }
    if task_lc.contains("release") {
        push_unique(&mut lanes, "full");
    }
    lanes
}

fn build_likely_rules(
    task_lc: &str,
    permission_profile: &str,
    owner: &str,
    allowed_paths: &[String],
) -> Vec<String> {
    let mut out = Vec::new();
    if permission_profile == "security-investigation" || task_lc.contains("security") {
        out.extend([
            "HLT-010-SECRET-SPRAWL".to_string(),
            "HLT-011-PROMPT-INJECTION".to_string(),
            "HLT-012-OVERBROAD-AGENCY".to_string(),
        ]);
    }
    if task_lc.contains("context") || task_lc.contains("repair") || owner == "agent" {
        out.extend([
            "HLT-011-PROMPT-INJECTION".to_string(),
            "HLT-012-OVERBROAD-AGENCY".to_string(),
            "HLT-015-CONTEXT-SETUP-GAP".to_string(),
            "HLT-017-OPAQUE-OBSERVABILITY".to_string(),
        ]);
    }
    if task_lc.contains("reference")
        || task_lc.contains("platform")
        || task_lc.contains("golden")
        || allowed_paths
            .iter()
            .any(|p| p.starts_with("packages/ux-qa/"))
    {
        out.extend([
            "HLT-004-UNMAPPED-PROOF".to_string(),
            "HLT-007-HANDWRITTEN-CONTRACT".to_string(),
            "HLT-013-RENDERED-UX-GAP".to_string(),
        ]);
    }
    if task_lc.contains("db") || task_lc.contains("schema") || task_lc.contains("contract") {
        out.extend([
            "HLT-006-DIRECT-DB-WRONG-LAYER".to_string(),
            "HLT-007-HANDWRITTEN-CONTRACT".to_string(),
            "HLT-021-DESTRUCTIVE-MIGRATION".to_string(),
        ]);
    }
    if task_lc.contains("generated") {
        out.push("HLT-002-GENERATED-MUTATION".to_string());
    }
    if task_lc.contains("streaming") {
        out.push("HLT-019-STREAMING-RUNTIME-DRIFT".to_string());
    }
    if out.is_empty() {
        out.extend([
            "HLT-003-OWNERLESS-PATH".to_string(),
            "HLT-004-UNMAPPED-PROOF".to_string(),
        ]);
    }
    out.sort();
    out.dedup();
    out
}

fn build_stop_conditions(
    permission_profile: &str,
    allowed_paths: &[String],
    generated_zones: &[String],
    scope_decisions: &[ContextScopeDecision],
) -> Vec<String> {
    let mut out = vec![
        "stop if the requested edit would touch `reference/`".to_string(),
        "stop if the requested edit would broaden the permission profile".to_string(),
        "stop if the change requires a new generated artifact without a source contract"
            .to_string(),
    ];
    if permission_profile == "security-investigation" {
        push_unique(
            &mut out,
            "stop and escalate for secrets, credentials, or token material",
        );
    }
    if allowed_paths.iter().any(|path| path.starts_with("docs/")) {
        push_unique(
            &mut out,
            "stop if the task would introduce product-runtime truth into docs",
        );
    }
    if !generated_zones.is_empty() {
        push_unique(
            &mut out,
            "stop if any edit lands in a generated zone instead of the declared source",
        );
    }
    if scope_decisions
        .iter()
        .any(|decision| decision.owner == "unmapped")
    {
        push_unique(
            &mut out,
            "stop if any changed path lacks owner-map coverage",
        );
    }
    if scope_decisions
        .iter()
        .any(|decision| decision.test_command == "unmapped")
    {
        push_unique(
            &mut out,
            "stop if any changed path lacks test-map/proof-lane coverage",
        );
    }
    if scope_decisions
        .iter()
        .any(|decision| decision.generated_zone)
    {
        push_unique(
            &mut out,
            "stop if the requested edit targets generated output instead of its declared source",
        );
    }
    out
}

fn build_scope_decisions(
    catalog: &RepoCatalog,
    candidate_paths: &[String],
) -> Vec<ContextScopeDecision> {
    let mut unique_paths = Vec::new();
    for path in candidate_paths {
        push_unique(&mut unique_paths, path.clone());
    }

    unique_paths
        .into_iter()
        .map(|path| {
            let owner = catalog
                .owner_for_path(&path)
                .unwrap_or("unmapped")
                .to_string();
            let owner_route = catalog
                .owner_prefix_for_path(&path)
                .unwrap_or_else(|| "unmapped".to_string());
            let test_command = catalog
                .test_route_for_path(&path)
                .map(|(_, spec)| spec.command)
                .unwrap_or_else(|| "unmapped".to_string());
            let proof_lane = if test_command == "unmapped" {
                "unmapped".to_string()
            } else {
                catalog
                    .proof_lane_for_command(&test_command)
                    .unwrap_or_else(|| "test-map".to_string())
            };
            let generated_match = generated_zone_for_path(&path, &catalog.generated_zones);
            let generated_zone = generated_match.is_some();
            let generated_source = generated_match
                .map(|zone| non_empty_or_none(&zone.source))
                .unwrap_or_else(|| "none".to_string());
            let generated_command = generated_match
                .map(|zone| non_empty_or_none(&zone.command))
                .unwrap_or_else(|| "none".to_string());
            let (decision, reason) = if generated_zone {
                (
                    "read-only",
                    "path is declared generated output; edit the source contract and rerun its command",
                )
            } else if owner == "unmapped" {
                (
                    "human-review",
                    "path lacks owner-map coverage; add ownership before editing",
                )
            } else if test_command == "unmapped" {
                (
                    "human-review",
                    "path lacks test-map proof routing; add proof before editing",
                )
            } else {
                (
                    "allowed",
                    "owner and proof route are mapped for the requested scope",
                )
            };

            ContextScopeDecision {
                path,
                owner,
                owner_route,
                test_command,
                proof_lane,
                generated_zone,
                generated_source,
                generated_command,
                decision: decision.to_string(),
                reason: reason.to_string(),
            }
        })
        .collect()
}

fn build_human_approval_reasons(decisions: &[ContextScopeDecision]) -> Vec<String> {
    let mut out = Vec::new();
    for decision in decisions {
        if decision.decision != "allowed" {
            push_unique(&mut out, format!("{}: {}", decision.path, decision.reason));
        }
    }
    out
}

fn generated_zone_for_path<'a>(
    path: &str,
    generated_zones: &'a [GeneratedZone],
) -> Option<&'a GeneratedZone> {
    let path = path.trim().trim_matches('/');
    if path.is_empty() {
        return None;
    }
    generated_zones.iter().find(|zone| {
        let zone_path = zone.path.trim().trim_matches('/');
        if zone_path.is_empty() {
            return false;
        }
        path == zone_path
            || path.starts_with(&format!("{zone_path}/"))
            || zone_path.starts_with(&format!("{path}/"))
    })
}

fn non_empty_or_none(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "none".to_string()
    } else {
        value.to_string()
    }
}

fn fallback_commands(permission_profile: &str) -> Vec<String> {
    let mut out = vec!["just fast".to_string(), "just score".to_string()];
    if permission_profile == "security-investigation" {
        push_unique(&mut out, "just security");
    }
    out
}

fn prefixes_for_path(catalog: &RepoCatalog, path: &str) -> Vec<String> {
    let mut out = Vec::new();
    for prefix in catalog.owners.keys() {
        if path_matches(path, prefix) {
            push_unique(&mut out, prefix.clone());
        }
    }
    out
}

fn infer_roots(task_lc: &str) -> Vec<String> {
    let mut out = Vec::new();
    if task_lc.contains("agent") || task_lc.contains("context") || task_lc.contains("repair") {
        out.extend([
            "agent/".to_string(),
            "crates/jankurai/src/commands/".to_string(),
            "docs/".to_string(),
            "tips/phases/08-agent-context-repair.md".to_string(),
        ]);
    }
    if task_lc.contains("reference") || task_lc.contains("platform") || task_lc.contains("golden") {
        out.extend([
            "docs/moonshot.md".to_string(),
            "tips/phases/09-reference-product-platform.md".to_string(),
        ]);
    }
    if task_lc.contains("ux") || task_lc.contains("web") {
        out.push("packages/ux-qa/".to_string());
    }
    if task_lc.contains("db") || task_lc.contains("schema") || task_lc.contains("contract") {
        out.extend(["db/".to_string(), "schemas/".to_string()]);
    }
    if task_lc.contains("security") {
        out.extend([
            ".github/".to_string(),
            "docs/boundary-oracle.md".to_string(),
        ]);
    }
    if task_lc.contains("paper") {
        out.push("tips/".to_string());
    }
    if out.is_empty() && !task_lc.is_empty() {
        out.extend(["agent/".to_string(), "docs/".to_string()]);
    }
    out
}

fn normalize_paths(paths: &[PathBuf]) -> Vec<String> {
    let mut out = Vec::new();
    for path in paths {
        let text = path.to_string_lossy().replace('\\', "/");
        push_unique(&mut out, text);
    }
    out
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn path_matches(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(prefix) || prefix.starts_with(path)
}
