use crate::model::Report;
use std::fmt::Write;

pub fn append_proof_receipts(out: &mut String, report: &Report) {
    if report.proof_receipts.is_empty() {
        return;
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Proof Receipts");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Lane | Command | Exit | Elapsed ms | Changed | Artifacts |"
    );
    let _ = writeln!(out, "| --- | --- | ---: | ---: | --- | --- |");
    for receipt in &report.proof_receipts {
        let artifacts = if receipt.artifacts.is_empty() {
            "none".into()
        } else {
            receipt.artifacts.join(", ")
        };
        let changed = if receipt.changed_paths.is_empty() {
            "none".into()
        } else {
            receipt.changed_paths.join(", ")
        };
        let _ = writeln!(
            out,
            "| `{}` | `{}` | {} | {} | {} | {} |",
            receipt.lane,
            receipt.command,
            receipt.exit_code,
            receipt.elapsed_ms,
            changed,
            artifacts
        );
        if let Some(owner) = &receipt.owner {
            let _ = writeln!(out, "  - owner: `{}`", owner);
        }
        if let Some(reason) = &receipt.skipped_reason {
            let _ = writeln!(out, "  - skipped: `{}`", reason);
        }
        if !receipt.residual_risk.is_empty() {
            let _ = writeln!(
                out,
                "  - residual risk: `{}`",
                receipt.residual_risk.join(", ")
            );
        }
        if let Some(log_path) = &receipt.log_path {
            let _ = writeln!(out, "  - log: `{}`", log_path);
        }
    }
}
