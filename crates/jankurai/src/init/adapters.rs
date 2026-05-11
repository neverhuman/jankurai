use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

pub const GENERATED_MARKER: &str = "jankurai generated adapter";
pub const AGENT_REQUEST_VERSION: &str = "v1";
pub const AGENT_REQUEST_MARKER: &str =
    "<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->";
pub const STARTUP_UPDATE_REQUEST: &str =
    "If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.";

pub const SUPPORTED_IDES: &[&str] = &[
    "cursor",
    "copilot",
    "claude",
    "gemini",
    "antigravity",
    "aider",
];

pub fn canonical_pointer() -> &'static str {
    "Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard."
}

pub fn master_plan_pointer() -> &'static str {
    "agent/MASTER_PLAN.md"
}

pub fn planner_protocol_pointer() -> &'static str {
    "agent/MASTER_PLAN.md#detailed-planner-protocol"
}

pub const ADAPTER_PATHS: &[&str] = &[
    "CLAUDE.md",
    "GEMINI.md",
    ".cursor/rules/jankurai.mdc",
    ".github/copilot-instructions.md",
    ".github/instructions/jankurai.instructions.md",
    ".github/instructions/jankurai-rust.instructions.md",
    ".github/instructions/jankurai-web.instructions.md",
    ".github/instructions/jankurai-python-ai.instructions.md",
    ".agents/agents.md",
    ".agents/skills/jankurai/SKILL.md",
    ".agents/workflows/jankurai-audit.md",
    ".agents/workflows/jankurai-context-pack.md",
    ".agents/workflows/jankurai-kickoff.md",
    ".agents/workflows/jankurai-prove.md",
    ".agents/workflows/jankurai-repair-plan.md",
    ".agents/workflows/jankurai-witness.md",
    ".claude/skills/jankurai/SKILL.md",
];

#[derive(Debug, Clone, Serialize)]
pub struct AdapterAction {
    pub path: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterFailure {
    pub path: String,
    pub problem: String,
}

pub fn is_adapter_path(path: &str) -> bool {
    ADAPTER_PATHS.contains(&path)
}

pub fn is_skill_path(path: &str) -> bool {
    matches!(
        path,
        ".agents/skills/jankurai/SKILL.md" | ".claude/skills/jankurai/SKILL.md"
    )
}

pub fn has_skill_frontmatter(text: &str) -> bool {
    text.starts_with("---\n")
        && text.contains("\nname: jankurai\n")
        && text.contains("\ndescription: Jankurai workspace guidance")
        && text.contains("\n---\n\n# jankurai")
}

pub fn needs_generated_skill_repair(path: &str, text: &str) -> bool {
    is_skill_path(path) && text.contains(GENERATED_MARKER) && !has_skill_frontmatter(text)
}

pub fn has_current_startup_request(text: &str) -> bool {
    text.contains(AGENT_REQUEST_MARKER) && text.contains(STARTUP_UPDATE_REQUEST)
}

pub fn generated_adapter_needs_refresh(existing_text: &str, template_body: &str) -> bool {
    existing_text.contains(GENERATED_MARKER) && existing_text != template_body
}

pub fn adapter_plan(repo: &Path, ide: &str) -> Vec<AdapterAction> {
    selected_adapter_paths(ide)
        .into_iter()
        .map(|path| AdapterAction {
            path: path.into(),
            action: if repo.join(path).exists() {
                "keep-existing".into()
            } else {
                "create".into()
            },
        })
        .collect()
}

pub fn write_adapters(repo: &Path, ide: &str, force_generated: bool) -> Result<Vec<AdapterAction>> {
    let mut actions = Vec::new();
    for template in crate::init::templates::TEMPLATES
        .iter()
        .filter(|template| selected_adapter_paths(ide).contains(&template.path))
    {
        let path = repo.join(template.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        if path.exists() {
            let text = fs::read_to_string(&path).unwrap_or_default();
            if text.contains(GENERATED_MARKER)
                && (force_generated
                    || generated_adapter_needs_refresh(&text, template.body)
                    || needs_generated_skill_repair(template.path, &text))
            {
                fs::write(&path, template.body)
                    .with_context(|| format!("write {}", path.display()))?;
                actions.push(AdapterAction {
                    path: template.path.into(),
                    action: "overwrote-generated".into(),
                });
            } else {
                actions.push(AdapterAction {
                    path: template.path.into(),
                    action: "keep-existing".into(),
                });
            }
            continue;
        }
        fs::write(&path, template.body).with_context(|| format!("write {}", path.display()))?;
        actions.push(AdapterAction {
            path: template.path.into(),
            action: "created".into(),
        });
    }
    Ok(actions)
}

pub fn verify_adapters(repo: &Path) -> Result<Vec<AdapterFailure>> {
    let mut failures = Vec::new();
    for path in ADAPTER_PATHS {
        let full = repo.join(path);
        if !full.exists() {
            continue;
        }
        let text = fs::read_to_string(&full).with_context(|| format!("read {}", full.display()))?;
        let requires_master_plan_routing =
            *path != ".github/instructions/jankurai-python-ai.instructions.md";
        if !text.contains("AGENTS.md")
            || !text.contains("agent/JANKURAI_STANDARD.md")
            || (requires_master_plan_routing && !text.contains(master_plan_pointer()))
            || (requires_master_plan_routing && !text.contains(planner_protocol_pointer()))
            || (requires_master_plan_routing && !text.contains("tips/phases/00-phase-index.md"))
            || (requires_master_plan_routing && !text.contains("tips/phases/logs/"))
            || (requires_master_plan_routing && !text.contains("explicit MASTER_PLAN/phase"))
        {
            failures.push(AdapterFailure {
                path: (*path).into(),
                problem: if requires_master_plan_routing {
                    "adapter lacks canonical AGENTS.md, standard, conditional MASTER_PLAN routing, planner protocol, phase index, and phase log pointers".into()
                } else {
                    "python-ai adapter lacks canonical AGENTS.md or standard pointer".into()
                },
            });
        }
        if !requires_master_plan_routing
            && (!text.contains("Do not create or expand Python")
                || !text.contains("product truth, authorization, repo tools, proof lanes, backend glue, or direct production DB writes"))
        {
            failures.push(AdapterFailure {
                path: (*path).into(),
                problem: "python-ai adapter is missing the Python exception policy".into(),
            });
        }
        if text.contains(GENERATED_MARKER) && !has_current_startup_request(&text) {
            failures.push(AdapterFailure {
                path: (*path).into(),
                problem: "generated adapter is missing the current startup update request marker"
                    .into(),
            });
        }
        let lower = text.to_ascii_lowercase();
        if lower.contains("ignore agents.md")
            || lower.contains("ignore `agents.md`")
            || lower.contains("do not read agents.md")
            || lower.contains("do not use agent/jankurai_standard.md")
            || lower.contains("do not use agent/master_plan.md")
        {
            failures.push(AdapterFailure {
                path: (*path).into(),
                problem: "adapter contains contradictory command/path policy".into(),
            });
        }
        if text.lines().count() > 80 {
            failures.push(AdapterFailure {
                path: (*path).into(),
                problem: "adapter is too long; adapters must not duplicate the full standard"
                    .into(),
            });
        }
        if is_skill_path(path) && !has_skill_frontmatter(&text) {
            failures.push(AdapterFailure {
                path: (*path).into(),
                problem: "skill adapter is missing YAML frontmatter delimited by ---".into(),
            });
        }
    }
    Ok(failures)
}

fn selected_adapter_paths(ide: &str) -> Vec<&'static str> {
    let mut paths = Vec::new();
    for token in ide.split(',').map(str::trim) {
        match token {
            "all" => {
                paths.extend_from_slice(ADAPTER_PATHS);
                break;
            }
            "claude" => {
                paths.push("CLAUDE.md");
                paths.push(".claude/skills/jankurai/SKILL.md");
            }
            "cursor" => paths.push(".cursor/rules/jankurai.mdc"),
            "copilot" => {
                paths.push(".github/copilot-instructions.md");
                paths.push(".github/instructions/jankurai.instructions.md");
                paths.push(".github/instructions/jankurai-rust.instructions.md");
                paths.push(".github/instructions/jankurai-web.instructions.md");
                paths.push(".github/instructions/jankurai-python-ai.instructions.md");
            }
            "gemini" => paths.push("GEMINI.md"),
            "antigravity" => {
                paths.push(".agents/agents.md");
                paths.push(".agents/skills/jankurai/SKILL.md");
                paths.push(".agents/workflows/jankurai-audit.md");
                paths.push(".agents/workflows/jankurai-context-pack.md");
                paths.push(".agents/workflows/jankurai-kickoff.md");
                paths.push(".agents/workflows/jankurai-prove.md");
                paths.push(".agents/workflows/jankurai-repair-plan.md");
                paths.push(".agents/workflows/jankurai-witness.md");
            }
            _ => {}
        }
    }
    paths.sort_unstable();
    paths.dedup();
    paths
}
