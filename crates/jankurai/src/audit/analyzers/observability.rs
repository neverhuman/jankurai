use crate::audit::helpers::*;
use crate::model::DimensionResult;

pub fn analyze(ctx: &AuditContext) -> DimensionResult {
    let mut score = 35;
    let mut evidence = vec![];
    let mut notes = vec![];
    let files = product_files(ctx);
    if files.is_empty()
        && ctx
            .all_files
            .iter()
            .any(|f| f.rel_path == "schemas/repo-score.schema.json")
    {
        return make_dim(
            "Observability and repair evidence",
            90,
            vec![
                "no adopter product runtime in scope; report attestation and repair schemas are present"
                    .into(),
            ],
            vec![],
        );
    }
    if files.iter().any(|f| {
        f.text.contains("tracing")
            || f.text.contains("opentelemetry")
            || f.text.contains("thiserror")
            || f.text.contains("anyhow")
    }) {
        score += 15;
        evidence.push("observability libraries or patterns found".into());
    }
    if files.iter().any(|f| {
        f.text.contains("request id")
            || f.text.contains("correlation id")
            || f.text.contains("json diagnostics")
    }) {
        score += 10;
        evidence.push("diagnostic shaping hints found".into());
    }
    if has_prefix(ctx, "ops") || has_prefix(ctx, "observability") {
        score += 10;
        evidence.push("ops/observability directory present".into());
    }
    if files.iter().any(|f| {
        f.text.contains("receipt") || f.text.contains("artifact") || f.text.contains("trace")
    }) {
        score += 8;
        evidence.push("repair receipts or raw artifact language found".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.rel_path == "schemas/repair-queue.schema.json")
    {
        score += 12;
        evidence.push("repair queue schema is present".into());
    }
    if ctx.all_files.iter().any(|f| {
        f.rel_path.starts_with("crates/jankurai/src/report/")
            || f.rel_path.starts_with("packages/ux-qa/src/receipts")
    }) {
        score += 12;
        evidence.push("structured report and receipt emitters are present".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.rel_path == "schemas/repo-score.schema.json")
    {
        score += 10;
        evidence.push("report attestation schema is present".into());
    }
    let observability_docs = observability_docs_text(ctx);
    if has_agent_friendly_exceptions(ctx) || has_agent_friendly_exception_docs(&observability_docs)
    {
        score += 12;
        evidence.push("agent-friendly exception pattern found".into());
    } else if !files.is_empty() {
        score -= 12;
        notes.push("no agent-friendly exception pattern found".into());
    }
    if observability_docs.contains("repair_hint")
        || observability_docs.contains("common fixes")
        || observability_docs.contains("docs_url")
        || observability_docs.contains("phase completion receipt")
    {
        score += 8;
        evidence.push("repair-hint and receipt convention are documented".into());
    }
    if observability_docs.contains("receipt")
        || observability_docs.contains("artifact")
        || observability_docs.contains("rerun command")
    {
        score += 8;
        evidence.push("repair receipt guidance is documented".into());
    }
    if files.iter().any(|f| {
        f.text.contains("println!") || f.text.contains("console.log") || f.text.contains("print(")
    }) {
        score -= 8;
        notes.push("free-form logging appears in scope".into());
    }
    make_dim("Observability and repair evidence", score, evidence, notes)
}
