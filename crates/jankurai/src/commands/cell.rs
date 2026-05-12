use crate::commands::cell_catalog::{
    manifest_for_cell, owner_for_cell, CellEvidence, CellManifest,
};
use crate::commands::context_data::RepoCatalog;
use crate::commands::repair::now_string;
use crate::validation::{self, ArtifactSchema};
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CellArgs {
    pub repo: PathBuf,
    pub cell_id: String,
    pub mode: String,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CellPlan {
    pub schema_version: String,
    pub command: String,
    pub repo: String,
    pub generated_at: String,
    pub status: String,
    pub mode: String,
    pub lifecycle_action: String,
    pub cell_id: String,
    pub owner: String,
    pub category: String,
    pub source_paths: Vec<String>,
    pub proof_lanes: Vec<String>,
    pub manifest: CellManifest,
    pub install_plan: InstallPlan,
    pub certification_evidence: Vec<CellEvidence>,
    pub certification_decision: Option<CertificationDecision>,
    pub dependency_closure: Vec<DependencyStatus>,
    pub proof_commands: Vec<String>,
    pub upgrade_plan: Vec<String>,
    pub deprecation_plan: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CertificationDecision {
    pub status: String,
    pub merge_ready: bool,
    pub missing_evidence: Vec<String>,
    pub dependency_satisfied: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyStatus {
    pub cell_id: String,
    pub certified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    pub dry_run: bool,
    pub planned_writes: Vec<String>,
    pub conflict_policy: String,
    pub forbidden_overwrites: Vec<String>,
}

pub fn run(args: CellArgs) -> Result<()> {
    let plan = build_cell_plan(&args.repo, &args.cell_id, &args.mode)?;
    validation::validate_serializable(&args.repo, ArtifactSchema::CellManifest, &plan.manifest)?;
    if let Some(path) = args.out.as_deref() {
        crate::render::write_json(path, &serde_json::to_string_pretty(&plan)?)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&plan))?;
    }
    Ok(())
}

pub fn build_cell_plan(repo: &Path, cell_id: &str, mode: &str) -> Result<CellPlan> {
    let catalog = RepoCatalog::load(repo)?;
    let manifest = manifest_for_cell(repo, &catalog, cell_id);
    let owner = manifest
        .source_paths
        .first()
        .and_then(|path| catalog.owner_for_path(path))
        .map(|owner| owner.to_string())
        .unwrap_or_else(|| owner_for_cell(&catalog, cell_id));
    let install_plan = build_install_plan(&manifest);
    let certification_evidence =
        if mode == "prove" || mode == "upgrade-plan" || mode == "deprecate-plan" {
            manifest.certification_evidence.clone()
        } else {
            Vec::new()
        };
    let proof_commands = if mode == "prove" || mode == "upgrade-plan" || mode == "deprecate-plan" {
        manifest.proof_commands.clone()
    } else {
        Vec::new()
    };

    // Dependency closure
    let dependency_closure: Vec<DependencyStatus> =
        if mode == "prove" || mode == "upgrade-plan" || mode == "deprecate-plan" {
            manifest
                .dependencies
                .iter()
                .map(|dep_id| {
                    let dep_manifest = manifest_for_cell(repo, &catalog, dep_id);
                    DependencyStatus {
                        cell_id: dep_id.clone(),
                        certified: dep_manifest.certification_status == "certified",
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

    // Certification decision
    let certification_decision = if mode == "prove" {
        let missing: Vec<String> = manifest
            .certification_evidence
            .iter()
            .filter(|e| e.required && e.status != "present")
            .map(|e| format!("{}:{}", e.kind, e.path))
            .collect();
        let deps_ok = dependency_closure.iter().all(|d| d.certified);
        let is_certified = missing.is_empty() && deps_ok;
        Some(CertificationDecision {
            status: if is_certified {
                "certified".to_string()
            } else {
                "candidate".to_string()
            },
            merge_ready: is_certified,
            missing_evidence: missing,
            dependency_satisfied: deps_ok,
        })
    } else {
        None
    };

    let lifecycle_action = match mode {
        "prove" => "prove-certification",
        "upgrade-plan" => "upgrade-plan",
        "deprecate-plan" => "deprecate-plan",
        _ => "install-ready",
    }
    .to_string();

    let upgrade_plan = if mode == "upgrade-plan" {
        manifest.upgrade_notes.clone()
    } else {
        Vec::new()
    };

    let deprecation_plan = if mode == "deprecate-plan" {
        manifest.rollback_notes.clone()
    } else {
        Vec::new()
    };

    Ok(CellPlan {
        schema_version: "1.0.0".to_string(),
        command: "jankurai cell".to_string(),
        repo: repo.display().to_string(),
        generated_at: now_string(),
        status: "complete".to_string(),
        mode: mode.to_string(),
        lifecycle_action,
        cell_id: cell_id.to_string(),
        owner: owner.clone(),
        category: manifest.category.clone(),
        source_paths: manifest.source_paths.clone(),
        proof_lanes: manifest.proof_lanes.clone(),
        install_plan,
        certification_evidence,
        certification_decision,
        dependency_closure,
        proof_commands,
        upgrade_plan,
        deprecation_plan,
        manifest,
        notes: vec![
            "cell output is generated from current ownership and proof routing".to_string(),
            "install-ready mode emits a dry-run plan only and never writes files".to_string(),
            "prove mode emits evidence and proof commands without executing them".to_string(),
        ],
    })
}

fn build_install_plan(manifest: &CellManifest) -> InstallPlan {
    let planned_writes = manifest
        .source_paths
        .iter()
        .chain(manifest.contract_paths.iter())
        .chain(manifest.migration_paths.iter())
        .chain(manifest.ui_routes.iter())
        .chain(manifest.docs.iter())
        .cloned()
        .collect::<Vec<_>>();
    InstallPlan {
        dry_run: true,
        forbidden_overwrites: planned_writes.clone(),
        planned_writes,
        conflict_policy: manifest.conflict_policy.clone(),
    }
}

fn render_markdown(plan: &CellPlan) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Cell Plan");
    let _ = writeln!(out);
    let _ = writeln!(out, "- command: `{}`", plan.command);
    let _ = writeln!(out, "- mode: `{}`", plan.mode);
    let _ = writeln!(out, "- lifecycle action: `{}`", plan.lifecycle_action);
    let _ = writeln!(out, "- cell: `{}`", plan.cell_id);
    let _ = writeln!(out, "- owner: `{}`", plan.owner);
    let _ = writeln!(out, "- category: `{}`", plan.category);
    let _ = writeln!(
        out,
        "- certification: `{}`",
        plan.manifest.certification_status
    );
    let _ = writeln!(
        out,
        "- install strategy: `{}`",
        plan.manifest.install_strategy
    );
    let _ = writeln!(
        out,
        "- conflict policy: `{}`",
        plan.install_plan.conflict_policy
    );
    let _ = writeln!(out, "- dry run: `{}`", plan.install_plan.dry_run);
    let _ = writeln!(out, "- source paths: `{}`", plan.source_paths.join(", "));
    let _ = writeln!(out, "- proof lanes: `{}`", plan.proof_lanes.join(", "));
    let _ = writeln!(
        out,
        "- proof commands: `{}`",
        plan.proof_commands.join(", ")
    );

    if !plan.dependency_closure.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Dependency Closure");
        let _ = writeln!(out);
        for dep in &plan.dependency_closure {
            let status = if dep.certified {
                "certified"
            } else {
                "not certified"
            };
            let _ = writeln!(out, "- `{}`: {}", dep.cell_id, status);
        }
    }

    if let Some(decision) = &plan.certification_decision {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Certification Decision");
        let _ = writeln!(out);
        let _ = writeln!(out, "- status: `{}`", decision.status);
        let _ = writeln!(out, "- merge ready: `{}`", decision.merge_ready);
        let _ = writeln!(
            out,
            "- dependencies satisfied: `{}`",
            decision.dependency_satisfied
        );
        if !decision.missing_evidence.is_empty() {
            let _ = writeln!(
                out,
                "- missing evidence: `{}`",
                decision.missing_evidence.join(", ")
            );
        }
    }

    if !plan.upgrade_plan.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Upgrade Plan");
        let _ = writeln!(out);
        for note in &plan.upgrade_plan {
            let _ = writeln!(out, "- {}", note);
        }
    }

    if !plan.deprecation_plan.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Deprecation Plan");
        let _ = writeln!(out);
        for note in &plan.deprecation_plan {
            let _ = writeln!(out, "- {}", note);
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "- notes: `{}`", plan.notes.join(", "));
    out
}
