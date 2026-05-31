use crate::audit::helpers::*;
use crate::model::{DimensionResult, ToolAdoptionItem, ToolAdoptionReadiness};
use serde_json::json;

pub fn analyze(ctx: &AuditContext) -> DimensionResult {
    let readiness = status(ctx);
    let mut evidence = vec![if readiness.control_plane_present {
        "control-plane files present".into()
    } else {
        "tool adoption control-plane files are missing".into()
    }];
    let mut notes = vec![];
    let score = if readiness.applicable_count == 0 {
        evidence.push("no applicable tool replacement surface".into());
        90
    } else {
        let applicable = readiness.applicable_count as f64;
        evidence.push(format!("applicable={}", readiness.applicable_count));
        evidence.push(format!("configured={}", readiness.configured_count));
        evidence.push(format!("ci_evidence={}", readiness.ci_evidence_count));
        evidence.push(format!(
            "artifact_verified={}",
            readiness.artifact_verified_count
        ));
        let score = if readiness.control_plane_present {
            10.0
        } else {
            0.0
        } + 20.0 * (readiness.configured_count as f64 / applicable)
            + 60.0 * (readiness.replaced_count as f64 / applicable)
            + 10.0 * (readiness.artifact_verified_count as f64 / applicable);
        score.round() as i32
    };

    if readiness.configured_count < readiness.applicable_count {
        notes.push("some applicable tools are not configured".into());
    }
    if readiness.ci_evidence_count < readiness.applicable_count {
        notes.push("some applicable tools lack CI evidence".into());
    }

    let mut dim = make_dim(
        "Jankurai tool adoption and CI replacement",
        score,
        evidence,
        notes,
    );
    dim.notes.extend(
        readiness
            .missing
            .iter()
            .take(4)
            .map(|tool| format!("missing CI evidence for `{tool}`")),
    );
    dim
}

pub fn status(ctx: &AuditContext) -> ToolAdoptionReadiness {
    let config = load_tool_adoption_config(&ctx.root);
    let workflow_text = tool_adoption_ci_text(ctx);
    let mut items = Vec::new();
    let mut applicable_count = 0usize;
    let mut configured_count = 0usize;
    let mut ci_evidence_count = 0usize;
    let mut artifact_verified_count = 0usize;

    for entry in TOOL_ADOPTION_CATALOG {
        let mode = config.mode_for(entry.id);
        let applicable = tool_adoption_applicable(entry, ctx, mode);
        let ci_command_present =
            applicable && workflow_text.contains(&entry.ci_command.to_ascii_lowercase());
        let upload_present = applicable
            && ci_command_present
            && workflow_text.contains("upload-artifact")
            && entry
                .artifact_paths
                .iter()
                .all(|artifact| workflow_text.contains(&artifact.to_ascii_lowercase()));
        let config_entry_present = applicable && config.present && config.has_entry(entry.id);
        let status = if !applicable {
            "not_applicable"
        } else if ci_command_present && upload_present {
            "artifact_verified"
        } else if ci_command_present {
            "ci_evidence"
        } else if config_entry_present {
            "configured"
        } else {
            "missing"
        };

        if applicable {
            applicable_count += 1;
        }
        if config_entry_present {
            configured_count += 1;
        }
        if matches!(status, "ci_evidence" | "artifact_verified") {
            ci_evidence_count += 1;
        }
        if status == "artifact_verified" {
            artifact_verified_count += 1;
        }

        let mut item_evidence = vec![format!("mode={}", mode.as_str())];
        let mut missing = vec![];
        if config_entry_present {
            item_evidence.push("tool-adoption config entry present".into());
        } else if applicable {
            missing.push("agent/tool-adoption.toml entry".into());
        }
        if ci_command_present {
            item_evidence.push("CI command found in workflow".into());
        } else if applicable {
            missing.push("CI command evidence".into());
        }
        if upload_present {
            item_evidence.push(format!(
                "artifact uploads found: {}",
                entry.artifact_paths.join(", ")
            ));
        } else if ci_command_present {
            missing.push("artifact upload reference".into());
        }

        items.push(ToolAdoptionItem {
            id: entry.id.to_string(),
            category: entry.category.to_string(),
            mode: mode.as_str().to_string(),
            applicable,
            status: status.to_string(),
            replaced_tools: entry
                .replaced_tools
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            evidence: item_evidence,
            missing,
            local_command: Some(entry.local_command.to_string()),
            ci_command: Some(entry.ci_command.to_string()),
            artifact_paths: entry
                .artifact_paths
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        });
    }

    let applicable_tools = applicable_tool_ids(ctx, &config);
    let configured_tools = items
        .iter()
        .filter(|item| item.applicable && config.has_entry(&item.id))
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    let ci_evidence_tools = items
        .iter()
        .filter(|item| matches!(item.status.as_str(), "ci_evidence" | "artifact_verified"))
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    let artifact_verified_tools = items
        .iter()
        .filter(|item| item.status == "artifact_verified")
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();

    ToolAdoptionReadiness {
        control_plane_present: tool_adoption_control_plane_present(ctx),
        applicable_count,
        configured_count,
        ci_evidence_count,
        artifact_verified_count,
        replaced_count: ci_evidence_count,
        items,
        evidence: json!({
            "config_present": config.present,
            "control_plane_present": tool_adoption_control_plane_present(ctx),
            "applicable_count": applicable_count,
            "configured_count": configured_count,
            "ci_evidence_count": ci_evidence_count,
            "artifact_verified_count": artifact_verified_count,
            "applicable_tools": applicable_tools,
            "configured_tools": configured_tools,
            "ci_evidence_tools": ci_evidence_tools,
            "artifact_verified_tools": artifact_verified_tools,
        }),
        missing: TOOL_ADOPTION_CATALOG
            .iter()
            .filter(|entry| {
                let mode = config.mode_for(entry.id);
                tool_adoption_applicable(entry, ctx, mode)
            })
            .filter(|entry| {
                let ci_command_present =
                    workflow_text.contains(&entry.ci_command.to_ascii_lowercase());
                let upload_present = ci_command_present
                    && workflow_text.contains("upload-artifact")
                    && entry
                        .artifact_paths
                        .iter()
                        .all(|artifact| workflow_text.contains(&artifact.to_ascii_lowercase()));
                !(ci_command_present && upload_present)
            })
            .map(|entry| entry.id.to_string())
            .collect(),
    }
}

pub fn missing_required_ci_tools(ctx: &AuditContext) -> Vec<String> {
    let config = load_tool_adoption_config(&ctx.root);
    let workflow_text = tool_adoption_ci_text(ctx);
    let mut missing = Vec::new();

    for entry in TOOL_ADOPTION_CATALOG {
        let mode = config.mode_for(entry.id);
        if mode != ToolAdoptionMode::Required {
            continue;
        }
        if !tool_adoption_applicable(entry, ctx, mode) {
            continue;
        }
        let ci_command_present = workflow_text.contains(&entry.ci_command.to_ascii_lowercase());
        let upload_present = ci_command_present
            && workflow_text.contains("upload-artifact")
            && entry
                .artifact_paths
                .iter()
                .all(|artifact| workflow_text.contains(&artifact.to_ascii_lowercase()));
        if !(ci_command_present && upload_present) {
            missing.push(entry.id.to_string());
        }
    }

    missing
}

fn applicable_tool_ids(ctx: &AuditContext, config: &ToolAdoptionConfig) -> Vec<String> {
    TOOL_ADOPTION_CATALOG
        .iter()
        .filter(|entry| {
            let mode = config.mode_for(entry.id);
            tool_adoption_applicable(entry, ctx, mode)
        })
        .map(|entry| entry.id.to_string())
        .collect()
}

fn tool_adoption_applicable(
    entry: &ToolAdoptionCatalogEntry,
    ctx: &AuditContext,
    mode: ToolAdoptionMode,
) -> bool {
    match mode {
        ToolAdoptionMode::Disabled => false,
        ToolAdoptionMode::Auto => (entry.applicability)(ctx),
        ToolAdoptionMode::Required | ToolAdoptionMode::Advisory => true,
    }
}
