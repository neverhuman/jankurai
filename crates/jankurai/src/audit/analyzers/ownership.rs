use crate::audit::helpers::*;
use crate::model::DimensionResult;
use crate::model::ProfileStructureReadiness;

pub fn analyze(
    ctx: &AuditContext,
    profile_structure: &ProfileStructureReadiness,
) -> DimensionResult {
    let mut score = 35;
    let mut evidence = vec![];
    let mut notes = vec![];
    if has_root_agents(ctx) {
        score += 18;
        evidence.push("root `AGENTS.md` present".into());
    }
    if ctx.all_files.iter().any(|f| f.name == "CODEOWNERS") {
        score += 10;
        evidence.push("`CODEOWNERS` present".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.rel_path == "agent/owner-map.json")
    {
        score += 10;
        evidence.push("owner map present".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.rel_path == "agent/test-map.json" || f.rel_path == "agent/proof-lanes.toml")
    {
        score += 8;
        evidence.push("test/proof routing map present".into());
    }
    if ctx
        .all_files
        .iter()
        .any(|f| f.name == "AGENTS.md" && f.rel_path != "AGENTS.md")
    {
        score += 4;
        evidence.push("local `AGENTS.md` file(s)".into());
    }
    for cell in profile_structure
        .cells
        .iter()
        .filter(|cell| cell.applicable)
    {
        match cell.status.as_str() {
            "canonical" => {
                score += 4;
                evidence.push(format!("reference-profile cell `{}` is canonical", cell.id));
            }
            "noncanonical" => {
                score -= 6;
                notes.push(format!(
                    "reference-profile cell `{}` is detected outside its canonical folder",
                    cell.id
                ));
            }
            _ => {}
        }
        match cell.guidance_status.as_str() {
            "present" => {
                score += 2;
                evidence.push(format!(
                    "reference-profile cell `{}` has local AGENTS.md guidance",
                    cell.id
                ));
            }
            "missing" => {
                score -= 2;
                notes.push(format!(
                    "reference-profile cell `{}` lacks local AGENTS.md guidance",
                    cell.id
                ));
            }
            _ => {}
        }
    }
    if root_readme_routes(ctx) {
        score += 6;
        evidence.push("root `README.md` routes to workspace layout".into());
    }
    if crate::audit::helpers::missing_owner_paths(ctx).is_empty() {
        score += 8;
        evidence.push("owner map covers audited paths".into());
    }
    if crate::audit::helpers::missing_test_paths(ctx).is_empty() {
        score += 7;
        evidence.push("test map covers audited paths".into());
    }
    let repo_rot = crate::audit::repo_rot::summary(ctx);
    if repo_rot.hard_findings > 0 {
        score -= 8;
        notes.push(format!(
            "repo-rot hard findings in active source paths: {}",
            repo_rot.hard_findings
        ));
    } else if repo_rot.advisory_signals > 0 {
        score -= 3;
        notes.push(format!(
            "repo-rot advisory signals in active source: {}",
            repo_rot.advisory_signals
        ));
    }
    if let Some(max) = max_loc(&product_code_files(ctx)) {
        if max > 500 {
            score -= 8;
            evidence.push("authored code file exceeds 500 LOC".into());
        }
        if max > 1000 {
            score -= 8;
            evidence.push("authored code file exceeds 1000 LOC".into());
        }
    }
    make_dim("Ownership and navigation surface", score, evidence, notes)
}
