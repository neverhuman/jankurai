use super::inventory::{detect_stack, has_lockfile, ContractEvidence, StackInventory};
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct LiabilityBreakdown {
    pub total: u32,
    pub dimensions: Vec<LiabilityDimension>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiabilityDimension {
    pub name: String,
    pub score: u32,
    pub weight: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationReport {
    pub schema_version: String,
    pub command: String,
    pub status: String,
    pub generated_at: String,
    pub source_root: String,
    pub source_stack: String,
    pub target_stack: String,
    pub liability_score: u32,
    pub liability_breakdown: LiabilityBreakdown,
    pub inventory: StackInventory,
    pub module_inventory: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_guesses: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_boundaries: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_surfaces: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_surfaces: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_logic: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_risk_areas: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_tests: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strangler_candidates: Option<Vec<String>>,
    pub contract_evidence: Vec<ContractEvidence>,
    pub recommended_slice_order: Vec<String>,
    pub required_proof_lanes: Vec<String>,
    pub rollback_cutover_notes: Vec<String>,
}

pub fn compute_liability(repo: &Path, inv: &StackInventory) -> LiabilityBreakdown {
    let mut dims = vec![];

    let has_agents = repo.join("AGENTS.md").exists() || repo.join("agent").exists();
    let ao_score = if has_agents { 20 } else { 60 };
    dims.push(LiabilityDimension {
        name: "agent-operability".to_string(),
        score: ao_score,
        weight: 0.10,
        evidence: if has_agents {
            vec!["agent configuration detected".into()]
        } else {
            vec!["no agent configuration".into()]
        },
    });

    let has_contracts = !inv.contract_evidence.is_empty();
    let cd_score = if has_contracts { 25 } else { 65 };
    dims.push(LiabilityDimension {
        name: "contract-drift".to_string(),
        score: cd_score,
        weight: 0.12,
        evidence: if has_contracts {
            vec![format!(
                "{} contract artifacts detected",
                inv.contract_evidence.len()
            )]
        } else {
            vec!["no contract artifacts detected".into()]
        },
    });

    let multi_lang = inv.languages.len() > 2;
    let pts_score = if multi_lang {
        70
    } else if inv.languages.len() > 1 {
        45
    } else {
        25
    };
    dims.push(LiabilityDimension {
        name: "product-truth-sprawl".to_string(),
        score: pts_score,
        weight: 0.12,
        evidence: vec![format!("{} languages detected", inv.languages.len())],
    });

    let no_lock = !has_lockfile(repo);
    let sr_score = if no_lock { 70 } else { 30 };
    dims.push(LiabilityDimension {
        name: "security-risk".to_string(),
        score: sr_score,
        weight: 0.15,
        evidence: if no_lock {
            vec!["no lockfile detected".into()]
        } else {
            vec!["lockfile present".into()]
        },
    });

    let db_count = inv.db_clients.len();
    let ddr_score = if db_count > 1 {
        65
    } else if db_count == 1 {
        35
    } else {
        15
    };
    dims.push(LiabilityDimension {
        name: "db-data-risk".to_string(),
        score: ddr_score,
        weight: 0.13,
        evidence: vec![format!("{} DB client(s) detected", db_count)],
    });

    let has_tests = !inv.test_frameworks.is_empty();
    let has_ci = !inv.ci_systems.is_empty();
    let tpg_score = match (has_tests, has_ci) {
        (true, true) => 15,
        (true, false) => 40,
        (false, true) => 50,
        (false, false) => 80,
    };
    dims.push(LiabilityDimension {
        name: "test-proof-gaps".to_string(),
        score: tpg_score,
        weight: 0.15,
        evidence: vec![format!("tests={has_tests}, ci={has_ci}")],
    });

    let is_rust = inv.languages.iter().any(|l| l.name == "rust");
    let rcr_score = if is_rust {
        15
    } else if inv.languages.iter().any(|l| l.name == "go") {
        25
    } else {
        50
    };
    dims.push(LiabilityDimension {
        name: "runtime-cost-risk".to_string(),
        score: rcr_score,
        weight: 0.10,
        evidence: vec![format!(
            "primary language: {}",
            inv.languages
                .first()
                .map(|l| l.name.as_str())
                .unwrap_or("unknown")
        )],
    });

    let fw_count = inv.frameworks.len();
    let mc_score = if fw_count > 2 {
        70
    } else if fw_count > 0 {
        40
    } else {
        55
    };
    dims.push(LiabilityDimension {
        name: "migration-complexity".to_string(),
        score: mc_score,
        weight: 0.13,
        evidence: vec![format!("{} framework(s) detected", fw_count)],
    });

    let total: f64 = dims.iter().map(|d| d.score as f64 * d.weight).sum();
    LiabilityBreakdown {
        total: total.round().clamp(0.0, 100.0) as u32,
        dimensions: dims,
    }
}

pub fn build_migration_report(repo: &Path, target: &str) -> Result<MigrationReport> {
    let inv = detect_stack(repo);
    let liability = compute_liability(repo, &inv);

    let mut module_inventory = vec![];
    for item in &inv.languages {
        module_inventory.push(format!("language:{}", item.name));
    }
    for item in &inv.frameworks {
        module_inventory.push(format!("framework:{}", item.name));
    }
    for item in &inv.package_managers {
        module_inventory.push(format!("package-manager:{}", item.name));
    }

    let db_surfaces = if inv.db_clients.is_empty() {
        None
    } else {
        Some(
            inv.db_clients
                .iter()
                .map(|c| format!("db-client:{}", c.name))
                .collect(),
        )
    };

    let api_surfaces = if inv.api_surfaces.is_empty() {
        None
    } else {
        Some(
            inv.api_surfaces
                .iter()
                .map(|a| format!("api-framework:{}", a.framework))
                .collect(),
        )
    };

    let strangler_candidates = if inv.db_clients.is_empty() && inv.api_surfaces.is_empty() {
        None
    } else {
        let mut candidates = vec![];
        for db in &inv.db_clients {
            candidates.push(format!("isolate-db-layer:{}", db.name));
        }
        for api in &inv.api_surfaces {
            candidates.push(format!("isolate-api-surface:{}", api.framework));
        }
        Some(candidates)
    };

    let mut recommended_slice_order = vec![
        "inventory-and-classify".to_string(),
        "extract-contracts".to_string(),
    ];
    if !inv.db_clients.is_empty() {
        recommended_slice_order.push("isolate-db-adapter-layer".to_string());
    }
    recommended_slice_order.push("port-business-logic".to_string());
    recommended_slice_order.push("prove-equivalence".to_string());
    recommended_slice_order.push("cutover-and-retire".to_string());

    let required_proof_lanes = vec!["fast".to_string(), "contract".to_string()];

    let missing_tests = if inv.test_frameworks.is_empty() {
        Some(vec![
            "no test framework detected — migration risk is elevated".to_string(),
        ])
    } else {
        None
    };

    let high_risk_areas = if inv.ci_systems.is_empty() {
        Some(vec![
            "no CI system detected — migration cannot be verified automatically".to_string(),
        ])
    } else {
        None
    };

    let source_stack = {
        let langs: Vec<&str> = inv.languages.iter().map(|l| l.name.as_str()).collect();
        let fws: Vec<&str> = inv.frameworks.iter().map(|f| f.name.as_str()).collect();
        let mut parts = vec![];
        if !langs.is_empty() {
            parts.push(langs.join("+"));
        }
        if !fws.is_empty() {
            parts.push(fws.join("+"));
        }
        if parts.is_empty() {
            "unknown".to_string()
        } else {
            parts.join("/")
        }
    };

    Ok(MigrationReport {
        schema_version: "1.0.0".to_string(),
        command: "jankurai migrate".to_string(),
        status: "complete".to_string(),
        generated_at: super::now_string(),
        source_root: repo.display().to_string(),
        source_stack,
        target_stack: target.to_string(),
        liability_score: liability.total,
        liability_breakdown: liability,
        inventory: inv.clone(),
        module_inventory,
        owner_guesses: None,
        external_boundaries: None,
        db_surfaces,
        api_surfaces,
        duplicate_logic: None,
        high_risk_areas,
        missing_tests,
        strangler_candidates,
        contract_evidence: inv.contract_evidence,
        recommended_slice_order,
        required_proof_lanes,
        rollback_cutover_notes: vec![
            "migration planning is derived from repo inventory and audit evidence".to_string(),
            "each slice should have explicit rollback before cutover".to_string(),
        ],
    })
}

pub(crate) fn render_report_markdown(report: &MigrationReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Migration Report");
    let _ = writeln!(out);
    let _ = writeln!(out, "- source stack: `{}`", report.source_stack);
    let _ = writeln!(out, "- target stack: `{}`", report.target_stack);
    let _ = writeln!(out, "- liability score: `{}`", report.liability_score);
    let _ = writeln!(
        out,
        "- module inventory: `{}`",
        report.module_inventory.join(", ")
    );
    if let Some(ref db) = report.db_surfaces {
        let _ = writeln!(out, "- DB surfaces: `{}`", db.join(", "));
    }
    if let Some(ref api) = report.api_surfaces {
        let _ = writeln!(out, "- API surfaces: `{}`", api.join(", "));
    }
    if let Some(ref strangler) = report.strangler_candidates {
        let _ = writeln!(out, "- strangler candidates: `{}`", strangler.join(", "));
    }
    let _ = writeln!(
        out,
        "- recommended slice order: `{}`",
        report.recommended_slice_order.join(" → ")
    );
    let _ = writeln!(
        out,
        "- required proof lanes: `{}`",
        report.required_proof_lanes.join(", ")
    );
    let _ = writeln!(
        out,
        "- rollback notes: `{}`",
        report.rollback_cutover_notes.join("; ")
    );
    out
}
