use crate::audit::helpers::*;
use crate::audit::scan;
use crate::model::DimensionResult;

pub fn analyze(ctx: &AuditContext) -> DimensionResult {
    let mut score = 35;
    let mut evidence = vec![];
    let notes = vec![];
    if has_contract_surface(ctx) {
        score += 15;
        evidence.push("contract surface found".into());
    }
    if has_generated_contracts(ctx) {
        score += 15;
        evidence.push("generated contract artifacts found".into());
    }
    if has_polyglot_boundary(ctx) {
        score += 10;
        evidence.push("polyglot boundary layout present".into());
    }
    if has_api_drift_checks(ctx) {
        score += 10;
        evidence.push("public API drift checks found".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.rel_path == "agent/boundaries.toml")
    {
        score += 10;
        evidence.push("boundary manifest present".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.rel_path.starts_with("schemas/") && f.name.ends_with(".schema.json"))
    {
        score += 10;
        evidence.push("machine-readable schemas present".into());
    }
    if has_web_surface(ctx)
        && ctx.all_files.iter().any(|f| {
            f.rel_path == "tsconfig.json" && f.text.to_ascii_lowercase().contains("strict")
        })
    {
        score += 10;
        evidence.push("TypeScript strict mode hinted by `tsconfig.json`".into());
    }
    if has_rust_surface(ctx)
        && ctx.all_files.iter().any(|f| {
            f.text.contains("serde") || f.text.contains("thiserror") || f.text.contains("anyhow")
        })
    {
        score += 8;
        evidence.push("Rust typed boundary helpers found".into());
    }
    if handwritten_api_hits(ctx) {
        score -= 15;
        evidence.push("handwritten web DTO/API marker found".into());
    }
    let orphaned = scan::contract_source_hits(ctx);
    if !orphaned.is_empty() {
        score -= 10;
        evidence.push(format!(
            "contract sources without generated zones: {}",
            orphaned.len()
        ));
    } else if has_contract_surface(ctx) {
        score += 5;
        evidence.push("all contract sources have generated zone entries".into());
    }
    if !scan::wrong_layer_db_hits(ctx).is_empty() {
        score -= 10;
        evidence.push("DB access found in likely wrong layer".into());
    }
    if !scan::streaming_runtime_hits(ctx).is_empty() {
        score -= 12;
        evidence.push("streaming client found outside queue adapter boundary".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.rel_path.starts_with("schemas/") && f.name.ends_with(".schema.json"))
        && has_generated_contracts(ctx)
        && scan::contract_source_hits(ctx).is_empty()
        && scan::wrong_layer_db_hits(ctx).is_empty()
        && scan::streaming_runtime_hits(ctx).is_empty()
    {
        score += 5;
        evidence.push("schema/tooling contract posture is clean".into());
    }
    make_dim("Contract and boundary integrity", score, evidence, notes)
}
