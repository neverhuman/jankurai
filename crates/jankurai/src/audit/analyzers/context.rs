use crate::audit::helpers::*;
use crate::model::DimensionResult;

pub fn analyze(ctx: &AuditContext) -> DimensionResult {
    let mut score = 20;
    let mut evidence = vec![];
    let mut notes = vec![];
    if has_root_agents(ctx) {
        score += 25;
        evidence.push("root `AGENTS.md` present".into());
        score += 10;
        evidence.push("root `AGENTS.md` stays short".into());
    } else {
        notes.push("no root `AGENTS.md`".into());
    }
    if ctx.all_files.iter().any(|f| {
        [
            "agent-map.json",
            "test-map.json",
            "proof-lanes.toml",
            "generated-zones.toml",
            "agent/owner-map.json",
        ]
        .contains(&f.rel_path.as_str())
    }) {
        score += 10;
        evidence.push("machine-readable routing artifacts present".into());
    }
    if ctx.all_files.iter().any(|f| {
        f.rel_path == "CLAUDE.md"
            || f.rel_path == "GEMINI.md"
            || f.rel_path == ".github/copilot-instructions.md"
            || f.rel_path.starts_with(".cursor/rules/")
            || f.rel_path.starts_with(".github/instructions/")
            || f.rel_path.starts_with(".agents/")
    }) {
        score += 8;
        evidence.push("thin IDE/agent adapters are present".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.name == "AGENTS.md" && f.rel_path != "AGENTS.md")
    {
        score += 10;
        evidence.push("local instruction files present".into());
    }
    if root_readme_routes(ctx) {
        score += 10;
        evidence.push("root README routes to the right docs".into());
    }
    let missing = missing_core_docs(ctx);
    if !missing.is_empty() {
        score -= 6 * missing.len() as i32;
        notes.push(format!(
            "missing agent-readable docs: {}",
            missing
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else {
        score += 8;
        evidence.push("core agent-readable docs present".into());
    }
    if !has_root_agents(ctx)
        && !ctx
            .all_files
            .iter()
            .any(|f| f.name == "AGENTS.md" && f.rel_path != "AGENTS.md")
    {
        score -= 10;
        notes.push("no instruction files to route agents".into());
    }
    make_dim(
        "Context economy and agent instructions",
        score,
        evidence,
        notes,
    )
}
