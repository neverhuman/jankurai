use serde_json::{json, Map};
use std::path::Path;

use crate::shared::git_dirty;
use crate::{ProofMarkReceipt, RuleCoverage, StandardProofReceipt};

pub(crate) fn standard_proof_receipt(
    repo: &Path,
    receipt: &ProofMarkReceipt,
    elapsed_ms: u128,
    generated_at: &str,
    git_head: &str,
) -> StandardProofReceipt {
    let mut rules = std::collections::BTreeMap::<String, String>::new();
    for result in &receipt.obligation_results {
        for rule in &result.rule_ids {
            let status = if result.status == "pass" {
                "covered"
            } else {
                "review"
            };
            rules
                .entry(rule.clone())
                .and_modify(|existing| {
                    if *existing == "covered" && status != "covered" {
                        *existing = status.into();
                    }
                })
                .or_insert_with(|| status.into());
        }
    }
    let mut extensions = Map::new();
    extensions.insert(
        "proofmark".into(),
        json!({
            "schema_version": receipt.schema_version,
            "changed_units": receipt.changed_units,
            "coverage": receipt.coverage,
            "mutation": receipt.mutation,
            "obligation_results": receipt.obligation_results,
            "satisfied_obligations": receipt.satisfied_obligations,
            "summary": receipt.summary,
        }),
    );
    StandardProofReceipt {
        schema_version: "1.0.0".into(),
        standard_version: crate::PROOFMARK_STANDARD_VERSION.into(),
        auditor_version: crate::PROOFMARK_AUDITOR_VERSION.into(),
        receipt_id: format!("proofmark-rust-{}", generated_at),
        lane: "proofmark-rust".into(),
        command: "jankurai proofmark rust".into(),
        exit_code: 0,
        elapsed_ms,
        artifacts: vec![
            "target/jankurai/proofmark/proofmark-receipt.json".into(),
            "target/jankurai/proofmark/proofmark.md".into(),
        ],
        changed_paths: receipt.changed_paths.clone(),
        generated_at: generated_at.into(),
        repo_root: repo.display().to_string(),
        git_head: git_head.into(),
        dirty_worktree: git_dirty(repo),
        rules_covered: rules
            .into_iter()
            .map(|(rule_id, status)| RuleCoverage { rule_id, status })
            .collect(),
        extensions,
    }
}

pub(crate) fn render_markdown(receipt: &ProofMarkReceipt) -> String {
    let mut out = String::new();
    out.push_str("# jankurai ProofMark\n\n");
    out.push_str(&format!("- mode: `{}`\n", receipt.mode));
    out.push_str(&format!(
        "- changed units: `{}`\n",
        receipt.summary.changed_units
    ));
    out.push_str(&format!(
        "- obligations: total=`{}` satisfied=`{}` review=`{}` verdict=`{}`\n",
        receipt.summary.total_obligations,
        receipt.summary.satisfied_obligations,
        receipt.summary.review_obligations,
        receipt.summary.verdict
    ));
    out.push_str(&format!(
        "- coverage: status=`{}` changed=`{}` covered=`{}` uncovered=`{}`\n",
        receipt.coverage.status,
        receipt.coverage.changed_line_count,
        receipt.coverage.covered_changed_line_count,
        receipt.coverage.uncovered_changed_line_count
    ));
    out.push_str(&format!(
        "- mutation: status=`{}` killed=`{}` survived=`{}` timeout=`{}`\n",
        receipt.mutation.status,
        receipt.mutation.killed,
        receipt.mutation.survived,
        receipt.mutation.timeout
    ));
    out.push_str("\n## Obligation Results\n");
    if receipt.obligation_results.is_empty() {
        out.push_str("- none\n");
    } else {
        for result in &receipt.obligation_results {
            out.push_str(&format!(
                "- `{}` status=`{}` coverage=`{}` mutation=`{}` negative=`{}`\n",
                result.path,
                result.status,
                result.coverage_status,
                result.mutation_status,
                result.negative_proof_status
            ));
        }
    }
    out
}
