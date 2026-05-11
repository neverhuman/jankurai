use crate::catalog::Catalog;
use crate::shared::path_symbol;
use crate::surface_rules::{
    contains_authz_marker, contains_destructive_sql, contains_input_marker, contains_process_sink,
    is_agent_tool_surface, rust_public_symbols, surface_id,
};
use crate::{ChangedSurface, ProofObligation};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub(crate) fn classify_changed_path(
    repo: &Path,
    catalog: &Catalog,
    path: &str,
) -> Result<Vec<ChangedSurface>> {
    let full_path = repo.join(path);
    let text =
        fs::read_to_string(&full_path).with_context(|| format!("read {}", full_path.display()))?;
    let lower_path = path.to_ascii_lowercase();
    let lower_text = text.to_ascii_lowercase();
    let mut surfaces = Vec::new();

    if lower_path.ends_with(".rs") {
        for symbol in rust_public_symbols(&text) {
            surfaces.push(surface(
                catalog,
                path,
                &symbol,
                "rust_public_api",
                "medium",
                vec!["public_api"],
                vec!["HLT-007-HANDWRITTEN-CONTRACT"],
                vec!["contract", "proofmark-rust"],
            ));
        }
        if lower_path.ends_with("main.rs")
            || lower_path.contains("/commands/")
            || lower_text.contains("subcommand")
        {
            let symbol = path_symbol(path);
            surfaces.push(surface(
                catalog,
                path,
                &symbol,
                "cli_command",
                "high",
                vec!["tool_surface", "operator_boundary"],
                vec!["HLT-024-AGENT-TOOL-SUPPLY-GAP"],
                vec!["security", "proofmark-rust"],
            ));
        }
        if contains_authz_marker(&lower_path, &lower_text) {
            surfaces.push(surface(
                catalog,
                path,
                "authz",
                "authz_boundary",
                "critical",
                vec![
                    "authorization",
                    "tenant_isolation",
                    "negative_proof_required",
                ],
                vec!["HLT-022-AUTHZ-ISOLATION-GAP"],
                vec!["security", "proofmark-rust"],
            ));
        }
        if contains_input_marker(&lower_path, &lower_text) {
            surfaces.push(surface(
                catalog,
                path,
                "input",
                "input_boundary",
                "high",
                vec!["input_validation", "negative_proof_required"],
                vec!["HLT-023-INPUT-BOUNDARY-GAP"],
                vec!["security", "proofmark-rust"],
            ));
        }
        if contains_process_sink(&lower_text) {
            surfaces.push(surface(
                catalog,
                path,
                "process_sink",
                "unsafe_or_process_sink",
                "high",
                vec!["process", "filesystem", "unsafe_sink"],
                vec!["HLT-023-INPUT-BOUNDARY-GAP"],
                vec!["security", "proofmark-rust"],
            ));
        }
    }

    if lower_path.ends_with(".sql") {
        let destructive = contains_destructive_sql(&lower_text);
        surfaces.push(surface(
            catalog,
            path,
            "sql",
            "sql_query",
            if destructive { "critical" } else { "high" },
            if destructive {
                vec!["sql", "destructive"]
            } else {
                vec!["sql"]
            },
            if destructive {
                vec!["HLT-021-DESTRUCTIVE-MIGRATION"]
            } else {
                vec!["HLT-006-DIRECT-DB-WRONG-LAYER"]
            },
            if destructive {
                vec!["db-migration-analyze"]
            } else {
                vec!["db"]
            },
        ));
        if lower_path.contains("migration") || lower_path.starts_with("db/") {
            surfaces.push(surface(
                catalog,
                path,
                "migration",
                "db_migration",
                if destructive { "critical" } else { "medium" },
                if destructive {
                    vec!["migration", "destructive"]
                } else {
                    vec!["migration"]
                },
                if destructive {
                    vec!["HLT-021-DESTRUCTIVE-MIGRATION"]
                } else {
                    vec!["HLT-006-DIRECT-DB-WRONG-LAYER"]
                },
                vec!["db-migration-analyze"],
            ));
        }
    }

    if is_agent_tool_surface(&lower_path, &lower_text) {
        surfaces.push(surface(
            catalog,
            path,
            "tool",
            if lower_text.contains("mcp") || lower_path.contains("mcp") {
                "mcp_tool"
            } else {
                "cli_command"
            },
            "high",
            vec!["agent_tool_supply", "tool_authority"],
            vec!["HLT-024-AGENT-TOOL-SUPPLY-GAP"],
            vec!["security"],
        ));
    }

    if surfaces.is_empty() {
        surfaces.push(surface(
            catalog,
            path,
            &path_symbol(path),
            "business_invariant",
            "medium",
            vec!["changed_behavior"],
            vec!["HLT-008-FALSE-GREEN-RISK"],
            vec!["proofmark-rust"],
        ));
    }

    Ok(surfaces)
}

#[allow(clippy::too_many_arguments)]
fn surface(
    catalog: &Catalog,
    path: &str,
    symbol: &str,
    surface_type: &str,
    severity: &str,
    risk_tags: Vec<&str>,
    required_rules: Vec<&str>,
    required_lanes: Vec<&str>,
) -> ChangedSurface {
    let (owner, owner_route) = catalog.owner_for_path(path);
    let (test_route, proof_lane) = catalog.test_for_path(path);
    ChangedSurface {
        surface_id: surface_id(surface_type, path, symbol),
        path: path.into(),
        symbol: symbol.into(),
        surface_type: surface_type.into(),
        severity: severity.into(),
        risk_tags: risk_tags.into_iter().map(str::to_string).collect(),
        owner,
        owner_route,
        test_route,
        proof_lane,
        required_rules: required_rules.into_iter().map(str::to_string).collect(),
        required_lanes: required_lanes.into_iter().map(str::to_string).collect(),
        repair_tasks: repair_tasks(surface_type, severity),
    }
}

fn required_receipt_kinds(surface: &ChangedSurface) -> Vec<String> {
    let mut kinds = vec!["proof-receipt".to_string()];
    if surface
        .required_lanes
        .iter()
        .any(|lane| lane == "proofmark-rust")
    {
        kinds.push("proofmark".into());
    }
    if surface
        .risk_tags
        .iter()
        .any(|tag| tag == "negative_proof_required")
    {
        kinds.push("negative-behavior-proof".into());
    }
    kinds
}

fn repair_tasks(surface_type: &str, severity: &str) -> Vec<String> {
    let task = match surface_type {
        "authz_boundary" => {
            "add negative authorization or tenant-isolation proof and attach a proofmark receipt"
        }
        "input_boundary" => {
            "add malformed-input or unsafe-sink negative proof and attach a proofmark receipt"
        }
        "db_migration" => {
            "run migration analysis and document rollback/backfill/lock evidence for destructive SQL"
        }
        "sql_query" => "prove the SQL boundary with migration or adapter evidence",
        "cli_command" | "mcp_tool" => {
            "prove the tool surface with supply-chain review and changed-behavior receipt"
        }
        "rust_public_api" => "prove public API compatibility and changed-line behavior",
        "unsafe_or_process_sink" => {
            "prove the process/filesystem sink with negative input evidence"
        }
        _ => "attach a focused proof receipt for the changed behavior",
    };
    let mut tasks = vec![task.to_string()];
    if matches!(severity, "high" | "critical") {
        tasks.push("do not merge as hard proof until the obligation is satisfied or waived".into());
    }
    tasks
}

pub(crate) fn obligation_for_surface(
    surface: &ChangedSurface,
    receipts: &[crate::receipts::ReceiptEvidence],
) -> ProofObligation {
    let obligation_id = format!(
        "obligation:{}:{}",
        match surface.required_rules.first() {
            Some(rule) => rule.clone(),
            None => "HLT-008-FALSE-GREEN-RISK".into(),
        },
        surface.surface_id
    );
    let mut receipt_paths = Vec::new();
    for receipt in receipts {
        if crate::receipts::receipt_satisfies(&obligation_id, surface, receipt) {
            receipt_paths.push(receipt.path.clone());
        }
    }
    receipt_paths.sort();
    receipt_paths.dedup();
    let satisfied = !receipt_paths.is_empty();
    ProofObligation {
        obligation_id,
        surface_id: surface.surface_id.clone(),
        path: surface.path.clone(),
        symbol: surface.symbol.clone(),
        surface_type: surface.surface_type.clone(),
        severity: surface.severity.clone(),
        risk_tags: surface.risk_tags.clone(),
        rule_ids: surface.required_rules.clone(),
        required_lanes: surface.required_lanes.clone(),
        required_receipt_kinds: required_receipt_kinds(surface),
        repair_task: match surface.repair_tasks.first() {
            Some(task) => task.clone(),
            None => "attach a focused proof receipt for the changed surface".into(),
        },
        satisfied,
        status: if satisfied { "satisfied" } else { "missing" }.into(),
        receipt_paths,
    }
}
