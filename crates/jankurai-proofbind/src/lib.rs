use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub mod catalog;
pub mod classify;
pub mod receipts;
pub mod shared;
pub mod summary;
pub mod surface_rules;

use catalog::Catalog;
use classify::{classify_changed_path, obligation_for_surface};
use receipts::load_receipts;
use shared::{git_output, resolve_changed_paths, unix_seconds};
use summary::{obligation_summary, render_markdown, surface_summary};

pub const PROOFBIND_SCHEMA_VERSION: &str = "1.0.0";
pub const PROOFBIND_STANDARD_VERSION: &str = "0.7.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofBindMode {
    Advisory,
    Required,
}

impl ProofBindMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::Required => "required",
        }
    }
}

impl std::str::FromStr for ProofBindMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "advisory" => Ok(Self::Advisory),
            "required" => Ok(Self::Required),
            other => anyhow::bail!("unknown proofbind mode `{other}`"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProofBindRequest {
    pub repo_root: PathBuf,
    pub changed_paths: Vec<PathBuf>,
    pub changed_from: Option<String>,
    pub mode: ProofBindMode,
    pub proof_receipts: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ProofBindOutput {
    pub witness: SurfaceWitness,
    pub obligations: ProofBindObligations,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceWitness {
    pub schema_version: String,
    pub standard_version: String,
    pub generated_at: String,
    pub repo_root: String,
    pub git_head: String,
    pub mode: String,
    pub changed_paths: Vec<String>,
    pub surfaces: Vec<ChangedSurface>,
    pub summary: SurfaceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurfaceSummary {
    pub changed_surface_count: usize,
    pub high_or_critical_surface_count: usize,
    pub by_surface_type: BTreeMap<String, usize>,
    pub by_owner: BTreeMap<String, usize>,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedSurface {
    pub surface_id: String,
    pub path: String,
    pub symbol: String,
    pub surface_type: String,
    pub severity: String,
    pub risk_tags: Vec<String>,
    pub owner: String,
    pub owner_route: String,
    pub test_route: String,
    pub proof_lane: String,
    pub required_rules: Vec<String>,
    pub required_lanes: Vec<String>,
    pub repair_tasks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofBindObligations {
    pub schema_version: String,
    pub standard_version: String,
    pub generated_at: String,
    pub repo_root: String,
    pub git_head: String,
    pub mode: String,
    pub obligations: Vec<ProofObligation>,
    pub summary: ObligationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObligationSummary {
    pub total: usize,
    pub satisfied: usize,
    pub missing: usize,
    pub high_or_critical_missing: usize,
    pub changed_surface_count: usize,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofObligation {
    pub obligation_id: String,
    pub surface_id: String,
    pub path: String,
    pub symbol: String,
    pub surface_type: String,
    pub severity: String,
    pub risk_tags: Vec<String>,
    pub rule_ids: Vec<String>,
    pub required_lanes: Vec<String>,
    pub required_receipt_kinds: Vec<String>,
    pub repair_task: String,
    pub satisfied: bool,
    pub status: String,
    pub receipt_paths: Vec<String>,
}

pub fn build_proofbind(request: ProofBindRequest) -> Result<ProofBindOutput> {
    let repo = request.repo_root;
    let changed_paths = resolve_changed_paths(
        &repo,
        &request.changed_paths,
        request.changed_from.as_deref(),
        |_| true,
    )?;
    let catalog = Catalog::load(&repo);
    let mut surfaces = Vec::new();
    for path in &changed_paths {
        surfaces.extend(classify_changed_path(&repo, &catalog, path)?);
    }
    surfaces.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.surface_type.cmp(&b.surface_type))
            .then(a.symbol.cmp(&b.symbol))
    });
    surfaces.dedup_by(|a, b| a.surface_id == b.surface_id);

    let receipts = load_receipts(&repo, request.proof_receipts.as_deref())?;
    let mut obligations = surfaces
        .iter()
        .map(|surface| obligation_for_surface(surface, &receipts))
        .collect::<Vec<_>>();
    obligations.sort_by(|a, b| a.obligation_id.cmp(&b.obligation_id));

    let generated_at = unix_seconds();
    let git_head = if let Some(head) = git_output(&repo, &["rev-parse", "--short", "HEAD"]) {
        head
    } else {
        "unknown".into()
    };
    let witness_summary = surface_summary(&surfaces);
    let obligation_summary = obligation_summary(&surfaces, &obligations, request.mode);
    let witness = SurfaceWitness {
        schema_version: PROOFBIND_SCHEMA_VERSION.into(),
        standard_version: PROOFBIND_STANDARD_VERSION.into(),
        generated_at: generated_at.clone(),
        repo_root: repo.display().to_string(),
        git_head: git_head.clone(),
        mode: request.mode.as_str().into(),
        changed_paths,
        surfaces,
        summary: witness_summary,
    };
    let obligations = ProofBindObligations {
        schema_version: PROOFBIND_SCHEMA_VERSION.into(),
        standard_version: PROOFBIND_STANDARD_VERSION.into(),
        generated_at,
        repo_root: repo.display().to_string(),
        git_head,
        mode: request.mode.as_str().into(),
        obligations,
        summary: obligation_summary,
    };
    let markdown = render_markdown(&witness, &obligations);
    Ok(ProofBindOutput {
        witness,
        obligations,
        markdown,
    })
}
