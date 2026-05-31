use serde::Serialize;
use std::collections::BTreeMap;

pub const STANDARD_VERSION: &str = "0.9.0";
pub const AUDITOR_VERSION: &str = "1.6.7";
pub const SCHEMA_VERSION: &str = "1.9.0";
pub const PAPER_EDITION: &str = "2026.05-ed8";
pub const TARGET_STACK_ID: &str = "rust-ts-vite-react-postgres-bounded-python";
pub const TARGET_STACK: &str = "Rust core + TypeScript/React/Vite + PostgreSQL + generated contracts + exception-only Python AI/data service";

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub rel_path: String,
    pub name: String,
    pub suffix: String,
    pub size: u64,
    pub line_count: usize,
    pub text: String,
    pub is_generated: bool,
    pub is_code: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DimensionResult {
    pub name: String,
    pub weight: u32,
    pub score: i32,
    pub weighted_points: f64,
    pub evidence: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub severity: String,
    pub category: String,
    pub path: String,
    pub problem: String,
    pub agent_fix: String,
    pub evidence: Vec<String>,
    pub check_id: String,
    pub hardness: String,
    pub confidence: f64,
    pub evidence_kind: String,
    pub rerun_command: String,
    pub fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tlr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_term: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportDecision {
    pub status: String,
    pub minimum_score: i32,
    pub passed: bool,
    pub hard_findings: usize,
    pub soft_findings: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratchet: Option<ReportRatchet>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportRatchet {
    pub baseline_score: i32,
    pub allowed_drop: i32,
    pub passed: bool,
    pub score_delta: i32,
    pub baseline_report_fingerprint: String,
    pub baseline_input_fingerprint: String,
    pub baseline_policy_fingerprint: String,
    pub new_caps: Vec<String>,
    pub new_hard_findings: Vec<String>,
    pub policy_changed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitSummary {
    pub head: Option<String>,
    pub base: Option<String>,
    pub changed_files: usize,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirty_worktree: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PolicySummary {
    pub path: String,
    pub minimum_score: i32,
    pub fail_on: Vec<String>,
    pub advisory_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auditor_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_edition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_stack: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RuleCoverage {
    Rich { rule_id: String, status: String },
    Simple(String),
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ArtifactDigest {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ProofReceipt {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auditor_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<String>,
    pub lane: String,
    pub command: String,
    pub exit_code: i32,
    pub elapsed_ms: u128,
    pub artifacts: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub changed_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub residual_risk: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_head: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirty_worktree: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_sha256: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub artifact_digests: Vec<ArtifactDigest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules_covered: Vec<RuleCoverage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_stderr_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UxQaReportArtifactSummary {
    /// Repo-relative path (POSIX slashes).
    pub path: String,
    pub report_count: usize,
    /// Worst decision across reports: `block` > `review` > `warn` > `pass`.
    pub worst_decision: String,
    pub total_violations: usize,
    /// Sum of per-report `summary.errors`.
    pub summary_errors: u64,
    /// Sum of per-report `summary.warnings`.
    pub summary_warnings: u64,
    pub reports_missing_required_states: usize,
    pub missing_state_names: Vec<String>,
    pub artifact_counts_by_kind: BTreeMap<String, usize>,
    pub reports_missing_required_artifacts: usize,
    pub missing_artifact_kinds: Vec<String>,
    pub reports_missing_required_accessibility_artifact: usize,
    pub accessibility_violation_total: u64,
    pub accessibility_incomplete_total: u64,
    pub accessibility_pass_total: u64,
    pub artifact_fingerprint_count: usize,
    pub visual_baseline_missing: usize,
    pub visual_baseline_changed: usize,
    pub visual_baseline_review: usize,
    pub visual_baseline_block: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct UxQaReadiness {
    pub web_surface: bool,
    pub has_rendered_ux_lane: bool,
    pub missing_categories: Vec<String>,
    pub evidence: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<UxQaReportArtifactSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolAdoptionItem {
    pub id: String,
    pub category: String,
    pub mode: String,
    pub applicable: bool,
    pub status: String,
    pub replaced_tools: Vec<String>,
    pub evidence: Vec<String>,
    pub missing: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ci_command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolAdoptionReadiness {
    pub control_plane_present: bool,
    pub applicable_count: usize,
    pub configured_count: usize,
    pub ci_evidence_count: usize,
    pub artifact_verified_count: usize,
    pub replaced_count: usize,
    pub items: Vec<ToolAdoptionItem>,
    pub evidence: serde_json::Value,
    pub missing: Vec<String>,
}

/// Compact summary of validated `target/jankurai/security/evidence.json` for repo-score (audit-only).
#[derive(Debug, Clone, Serialize)]
pub struct SecurityEvidenceArtifactSummary {
    /// Repo-relative path (POSIX slashes).
    pub path: String,
    pub envelope_exit_code: i32,
    pub elapsed_ms: u64,
    pub wrapper_strict: bool,
    pub profile: String,
    pub commands_ran: usize,
    pub commands_skipped: usize,
    pub commands_failed: usize,
    pub required_commands_skipped: usize,
    pub required_commands_failed: usize,
    pub blocking_commands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_head: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SecurityEvidenceReadiness {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<SecurityEvidenceArtifactSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryEvidenceArtifactSummary {
    pub path: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    pub file_count: usize,
    pub check_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryReclassification {
    pub id: String,
    pub paths: Vec<String>,
    pub classification: String,
    pub product_surface: bool,
    pub runtime_language: String,
    pub status: String,
    pub reclassified_caps: Vec<String>,
    pub covered_file_count: usize,
    pub covered_line_count: usize,
    pub covered_files: Vec<String>,
    pub evidence_artifacts: Vec<BoundaryEvidenceArtifactSummary>,
    pub missing_checks: Vec<String>,
    pub failed_checks: Vec<String>,
    pub rerun_command: String,
    #[serde(skip_serializing)]
    pub suppresses_python_stack_caps: bool,
}

/// Compact summary of validated `agent/boundaries.toml` for repo-score (audit-only).
#[derive(Debug, Clone, Serialize)]
pub struct BoundariesManifestSummary {
    /// Repo-relative path (POSIX slashes).
    pub path: String,
    /// SHA-256 fingerprint of raw manifest bytes (same convention as `manifest_fingerprints`).
    pub content_fingerprint: String,
    pub stack_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_version: Option<String>,
    pub adapter_path_count: usize,
    pub event_contract_path_count: usize,
    pub generated_type_path_count: usize,
    pub client_marker_count: usize,
    pub streaming_exception_count: usize,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct BoundariesReadiness {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<BoundariesManifestSummary>,
    #[serde(default)]
    pub reclassifications: Vec<BoundaryReclassification>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileStructureCell {
    pub id: String,
    pub applicable: bool,
    pub status: String,
    pub canonical_path: String,
    pub detected_paths: Vec<String>,
    pub aliases: Vec<String>,
    pub guidance_status: String,
    pub owner: String,
    pub proof_lane: String,
    pub agent_fix: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileStructureReadiness {
    pub applicable_count: usize,
    pub canonical_count: usize,
    pub noncanonical_count: usize,
    pub guidance_missing_count: usize,
    pub cells: Vec<ProfileStructureCell>,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct VibeCoverageGap {
    pub id: String,
    pub name: String,
    pub coverage: String,
    pub priority: String,
    pub gap: String,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct VibeCoverageSummary {
    pub source_path: String,
    pub issue_count: usize,
    pub source_ref_count: usize,
    pub unmapped_source_rows: usize,
    pub coverage_counts: BTreeMap<String, usize>,
    pub tlr_counts: BTreeMap<String, usize>,
    pub priority_counts: BTreeMap<String, usize>,
    pub top_gaps: Vec<VibeCoverageGap>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct CoverageEvidenceSummary {
    pub artifact: String,
    pub status: String,
    pub sources_total: usize,
    pub sources_present: usize,
    pub hard_findings: usize,
    pub soft_findings: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub report_fingerprint: String,
    pub input_fingerprint: String,
    pub policy_fingerprint: String,
    pub manifest_fingerprints: ManifestFingerprints,
    pub dirty_worktree: bool,
    pub generated_at: String,
    pub schema_url: String,
    pub standard: String,
    pub standard_version: String,
    pub auditor_version: String,
    pub schema_version: String,
    pub paper_edition: String,
    pub target_stack_id: String,
    pub target_stack: String,
    pub claimed_conformance_level: String,
    pub observed_conformance_level: String,
    pub conformance_decision: String,
    pub conformance_blockers: Vec<String>,
    pub repo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u128>,
    pub scope: Scope,
    pub score: i32,
    pub raw_score: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<ReportDecision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<PolicySummary>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub proof_receipts: Vec<ProofReceipt>,
    pub caps_applied: Vec<String>,
    pub hard_rules: Vec<HardRule>,
    pub dimensions: Vec<DimensionResult>,
    pub ux_qa: UxQaReadiness,
    pub tool_adoption: ToolAdoptionReadiness,
    pub security_evidence: SecurityEvidenceReadiness,
    pub boundaries: BoundariesReadiness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy_code: Option<crate::audit::copy_code::CopyCodeReport>,
    pub profile_structure: ProfileStructureReadiness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vibe_coverage: Option<VibeCoverageSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_evidence: Option<CoverageEvidenceSummary>,
    pub findings: Vec<Finding>,
    pub agent_fix_queue: Vec<AgentFix>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Default)]
pub struct ManifestFingerprints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_map: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_map: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_zones: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundaries: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_lanes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Scope {
    pub mode: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HardRule {
    pub id: String,
    pub max_score: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentFix {
    pub path: String,
    pub priority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tlr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub task: String,
    pub why: String,
}
