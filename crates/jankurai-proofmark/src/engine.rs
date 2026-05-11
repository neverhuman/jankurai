use anyhow::Result;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::coverage::{changed_lines_for_paths, load_coverage};
use crate::render::{render_markdown, standard_proof_receipt};
use crate::report::{
    changed_unit, coverage_summary, load_mutation, load_obligations, obligation_result,
    proofmark_summary,
};
use crate::shared::{elapsed_ms, git_output, resolve_changed_paths, unix_seconds};
use crate::{ProofMarkMode, ProofMarkOutput, ProofMarkReceipt};

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_proofmark_output(
    repo: PathBuf,
    changed_paths: Vec<PathBuf>,
    changed_from: Option<String>,
    obligations_path: Option<PathBuf>,
    coverage_path: Option<PathBuf>,
    mutation_path: Option<PathBuf>,
    negative_proofs: Vec<String>,
    mode: ProofMarkMode,
) -> Result<ProofMarkOutput> {
    let started = SystemTime::now();
    let changed_paths =
        resolve_changed_paths(&repo, &changed_paths, changed_from.as_deref(), |path| {
            path.ends_with(".rs")
        })?;
    let coverage = load_coverage(&repo, coverage_path.as_deref())?;
    let mutation = load_mutation(&repo, mutation_path.as_deref())?;
    let changed_lines = changed_lines_for_paths(&repo, changed_from.as_deref(), &changed_paths);
    let changed_units = changed_paths
        .iter()
        .filter(|path| path.ends_with(".rs"))
        .map(|path| changed_unit(path, changed_lines.get(path), &coverage))
        .collect::<Vec<_>>();

    let obligations = load_obligations(&repo, obligations_path.as_deref())?;
    let negative_proofs = negative_proofs
        .iter()
        .map(|item| item.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let obligation_results = obligations
        .iter()
        .filter(|obligation| {
            obligation
                .required_lanes
                .iter()
                .any(|lane| lane == "proofmark-rust")
                || obligation.path.ends_with(".rs")
        })
        .map(|obligation| {
            obligation_result(obligation, &changed_units, &mutation, &negative_proofs)
        })
        .collect::<Vec<_>>();
    let satisfied_obligations = obligation_results
        .iter()
        .filter(|result| result.status == "pass")
        .map(|result| result.obligation_id.clone())
        .collect::<Vec<_>>();
    let coverage_summary =
        coverage_summary(coverage_path.as_deref(), &changed_units, coverage.loaded);
    let summary = proofmark_summary(&changed_units, &obligation_results, mode);
    let generated_at = unix_seconds();
    let git_head = if let Some(head) = git_output(&repo, &["rev-parse", "--short", "HEAD"]) {
        head
    } else {
        "unknown".into()
    };
    let receipt = ProofMarkReceipt {
        schema_version: "1.0.0".into(),
        standard_version: crate::PROOFMARK_STANDARD_VERSION.into(),
        generated_at: generated_at.clone(),
        repo_root: repo.display().to_string(),
        git_head: git_head.clone(),
        mode: mode.as_str().into(),
        changed_paths: changed_paths.clone(),
        changed_units,
        coverage: coverage_summary,
        mutation,
        obligation_results,
        satisfied_obligations,
        summary,
    };
    let proof_receipt = standard_proof_receipt(
        &repo,
        &receipt,
        elapsed_ms(started),
        &generated_at,
        &git_head,
    );
    let markdown = render_markdown(&receipt);
    Ok(ProofMarkOutput {
        receipt,
        proof_receipt,
        markdown,
    })
}
