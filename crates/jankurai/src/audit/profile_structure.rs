use crate::audit::helpers::{has_prefix, path_matches_prefix, AuditContext};
use crate::model::{ProfileStructureCell, ProfileStructureReadiness};
use serde_json::json;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy)]
struct ProfileCellSpec {
    id: &'static str,
    canonical_path: &'static str,
    aliases: &'static [&'static str],
    owner: &'static str,
    proof_lane: &'static str,
    guidance: &'static str,
    agent_fix: &'static str,
}

const CELL_SPECS: &[ProfileCellSpec] = &[
    ProfileCellSpec {
        id: "web",
        canonical_path: "apps/web/",
        aliases: &["frontend/", "ui/", "packages/web/", "packages/ui/"],
        owner: "apps/web",
        proof_lane: "rendered UX / Playwright",
        guidance: "apps/web/AGENTS.md",
        agent_fix:
            "migrate the detected web surface to `apps/web/` or document an alternate profile with owner, proof lane, expiry, and migration plan",
    },
    ProfileCellSpec {
        id: "api",
        canonical_path: "apps/api/",
        aliases: &["api/", "server/", "backend/"],
        owner: "apps/api",
        proof_lane: "edge handler / contract tests",
        guidance: "apps/api/AGENTS.md",
        agent_fix:
            "migrate the detected API edge to `apps/api/` or document an alternate profile with owner, proof lane, expiry, and migration plan",
    },
    ProfileCellSpec {
        id: "domain",
        canonical_path: "crates/domain/",
        aliases: &["domain/", "core/"],
        owner: "crates/domain",
        proof_lane: "unit / property tests",
        guidance: "crates/domain/AGENTS.md",
        agent_fix:
            "migrate the detected domain layer to `crates/domain/` or document an alternate profile with owner, proof lane, expiry, and migration plan",
    },
    ProfileCellSpec {
        id: "application",
        canonical_path: "crates/application/",
        aliases: &["application/", "usecases/", "use-cases/"],
        owner: "crates/application",
        proof_lane: "use-case / authz tests",
        guidance: "crates/application/AGENTS.md",
        agent_fix:
            "migrate the detected application layer to `crates/application/` or document an alternate profile with owner, proof lane, expiry, and migration plan",
    },
    ProfileCellSpec {
        id: "adapters",
        canonical_path: "crates/adapters/",
        aliases: &["adapters/", "infra/", "integrations/"],
        owner: "crates/adapters",
        proof_lane: "adapter integration tests",
        guidance: "crates/adapters/AGENTS.md",
        agent_fix:
            "migrate the detected adapter layer to `crates/adapters/` or document an alternate profile with owner, proof lane, expiry, and migration plan",
    },
    ProfileCellSpec {
        id: "workers",
        canonical_path: "crates/workers/",
        aliases: &["workers/", "jobs/", "scheduler/", "queue/"],
        owner: "crates/workers",
        proof_lane: "workflow / replay tests",
        guidance: "crates/workers/AGENTS.md",
        agent_fix:
            "migrate the detected worker layer to `crates/workers/` or document an alternate profile with owner, proof lane, expiry, and migration plan",
    },
    ProfileCellSpec {
        id: "contracts",
        canonical_path: "contracts/",
        aliases: &["openapi/", "protobuf/", "json-schema/", "generated/"],
        owner: "contracts",
        proof_lane: "generation / drift checks",
        guidance: "contracts/AGENTS.md",
        agent_fix:
            "migrate the detected contract surface to `contracts/` or document an alternate profile with owner, proof lane, expiry, and migration plan",
    },
    ProfileCellSpec {
        id: "db",
        canonical_path: "db/",
        aliases: &["migrations/", "constraints/", "sql/"],
        owner: "db",
        proof_lane: "migration / constraint tests",
        guidance: "db/AGENTS.md",
        agent_fix:
            "migrate the detected database surface to `db/` or document an alternate profile with owner, proof lane, expiry, and migration plan",
    },
    ProfileCellSpec {
        id: "python-ai",
        canonical_path: "python/ai-service/",
        aliases: &["python/", "ai-service/", "evals/", "embeddings/", "model/"],
        owner: "python/ai-service",
        proof_lane: "eval / contract tests",
        guidance: "python/ai-service/AGENTS.md",
        agent_fix:
            "migrate the detected Python AI/data surface to `python/ai-service/` or document a dated advanced-ML/data exception with owner, proof lane, expiry, and migration plan",
    },
    ProfileCellSpec {
        id: "ops",
        canonical_path: "ops/",
        aliases: &[".github/", ".github/workflows/", "ci/", "release/", "observability/", "security/"],
        owner: "ops",
        proof_lane: "security lane / workflow lint",
        guidance: "ops/AGENTS.md",
        agent_fix:
            "migrate the detected ops surface to `ops/` or document an alternate profile with owner, proof lane, expiry, and migration plan",
    },
];

pub fn analyze(ctx: &AuditContext) -> ProfileStructureReadiness {
    let cells = CELL_SPECS
        .iter()
        .map(|spec| profile_cell(ctx, spec))
        .collect::<Vec<_>>();
    let applicable_count = cells.iter().filter(|cell| cell.applicable).count();
    let canonical_count = cells
        .iter()
        .filter(|cell| cell.status == "canonical")
        .count();
    let noncanonical_count = cells
        .iter()
        .filter(|cell| cell.status == "noncanonical")
        .count();
    let guidance_missing_count = cells
        .iter()
        .filter(|cell| cell.guidance_status == "missing")
        .count();
    let evidence = json!({
        "applicable_count": applicable_count,
        "canonical_count": canonical_count,
        "noncanonical_count": noncanonical_count,
        "guidance_missing_count": guidance_missing_count,
        "detected_cells": cells.iter().filter(|cell| cell.applicable).map(|cell| {
            json!({
                "id": cell.id,
                "status": cell.status,
                "canonical_path": cell.canonical_path,
                "detected_paths": cell.detected_paths,
                "guidance_status": cell.guidance_status,
            })
        }).collect::<Vec<_>>(),
    });
    ProfileStructureReadiness {
        applicable_count,
        canonical_count,
        noncanonical_count,
        guidance_missing_count,
        cells,
        evidence,
    }
}

fn profile_cell(ctx: &AuditContext, spec: &ProfileCellSpec) -> ProfileStructureCell {
    let canonical = canonical_path_present(ctx, spec.canonical_path);
    let mut detected_paths = detect_paths(ctx, spec.canonical_path, spec.aliases);
    if detected_paths.is_empty() && !canonical {
        if let Some(alias) = spec.aliases.iter().find(|alias| has_prefix(ctx, alias)) {
            detected_paths.push(alias.trim_end_matches('/').to_string());
        }
    }
    let applicable = !detected_paths.is_empty();
    let status = if !applicable {
        "not_applicable"
    } else if canonical {
        "canonical"
    } else {
        "noncanonical"
    };
    let guidance_present = canonical_guidance_present(ctx, spec.guidance);
    let guidance_status = if !applicable {
        "not_required"
    } else if guidance_present {
        "present"
    } else {
        "missing"
    };
    let agent_fix = match status {
        "not_applicable" => "no action".to_string(),
        "canonical" if guidance_present => format!(
            "keep `{}` aligned with owns / forbidden / proof lane guidance",
            spec.guidance
        ),
        "canonical" => format!(
            "add `{}` with owns / forbidden / proof lane guidance",
            spec.guidance
        ),
        _ => spec.agent_fix.to_string(),
    };
    ProfileStructureCell {
        id: spec.id.into(),
        applicable,
        status: status.into(),
        canonical_path: spec.canonical_path.into(),
        detected_paths,
        aliases: spec
            .aliases
            .iter()
            .map(|alias| (*alias).to_string())
            .collect(),
        guidance_status: guidance_status.into(),
        owner: spec.owner.into(),
        proof_lane: spec.proof_lane.into(),
        agent_fix,
    }
}

fn canonical_path_present(ctx: &AuditContext, canonical_path: &str) -> bool {
    ctx.all_files
        .iter()
        .any(|file| path_matches_prefix(&file.rel_path, canonical_path))
}

fn canonical_guidance_present(ctx: &AuditContext, guidance_path: &str) -> bool {
    ctx.all_files
        .iter()
        .any(|file| file.rel_path == guidance_path)
}

fn detect_paths(ctx: &AuditContext, canonical_path: &str, aliases: &[&str]) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for file in &ctx.all_files {
        if path_matches_prefix(&file.rel_path, canonical_path) {
            paths.insert(canonical_path.trim_end_matches('/').to_string());
        }
        for alias in aliases {
            if path_matches_prefix(&file.rel_path, alias) {
                paths.insert(alias.trim_end_matches('/').to_string());
            }
        }
    }
    paths.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::helpers::AuditContext;
    use crate::model::{BoundaryReclassification, FileInfo};
    use std::path::PathBuf;

    fn file(rel_path: &str) -> FileInfo {
        FileInfo {
            rel_path: rel_path.into(),
            name: PathBuf::from(rel_path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            suffix: PathBuf::from(rel_path)
                .extension()
                .map(|ext| format!(".{}", ext.to_string_lossy()))
                .unwrap_or_default(),
            size: 0,
            line_count: 1,
            text: String::new(),
            is_generated: false,
            is_code: false,
        }
    }

    fn ctx(paths: &[&str]) -> AuditContext {
        let files = paths.iter().map(|path| file(path)).collect::<Vec<_>>();
        AuditContext {
            root: PathBuf::from("."),
            scope_files: files.clone(),
            all_files: files,
            scope_paths: vec![],
            self_audit: false,
            boundary_reclassifications: vec![BoundaryReclassification {
                id: "test".into(),
                paths: vec![],
                classification: "test".into(),
                product_surface: false,
                runtime_language: "rust".into(),
                status: "ok".into(),
                reclassified_caps: vec![],
                covered_file_count: 0,
                covered_line_count: 0,
                covered_files: vec![],
                evidence_artifacts: vec![],
                missing_checks: vec![],
                failed_checks: vec![],
                rerun_command: "test".into(),
                suppresses_python_stack_caps: false,
            }],
            copy_code: None,
        }
    }

    #[test]
    fn detects_alias_paths_as_noncanonical_cells() {
        let report = analyze(&ctx(&["frontend/App.tsx"]));
        let web = report.cells.iter().find(|cell| cell.id == "web").unwrap();

        assert_eq!(web.status, "noncanonical");
        assert_eq!(web.canonical_path, "apps/web/");
        assert!(web.detected_paths.iter().any(|path| path == "frontend"));
        assert_eq!(web.guidance_status, "missing");
    }

    #[test]
    fn detects_canonical_paths_and_guidance() {
        let report = analyze(&ctx(&["apps/web/AGENTS.md", "apps/web/src/main.tsx"]));
        let web = report.cells.iter().find(|cell| cell.id == "web").unwrap();

        assert_eq!(web.status, "canonical");
        assert_eq!(web.guidance_status, "present");
        assert!(web.detected_paths.iter().any(|path| path == "apps/web"));
    }

    #[test]
    fn leaves_unrelated_cells_not_applicable() {
        let report = analyze(&ctx(&["README.md"]));

        for cell_id in ["web", "api", "db", "python-ai", "ops"] {
            let cell = report.cells.iter().find(|cell| cell.id == cell_id).unwrap();
            assert_eq!(
                cell.status, "not_applicable",
                "{cell_id} should stay inactive"
            );
        }
        assert_eq!(report.applicable_count, 0);
        assert_eq!(report.canonical_count, 0);
        assert_eq!(report.noncanonical_count, 0);
    }
}
