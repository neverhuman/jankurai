use super::report::build_migration_report;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub schema_version: String,
    pub command: String,
    pub status: String,
    pub generated_at: String,
    pub source_report: String,
    pub target_stack: String,
    pub plan_mode: String,
    pub slices: Vec<MigrationSlice>,
    pub human_approval_requirements: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSlice {
    pub slice_id: String,
    pub owner: String,
    pub status: String,
    pub risk_level: String,
    pub dependency_order: u32,
    pub human_approval_required: bool,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub contracts: Vec<String>,
    pub tests: Vec<String>,
    pub proof_lanes: Vec<String>,
    pub rollback_notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cutover_notes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

pub fn build_migration_plan(repo: &Path, target: &str) -> Result<MigrationPlan> {
    let report = build_migration_report(repo, target)?;

    let mut slices = vec![];
    let mut order: u32 = 1;

    if let Some(ref db_surfaces) = report.db_surfaces {
        for (i, db) in db_surfaces.iter().enumerate() {
            let risk = if report.liability_score > 60 {
                "high"
            } else {
                "medium"
            };
            slices.push(MigrationSlice {
                slice_id: format!("db-isolation-{}", i + 1),
                owner: "tools".to_string(),
                status: "candidate".to_string(),
                risk_level: risk.to_string(),
                dependency_order: order,
                human_approval_required: risk == "high",
                allowed_paths: vec![format!("crates/adapters/db/{}", db.replace("db-client:", ""))],
                forbidden_paths: vec!["crates/domain/".to_string()],
                contracts: vec!["adapter boundary interface".to_string()],
                tests: vec!["adapter integration test".to_string()],
                proof_lanes: vec!["db".to_string(), "fast".to_string()],
                rollback_notes: vec!["revert adapter extraction if contract tests fail".to_string()],
                cutover_notes: None,
                notes: Some(format!("isolate {db} behind adapter boundary")),
            });
            order += 1;
        }
    }

    if let Some(ref api_surfaces) = report.api_surfaces {
        for (i, api) in api_surfaces.iter().enumerate() {
            slices.push(MigrationSlice {
                slice_id: format!("api-contract-{}", i + 1),
                owner: "tools".to_string(),
                status: "candidate".to_string(),
                risk_level: "medium".to_string(),
                dependency_order: order,
                human_approval_required: false,
                allowed_paths: vec!["contracts/".to_string()],
                forbidden_paths: vec![],
                contracts: vec!["OpenAPI or JSON Schema contract".to_string()],
                tests: vec!["consumer/provider contract test".to_string()],
                proof_lanes: vec!["contract".to_string(), "fast".to_string()],
                rollback_notes: vec![
                    "revert contract extraction if provider tests fail".to_string()
                ],
                cutover_notes: None,
                notes: Some(format!("extract contract for {api}")),
            });
            order += 1;
        }
    }

    slices.push(MigrationSlice {
        slice_id: "equivalence-proof".to_string(),
        owner: "tools".to_string(),
        status: if slices.is_empty() {
            "blocked".to_string()
        } else {
            "candidate".to_string()
        },
        risk_level: "high".to_string(),
        dependency_order: order,
        human_approval_required: true,
        allowed_paths: vec!["tests/equivalence/".to_string()],
        forbidden_paths: vec![],
        contracts: vec!["golden input/output equivalence".to_string()],
        tests: vec!["equivalence comparison test".to_string()],
        proof_lanes: vec!["fast".to_string()],
        rollback_notes: vec!["equivalence failures block cutover".to_string()],
        cutover_notes: Some(vec![
            "shadow reads recommended before cutover".to_string(),
            "parallel-run comparison for critical paths".to_string(),
        ]),
        notes: Some("prove equivalent behavior before retiring old code".to_string()),
    });

    let mut human_approvals = vec!["high-risk cutovers require human review".to_string()];
    if report.liability_score > 70 {
        human_approvals
            .push("liability score above 70 — all slices require human approval".to_string());
    }

    Ok(MigrationPlan {
        schema_version: "1.0.0".to_string(),
        command: "jankurai migrate".to_string(),
        status: "complete".to_string(),
        generated_at: super::now_string(),
        source_report: "target/jankurai/migration-report.json".to_string(),
        target_stack: report.target_stack.clone(),
        plan_mode: "dry-run".to_string(),
        slices,
        human_approval_requirements: human_approvals,
        commands: Some(vec![
            "jankurai migrate analyze . --json target/jankurai/migration-report.json".to_string(),
            "jankurai migrate plan . --json target/jankurai/migration-plan.json".to_string(),
        ]),
        warnings: if report.liability_score > 60 {
            Some(vec![format!(
                "liability score {} indicates elevated migration risk",
                report.liability_score
            )])
        } else {
            None
        },
    })
}

pub(crate) fn render_plan_markdown(plan: &MigrationPlan) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Migration Plan");
    let _ = writeln!(out);
    let _ = writeln!(out, "- target stack: `{}`", plan.target_stack);
    let _ = writeln!(out, "- plan mode: `{}`", plan.plan_mode);
    let _ = writeln!(out, "- slices: `{}`", plan.slices.len());
    let _ = writeln!(out);
    for slice in &plan.slices {
        let _ = writeln!(out, "## Slice: {}", slice.slice_id);
        let _ = writeln!(out, "- owner: `{}`", slice.owner);
        let _ = writeln!(out, "- status: `{}`", slice.status);
        let _ = writeln!(out, "- proof lanes: `{}`", slice.proof_lanes.join(", "));
        if let Some(ref notes) = slice.notes {
            let _ = writeln!(out, "- notes: {notes}");
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(out, "## Human Approval Requirements");
    for req in &plan.human_approval_requirements {
        let _ = writeln!(out, "- {req}");
    }
    out
}
