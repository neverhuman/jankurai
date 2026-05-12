use crate::audit::run_audit;
use crate::commands::migrate;
use crate::commands::repair::now_string;
use crate::init;
use crate::validation::{self, ArtifactSchema};
use anyhow::{bail, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_OUT: &str = "target/jankurai/adoption-plan.json";
pub const DEFAULT_MD: &str = "target/jankurai/adoption-plan.md";

#[derive(Debug, Clone)]
pub struct AdoptArgs {
    pub repo: PathBuf,
    pub profile: String,
    pub mode: String,
    pub out: String,
    pub md: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolRolloutItem {
    pub id: String,
    pub category: String,
    pub status: String,
    pub score_priority: usize,
    pub next_command: String,
    pub artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdoptionPlan {
    pub schema_version: String,
    pub command: String,
    pub status: String,
    pub generated_at: String,
    pub source_root: String,
    pub mode: String,
    pub requested_profile: String,
    pub recommended_profile: String,
    pub risk_tier: String,
    pub detected_surfaces: Vec<String>,
    pub source_stack: String,
    pub target_stack: String,
    pub liability_score: u32,
    pub audit_score: Option<i32>,
    pub safe_commands: Vec<String>,
    pub tool_rollout: Vec<ToolRolloutItem>,
    pub stop_conditions: Vec<String>,
    pub next_milestones: Vec<String>,
    pub artifacts: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn run(args: AdoptArgs) -> Result<()> {
    let mut plan = build_adoption_plan(&args.repo, &args.profile, &args.mode)?;
    plan.artifacts = vec![args.out.clone(), args.md.clone()];
    validation::write_json(&args.repo, ArtifactSchema::AdoptionPlan, &args.out, &plan)?;
    crate::render::write_markdown(&args.md, &render_markdown(&plan))?;
    eprintln!("wrote {} and {}", args.out, args.md);
    Ok(())
}

pub fn build_adoption_plan(
    repo: &Path,
    requested_profile: &str,
    mode: &str,
) -> Result<AdoptionPlan> {
    if !matches!(mode, "observe" | "advisory" | "ratchet") {
        bail!("unknown adoption mode `{mode}`; expected observe, advisory, or ratchet");
    }

    let detected_surfaces = init::detect::detect_surfaces(repo);
    let migration_report = migrate::build_migration_report(repo, "rust-ts-postgres")?;
    let recommended_profile =
        recommend_profile(requested_profile, &detected_surfaces, &migration_report);
    let audit_report = run_audit(repo, &[]).ok();
    let audit_score = audit_report
        .as_ref()
        .map(|report| report.score)
        .or_else(|| read_existing_audit_score(repo));
    let risk_tier = risk_tier(migration_report.liability_score);
    let mut warnings = Vec::new();
    if requested_profile != "auto" && requested_profile != recommended_profile {
        warnings.push(format!(
            "requested profile `{requested_profile}` overrides recommended `{recommended_profile}`"
        ));
    }
    if audit_score.is_none() {
        warnings.push(
            "audit score unavailable; run the no-write audit command before ratcheting".into(),
        );
    }
    if recommended_profile == "migration-target" {
        warnings.push(
            "repo appears far enough from the target stack to route through migration planning"
                .into(),
        );
    }
    let safe_commands = safe_commands(mode, &recommended_profile);
    let tool_rollout = tool_rollout(audit_report.as_ref().map(|report| &report.tool_adoption));

    Ok(AdoptionPlan {
        schema_version: "1.0.0".into(),
        command: "jankurai adopt".into(),
        status: "complete".into(),
        generated_at: now_string(),
        source_root: repo.display().to_string(),
        mode: mode.to_string(),
        requested_profile: requested_profile.to_string(),
        recommended_profile,
        risk_tier: risk_tier.to_string(),
        detected_surfaces,
        source_stack: migration_report.source_stack,
        target_stack: migration_report.target_stack,
        liability_score: migration_report.liability_score,
        audit_score,
        safe_commands,
        tool_rollout,
        stop_conditions: stop_conditions(mode),
        next_milestones: next_milestones(mode, risk_tier),
        artifacts: vec![DEFAULT_OUT.into(), DEFAULT_MD.into()],
        warnings,
    })
}

fn recommend_profile(
    requested_profile: &str,
    surfaces: &[String],
    report: &migrate::MigrationReport,
) -> String {
    if requested_profile != "auto" {
        return requested_profile.to_string();
    }
    let has_rust = report
        .inventory
        .languages
        .iter()
        .any(|item| item.name == "rust");
    let has_node = report
        .inventory
        .languages
        .iter()
        .any(|item| item.name == "typescript");
    let has_db =
        !report.inventory.db_clients.is_empty() || surfaces.iter().any(|s| s == "postgres");
    let has_web = surfaces.iter().any(|s| s == "vite-react")
        || report
            .inventory
            .frameworks
            .iter()
            .any(|item| matches!(item.name.as_str(), "react" | "vue" | "angular" | "svelte"));
    let has_api = !report.inventory.api_surfaces.is_empty();

    if report.source_stack != "unknown" && !has_rust {
        return "migration-target".into();
    }
    if has_rust && (has_node || has_web || has_db) {
        return "rust-ts-postgres".into();
    }
    if has_rust && has_api {
        return "rust-api".into();
    }
    if has_web {
        return "react-web".into();
    }
    if report.source_stack == "unknown" {
        return "rust-ts-postgres".into();
    }
    "migration-target".into()
}

fn safe_commands(mode: &str, recommended_profile: &str) -> Vec<String> {
    let mut commands = vec![
        "jankurai audit . --mode advisory --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md".into(),
        "jankurai adopt . --mode observe --out target/jankurai/adoption-plan.json --md target/jankurai/adoption-plan.md".into(),
        format!("jankurai init . --profile {recommended_profile} --dry-run --plan-json target/jankurai/init-plan.json"),
        "jankurai ci install . --github --mode observe --dry-run".into(),
        "jankurai score trend --history agent/score-history.jsonl --out target/jankurai/score-trend.json --md target/jankurai/score-trend.md".into(),
        "jankurai witness . --changed-from origin/main --baseline agent/repo-score.json --out target/jankurai/merge-witness.json --md target/jankurai/merge-witness.md".into(),
    ];
    if mode == "ratchet" {
        commands.push(
            "jankurai audit . --mode ratchet --baseline target/jankurai/baseline-score.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md".into(),
        );
    }
    commands
}

fn stop_conditions(mode: &str) -> Vec<String> {
    let mut conditions = vec![
        "a generated command requires this source workspace instead of the installed `jankurai` binary".into(),
        "dry-run output plans to overwrite existing AGENTS.md, Justfile, workflows, or docs without a merge action".into(),
        "the plan claims production compliance before audit evidence and proof lanes exist".into(),
    ];
    if mode != "ratchet" {
        conditions.push("CI enforces a score floor before a baseline has been accepted".into());
    }
    conditions
}

fn next_milestones(mode: &str, risk_tier: &str) -> Vec<String> {
    let mut milestones = vec![
        "review adoption plan and liability evidence".into(),
        "run init dry-run and inspect planned control-plane files".into(),
        "install observe-mode CI only after preserving existing workflows".into(),
    ];
    if risk_tier == "high" {
        milestones
            .push("route through migration-target profile and slice the legacy migration".into());
    }
    if mode == "ratchet" {
        milestones
            .push("commit an accepted baseline before enforcing score regression checks".into());
    } else {
        milestones.push("stay advisory until the first accepted baseline exists".into());
    }
    milestones
}

fn tool_rollout(readiness: Option<&crate::model::ToolAdoptionReadiness>) -> Vec<ToolRolloutItem> {
    let Some(readiness) = readiness else {
        return vec![];
    };

    let mut items = readiness
        .items
        .iter()
        .filter(|item| {
            item.applicable && item.status != "artifact_verified" && item.status != "ci_evidence"
        })
        .map(|item| ToolRolloutItem {
            id: item.id.clone(),
            category: item.category.clone(),
            status: item.status.clone(),
            score_priority: rollout_priority(&item.id),
            next_command: next_tool_command(&item.id),
            artifact_paths: item.artifact_paths.clone(),
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|item| item.score_priority);
    items
}

fn rollout_priority(id: &str) -> usize {
    match id {
        "audit-ci" => 0,
        "security" => 1,
        "ux-qa" => 2,
        "db-migration-analyze" => 3,
        "contract-drift" => 4,
        "copy-code" => 5,
        "rust-witness" => 5,
        "proof-routing" => 6,
        _ => 99,
    }
}

fn next_tool_command(id: &str) -> String {
    match id {
        "audit-ci" => "jankurai ci install . --github --mode observe".into(),
        "proof-routing" => {
            "cargo run -p jankurai -- audit . --mode ratchet --baseline agent/repo-score.json --json agent/repo-score.json --md agent/repo-score.md --repair-queue-jsonl target/jankurai/repair-queue.jsonl".into()
        }
        "security" => "cargo run -p jankurai -- security run . --out target/jankurai/security/evidence.json".into(),
        "ux-qa" => "jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json".into(),
        "db-migration-analyze" => {
            "cargo run -p jankurai -- migrate . --analyze --json target/jankurai/migration-report.json".into()
        }
        "contract-drift" => {
            "cargo run -p jankurai -- audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md".into()
        }
        "copy-code" => {
            "cargo run -p jankurai -- copy-code . --json target/jankurai/copy-code.json --md target/jankurai/copy-code.md".into()
        }
        "rust-witness" => "cargo run -p jankurai -- rust witness build .".into(),
        _ => "jankurai adopt".into(),
    }
}

fn risk_tier(score: u32) -> &'static str {
    match score {
        0..=39 => "low",
        40..=59 => "medium",
        _ => "high",
    }
}

fn read_existing_audit_score(repo: &Path) -> Option<i32> {
    let path = repo.join("agent/repo-score.json");
    let text = fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("score")?.as_i64().map(|score| score as i32)
}


fn render_markdown(plan: &AdoptionPlan) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Adoption Plan");
    let _ = writeln!(out);
    let _ = writeln!(out, "- mode: `{}`", plan.mode);
    let _ = writeln!(out, "- recommended profile: `{}`", plan.recommended_profile);
    let _ = writeln!(out, "- risk tier: `{}`", plan.risk_tier);
    let _ = writeln!(out, "- source stack: `{}`", plan.source_stack);
    let _ = writeln!(out, "- liability score: `{}`", plan.liability_score);
    if let Some(score) = plan.audit_score {
        let _ = writeln!(out, "- audit score: `{score}`");
    } else {
        let _ = writeln!(out, "- audit score: unavailable");
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Safe Commands");
    for command in &plan.safe_commands {
        let _ = writeln!(out, "- `{command}`");
    }
    if !plan.tool_rollout.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Tool Rollout");
        let _ = writeln!(out, "| Tool | Status | Next Command | Artifacts |");
        let _ = writeln!(out, "| --- | --- | --- | --- |");
        for item in &plan.tool_rollout {
            let artifacts = if item.artifact_paths.is_empty() {
                "none".into()
            } else {
                item.artifact_paths.join(", ")
            };
            let _ = writeln!(
                out,
                "| `{}` | `{}` | `{}` | `{}` |",
                item.id, item.status, item.next_command, artifacts
            );
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Stop Conditions");
    for condition in &plan.stop_conditions {
        let _ = writeln!(out, "- {condition}");
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Next Milestones");
    for milestone in &plan.next_milestones {
        let _ = writeln!(out, "- {milestone}");
    }
    if !plan.warnings.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Warnings");
        for warning in &plan.warnings {
            let _ = writeln!(out, "- {warning}");
        }
    }
    out
}
