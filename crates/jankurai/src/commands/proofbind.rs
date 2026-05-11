use crate::validation::{self, ArtifactSchema};
use anyhow::Result;
use jankurai_proofbind::{build_proofbind, ProofBindMode, ProofBindRequest};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ProofBindMapArgs {
    pub repo: PathBuf,
    pub changed: Vec<PathBuf>,
    pub changed_from: Option<String>,
    pub mode: String,
    pub proof_receipts: String,
    pub out: String,
    pub obligations_out: String,
    pub md: String,
}

#[derive(Debug, Clone)]
pub struct ProofBindVerifyArgs {
    pub repo: PathBuf,
    pub changed: Vec<PathBuf>,
    pub changed_from: Option<String>,
    pub mode: String,
    pub proof_receipts: String,
    pub out: String,
    pub obligations_out: String,
    pub md: String,
}

pub fn run_map(args: ProofBindMapArgs) -> Result<()> {
    write_outputs(args)
}

pub fn run_verify(args: ProofBindVerifyArgs) -> Result<()> {
    write_outputs(ProofBindMapArgs {
        repo: args.repo,
        changed: args.changed,
        changed_from: args.changed_from,
        mode: args.mode,
        proof_receipts: args.proof_receipts,
        out: args.out,
        obligations_out: args.obligations_out,
        md: args.md,
    })
}

fn write_outputs(args: ProofBindMapArgs) -> Result<()> {
    let mode = args
        .mode
        .parse::<ProofBindMode>()
        .unwrap_or(ProofBindMode::Advisory);
    let output = build_proofbind(ProofBindRequest {
        repo_root: args.repo.clone(),
        changed_paths: args.changed,
        changed_from: args.changed_from,
        mode,
        proof_receipts: Some(PathBuf::from(&args.proof_receipts)),
    })?;
    ensure_parent(&args.out)?;
    ensure_parent(&args.obligations_out)?;
    ensure_parent(&args.md)?;
    validation::write_json(
        &args.repo,
        ArtifactSchema::ProofBindWitness,
        &args.out,
        &output.witness,
    )?;
    validation::write_json(
        &args.repo,
        ArtifactSchema::ProofBindObligations,
        &args.obligations_out,
        &output.obligations,
    )?;
    crate::render::write_markdown(&args.md, &output.markdown)?;
    if mode == ProofBindMode::Required && output.obligations.summary.missing > 0 {
        anyhow::bail!(
            "proofbind required mode has {} missing obligation(s)",
            output.obligations.summary.missing
        );
    }
    Ok(())
}

fn ensure_parent(path: &str) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}
