use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::PathBuf;

pub mod coverage;
pub mod engine;
pub mod render;
pub mod report;
pub mod shared;

pub const PROOFMARK_SCHEMA_VERSION: &str = "1.0.0";
pub const PROOFMARK_STANDARD_VERSION: &str = "0.7.0";
pub const PROOFMARK_AUDITOR_VERSION: &str = "0.7.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofMarkMode {
    Advisory,
    Required,
}

impl ProofMarkMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::Required => "required",
        }
    }
}

impl std::str::FromStr for ProofMarkMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "advisory" => Ok(Self::Advisory),
            "required" => Ok(Self::Required),
            other => anyhow::bail!("unknown proofmark mode `{other}`"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProofMarkRequest {
    pub repo_root: PathBuf,
    pub changed_paths: Vec<PathBuf>,
    pub changed_from: Option<String>,
    pub obligations_path: Option<PathBuf>,
    pub coverage_path: Option<PathBuf>,
    pub mutation_path: Option<PathBuf>,
    pub negative_proofs: Vec<String>,
    pub mode: ProofMarkMode,
}

#[derive(Debug, Clone)]
pub struct ProofMarkOutput {
    pub receipt: ProofMarkReceipt,
    pub proof_receipt: StandardProofReceipt,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMarkReceipt {
    pub schema_version: String,
    pub standard_version: String,
    pub generated_at: String,
    pub repo_root: String,
    pub git_head: String,
    pub mode: String,
    pub changed_paths: Vec<String>,
    pub changed_units: Vec<ChangedUnit>,
    pub coverage: CoverageSummary,
    pub mutation: MutationSummary,
    pub obligation_results: Vec<ObligationResult>,
    pub satisfied_obligations: Vec<String>,
    pub summary: ProofMarkSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedUnit {
    pub path: String,
    pub unit: String,
    pub changed_lines: Vec<u32>,
    pub covered_changed_lines: Vec<u32>,
    pub uncovered_changed_lines: Vec<u32>,
    pub coverage_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    pub source: String,
    pub changed_line_count: usize,
    pub covered_changed_line_count: usize,
    pub uncovered_changed_line_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationSummary {
    pub source: String,
    pub status: String,
    pub killed: usize,
    pub survived: usize,
    pub timeout: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObligationResult {
    pub obligation_id: String,
    pub path: String,
    pub rule_ids: Vec<String>,
    pub required_lanes: Vec<String>,
    pub status: String,
    pub coverage_status: String,
    pub mutation_status: String,
    pub negative_proof_status: String,
    pub evidence: Vec<String>,
    pub residual_risk: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMarkSummary {
    pub total_obligations: usize,
    pub satisfied_obligations: usize,
    pub review_obligations: usize,
    pub changed_units: usize,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardProofReceipt {
    pub schema_version: String,
    pub standard_version: String,
    pub auditor_version: String,
    pub receipt_id: String,
    pub lane: String,
    pub command: String,
    pub exit_code: i32,
    pub elapsed_ms: u128,
    pub artifacts: Vec<String>,
    pub changed_paths: Vec<String>,
    pub generated_at: String,
    pub repo_root: String,
    pub git_head: String,
    pub dirty_worktree: bool,
    pub rules_covered: Vec<RuleCoverage>,
    pub extensions: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCoverage {
    pub rule_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct ProofBindObligation {
    #[serde(default)]
    pub obligation_id: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub rule_ids: Vec<String>,
    #[serde(default)]
    pub required_lanes: Vec<String>,
    #[serde(default)]
    pub required_receipt_kinds: Vec<String>,
}

pub fn build_proofmark(request: ProofMarkRequest) -> Result<ProofMarkOutput> {
    engine::build_proofmark_output(
        request.repo_root,
        request.changed_paths,
        request.changed_from,
        request.obligations_path,
        request.coverage_path,
        request.mutation_path,
        request.negative_proofs,
        request.mode,
    )
}
