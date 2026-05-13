pub mod inventory;
pub mod plan;
pub mod prompt_verify;
pub mod report;
pub mod slice_risk;

use crate::commands::repair::now_string;
use crate::validation::{self, ArtifactSchema};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub use inventory::{detect_stack, ApiSurface, ContractEvidence, DetectedItem, StackInventory};
pub use plan::{build_migration_plan, MigrationPlan, MigrationSlice};
#[allow(unused_imports)]
pub use prompt_verify::{run as run_prompt_verify, PromptVerificationReport, PromptVerifyArgs};
pub use report::{
    build_migration_report, compute_liability, LiabilityBreakdown, LiabilityDimension,
    MigrationReport,
};
#[allow(unused_imports)]
pub use slice_risk::{run as run_slice_risk, SliceRiskArgs, SliceRiskReport};

#[derive(Debug, Clone)]
pub struct MigrateArgs {
    pub repo: PathBuf,
    pub out: Option<String>,
    pub md: Option<String>,
    pub mode: MigrateMode,
    pub target: String,
}

#[derive(Debug, Clone)]
pub enum MigrateMode {
    Analyze,
    Plan,
}

pub fn run(args: MigrateArgs) -> Result<()> {
    match args.mode {
        MigrateMode::Analyze => run_analyze(
            &args.repo,
            args.out.as_deref(),
            args.md.as_deref(),
            &args.target,
        ),
        MigrateMode::Plan => run_plan(
            &args.repo,
            args.out.as_deref(),
            args.md.as_deref(),
            &args.target,
        ),
    }
}

fn run_analyze(repo: &Path, out: Option<&str>, md: Option<&str>, target: &str) -> Result<()> {
    let report = build_migration_report(repo, target)?;
    if let Some(path) = out {
        validation::write_json(repo, ArtifactSchema::MigrationReport, path, &report)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    if let Some(path) = md {
        crate::render::write_markdown(path, &report::render_report_markdown(&report))?;
    }
    Ok(())
}

fn run_plan(repo: &Path, out: Option<&str>, md: Option<&str>, target: &str) -> Result<()> {
    let plan = build_migration_plan(repo, target)?;
    if let Some(path) = out {
        validation::write_json(repo, ArtifactSchema::MigrationPlan, path, &plan)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    }
    if let Some(path) = md {
        crate::render::write_markdown(path, &plan::render_plan_markdown(&plan))?;
    }
    Ok(())
}
