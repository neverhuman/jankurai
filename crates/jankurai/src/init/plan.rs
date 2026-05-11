use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct InitPlan {
    pub profile: String,
    pub level: String,
    pub profile_manifest: super::profiles::ProfileManifest,
    pub ide: String,
    pub mode: String,
    pub ci: String,
    pub issue_backend: String,
    pub ux_qa: bool,
    pub package_manager: String,
    pub detected: Vec<String>,
    pub actions: Vec<PlannedAction>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedAction {
    pub path: String,
    pub action: String,
}

// Init plan construction carries CLI-selected profile, level, IDE, CI, and UX knobs.
#[allow(clippy::too_many_arguments)]
pub fn build_plan(
    repo: &Path,
    profile: &str,
    profile_file: Option<&Path>,
    level: &str,
    ide: &str,
    mode: &str,
    ci: &str,
    issue_backend: &str,
    ux_qa: bool,
) -> Result<InitPlan> {
    let existing = super::detect::existing_standard_files(repo);
    let detected = super::detect::detect_surfaces(repo);
    let selected_level = InitLevel::parse(level)?;
    let profile_manifest = match profile_file {
        Some(path) => super::profiles::load_profile_from_path(repo, path)?,
        None => super::profiles::resolve_profile(repo, profile)?,
    };
    let profile_manifest = augment_for_repo(
        repo,
        filter_profile_manifest(profile_manifest, selected_level),
        selected_level,
    );
    let profile = profile_manifest.id.clone();
    let mut paths = profile_manifest.generated_paths.clone();
    paths.sort();
    let mut actions = Vec::new();
    for path in &paths {
        if super::templates::template_for_path(path).is_none() {
            bail!("profile declares `{path}` but no init template is registered (see init/templates.rs)");
        }
        let action = if repo.join(path).exists() {
            profile_manifest
                .merge_policy_for_path(path)
                .plan_action()
                .into()
        } else {
            "create".into()
        };
        actions.push(PlannedAction {
            path: path.clone(),
            action,
        });
    }
    let mut warnings = Vec::new();
    if !existing.is_empty() {
        warnings.push(format!(
            "existing files left intact: {}",
            existing.join(", ")
        ));
    }
    Ok(InitPlan {
        profile,
        level: selected_level.as_str().into(),
        profile_manifest,
        ide: ide.into(),
        mode: mode.into(),
        ci: ci.into(),
        issue_backend: issue_backend.into(),
        ux_qa,
        package_manager: super::package_manager::detect_package_manager(repo).to_string(),
        detected,
        actions,
        warnings,
    })
}

pub fn render_plan(plan: &InitPlan) -> String {
    let mut out = String::new();
    use std::fmt::Write;
    let color = crate::ui::stdout_color_enabled();
    let _ = writeln!(
        out,
        "{}",
        crate::ui::paint(crate::ui::Style::Heading, "Jankurai Init Plan", color)
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "- profile: `{}`", plan.profile);
    let _ = writeln!(out, "- level: `{}`", plan.level);
    let _ = writeln!(out, "- profile id: `{}`", plan.profile_manifest.id);
    let _ = writeln!(
        out,
        "- profile display: `{}`",
        plan.profile_manifest.display_name
    );
    let _ = writeln!(
        out,
        "- target stack: `{}`",
        plan.profile_manifest.target_stack_id
    );
    let _ = writeln!(out, "- ide: `{}`", plan.ide);
    let _ = writeln!(out, "- mode: `{}`", plan.mode);
    let _ = writeln!(out, "- ci: `{}`", plan.ci);
    let _ = writeln!(out, "- issue backend: `{}`", plan.issue_backend);
    let _ = writeln!(out, "- ux qa: `{}`", plan.ux_qa);
    let _ = writeln!(out, "- package manager: `{}`", plan.package_manager);
    if !plan.profile_manifest.required_lanes.is_empty() {
        let _ = writeln!(
            out,
            "- required lanes: `{}`",
            plan.profile_manifest.required_lanes.join(", ")
        );
    }
    if !plan.profile_manifest.optional_lanes.is_empty() {
        let _ = writeln!(
            out,
            "- optional lanes: `{}`",
            plan.profile_manifest.optional_lanes.join(", ")
        );
    }
    if !plan.detected.is_empty() {
        let _ = writeln!(out, "- detected: `{}`", plan.detected.join(", "));
    }
    if !plan.profile_manifest.validation_commands.is_empty() {
        let _ = writeln!(
            out,
            "- validation: `{}`",
            plan.profile_manifest.validation_commands.join(" | ")
        );
    }
    for warning in &plan.warnings {
        let _ = writeln!(out, "- warning: `{warning}`");
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{}",
        crate::ui::paint(crate::ui::Style::Heading, "Planned file actions:", color)
    );
    let created = plan
        .actions
        .iter()
        .filter(|action| action.action == "create")
        .count();
    let merged = plan
        .actions
        .iter()
        .filter(|action| action.action.starts_with("merge"))
        .count();
    let kept = plan
        .actions
        .iter()
        .filter(|action| action.action == "keep-existing")
        .count();
    let _ = writeln!(
        out,
        "  {} create  {} merge  {} keep-existing",
        crate::ui::paint(crate::ui::Style::Create, created.to_string(), color),
        crate::ui::paint(crate::ui::Style::Merge, merged.to_string(), color),
        crate::ui::paint(crate::ui::Style::Keep, kept.to_string(), color)
    );
    let _ = writeln!(
        out,
        "  {}",
        crate::ui::paint(
            crate::ui::Style::Muted,
            "merge-* preserves existing content; keep-existing never overwrites user files",
            color
        )
    );
    for action in &plan.actions {
        let style = match action.action.as_str() {
            "create" => crate::ui::Style::Create,
            "keep-existing" => crate::ui::Style::Keep,
            _ => crate::ui::Style::Merge,
        };
        let _ = writeln!(
            out,
            "  {} {}",
            crate::ui::paint(style, &action.action, color),
            action.path
        );
    }
    out
}

pub fn render_next_steps(
    plan: &InitPlan,
    applied: bool,
    receipt: Option<&Path>,
    repo: &Path,
) -> String {
    let mut out = String::new();
    use std::fmt::Write;
    let color = crate::ui::stdout_color_enabled();
    let title = if applied {
        "Installed. Next 3 steps:"
    } else {
        "Preview complete. Next 3 steps:"
    };
    let _ = writeln!(
        out,
        "{}",
        crate::ui::paint(crate::ui::Style::Heading, title, color)
    );

    let repo_arg = shell_arg(repo);
    let repo_label = if repo_arg == "." {
        "this repo root".to_string()
    } else {
        format!("`{repo_arg}`")
    };
    let doctor = crate::ui::paint(
        crate::ui::Style::Accent,
        format!("jankurai doctor {repo_arg} --fail-on high"),
        color,
    );
    let json_out = if repo_arg == "." {
        "target/jankurai/repo-score.json".to_string()
    } else {
        shell_arg(&repo.join("target/jankurai/repo-score.json"))
    };
    let md_out = if repo_arg == "." {
        "target/jankurai/repo-score.md".to_string()
    } else {
        shell_arg(&repo.join("target/jankurai/repo-score.md"))
    };
    let audit = crate::ui::paint(
        crate::ui::Style::Accent,
        format!("jankurai audit {repo_arg} --mode advisory --json {json_out} --md {md_out}"),
        color,
    );
    let agent_prompt = crate::ui::paint(
        crate::ui::Style::Accent,
        "Read AGENTS.md, follow the jankurai standard, then run the proof lane for my change.",
        color,
    );

    if !applied {
        let _ = writeln!(
            out,
            "  1. Review the planned actions above, then apply with `{}`.",
            crate::ui::paint(
                crate::ui::Style::Accent,
                format!(
                    "jankurai init {repo_arg} --profile {} --level {} --yes",
                    plan.profile, plan.level
                ),
                color
            )
        );
        let _ = writeln!(
            out,
            "  2. After applying, run `{doctor}` for local health, then `{audit}` for a score."
        );
        let _ = writeln!(
            out,
            "  3. Start Codex, OpenCode, Claude, Cursor, or another agent from {repo_label} and say: `{agent_prompt}`"
        );
    } else {
        let _ = writeln!(out, "  1. Run `{doctor}` for local health.");
        let _ = writeln!(
            out,
            "  2. Run `{audit}` for the repo score and repair queue."
        );
        let _ = writeln!(
            out,
            "  3. Start Codex, OpenCode, Claude, Cursor, or another agent from {repo_label} and say: `{agent_prompt}`"
        );
    }

    if let Some(path) = receipt {
        let _ = writeln!(
            out,
            "- receipt: `{}`",
            crate::ui::paint(crate::ui::Style::Muted, path.display().to_string(), color)
        );
    }
    let _ = writeln!(
        out,
        "- agent entrypoint: `{}`",
        crate::ui::paint(crate::ui::Style::Good, "AGENTS.md", color)
    );
    out
}

fn shell_arg(path: &Path) -> String {
    let text = path.as_os_str().to_string_lossy();
    if text.is_empty() {
        return ".".to_string();
    }
    if text.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '.' | '/' | '_' | '-' | ':' | '@' | '+')
    }) {
        return text.into_owned();
    }
    format!("'{}'", text.replace('\'', "'\\''"))
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum InitLevel {
    Agents,
    Score,
    Ci,
    Full,
}

impl InitLevel {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "agents" => Ok(Self::Agents),
            "score" => Ok(Self::Score),
            "ci" => Ok(Self::Ci),
            "full" => Ok(Self::Full),
            other => bail!("unknown init level `{other}`; expected agents, score, ci, or full"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Agents => "agents",
            Self::Score => "score",
            Self::Ci => "ci",
            Self::Full => "full",
        }
    }
}

fn filter_profile_manifest(
    mut manifest: super::profiles::ProfileManifest,
    level: InitLevel,
) -> super::profiles::ProfileManifest {
    if level == InitLevel::Full {
        return manifest;
    }

    let allowed = level_allowed_paths(level);
    manifest
        .generated_paths
        .retain(|path| allowed.contains(path.as_str()));
    if matches!(level, InitLevel::Score | InitLevel::Ci)
        && !manifest
            .generated_paths
            .iter()
            .any(|path| path == "Justfile")
    {
        manifest.generated_paths.push("Justfile".into());
        manifest.merge_policy.insert(
            "Justfile".into(),
            super::profiles::MergePolicyAction::MergeLines,
        );
    }
    manifest.agent_adapters = filter_list(manifest.agent_adapters, &allowed);
    manifest.ci_templates = filter_list(manifest.ci_templates, &allowed);
    manifest.docs = filter_list(manifest.docs, &allowed);
    manifest.security_controls = filter_list(manifest.security_controls, &allowed);
    manifest.ux_controls = filter_list(manifest.ux_controls, &allowed);
    manifest.contract_system = filter_list(manifest.contract_system, &allowed);
    manifest.db_policy = filter_list(manifest.db_policy, &allowed);
    manifest.required_lanes = level_required_lanes(level);
    manifest.optional_lanes = level_optional_lanes(level);
    manifest.validation_commands = level_validation_commands(level);

    let generated: BTreeSet<&str> = manifest
        .generated_paths
        .iter()
        .map(String::as_str)
        .collect();
    manifest.merge_policy = manifest
        .merge_policy
        .into_iter()
        .filter(|(path, _)| generated.contains(path.as_str()))
        .collect::<BTreeMap<_, _>>();
    manifest
}

fn augment_for_repo(
    repo: &Path,
    mut manifest: super::profiles::ProfileManifest,
    level: InitLevel,
) -> super::profiles::ProfileManifest {
    if level != InitLevel::Full || !repo.join("Cargo.toml").exists() {
        return manifest;
    }

    for path in ["Justfile", "tools/jankurai-rust/witness.sh"] {
        if !manifest
            .generated_paths
            .iter()
            .any(|existing| existing == path)
        {
            manifest.generated_paths.push(path.into());
        }
    }
    manifest.generated_paths.sort();
    manifest.merge_policy.insert(
        "Justfile".into(),
        super::profiles::MergePolicyAction::MergeLines,
    );
    manifest.merge_policy.insert(
        "tools/jankurai-rust/witness.sh".into(),
        super::profiles::MergePolicyAction::KeepExisting,
    );
    for command in [
        "jankurai rust map .",
        "jankurai rust witness build .",
        "jankurai rust diagnose .",
    ] {
        if !manifest
            .validation_commands
            .iter()
            .any(|existing| existing == command)
        {
            manifest.validation_commands.push(command.into());
        }
    }
    manifest
}

fn filter_list(values: Vec<String>, allowed: &BTreeSet<&'static str>) -> Vec<String> {
    values
        .into_iter()
        .filter(|path| allowed.contains(path.as_str()))
        .collect()
}

fn level_allowed_paths(level: InitLevel) -> BTreeSet<&'static str> {
    let mut paths = BTreeSet::from([
        "AGENTS.md",
        "CLAUDE.md",
        "GEMINI.md",
        ".agents/agents.md",
        ".agents/skills/jankurai/SKILL.md",
        ".agents/workflows/jankurai-audit.md",
        ".agents/workflows/jankurai-context-pack.md",
        ".agents/workflows/jankurai-kickoff.md",
        ".agents/workflows/jankurai-prove.md",
        ".agents/workflows/jankurai-witness.md",
        ".agents/workflows/jankurai-repair-plan.md",
        ".claude/skills/jankurai/SKILL.md",
        ".cursor/rules/jankurai.mdc",
        ".github/copilot-instructions.md",
        ".github/instructions/jankurai.instructions.md",
        ".github/instructions/jankurai-python-ai.instructions.md",
        ".github/instructions/jankurai-rust.instructions.md",
        ".github/instructions/jankurai-web.instructions.md",
        "agent/JANKURAI_STANDARD.md",
        "agent/MASTER_PLAN.md",
    ]);

    if matches!(level, InitLevel::Score | InitLevel::Ci) {
        paths.extend([
            "apps/api/AGENTS.md",
            "apps/web/AGENTS.md",
            "Justfile",
            "contracts/AGENTS.md",
            "crates/adapters/AGENTS.md",
            "crates/application/AGENTS.md",
            "crates/domain/AGENTS.md",
            "crates/workers/AGENTS.md",
            "agent/audit-policy.toml",
            "agent/generated-zones.toml",
            "agent/jankurai-install.toml",
            "agent/owner-map.json",
            "agent/proof-lanes.toml",
            "agent/tool-adoption.toml",
            "agent/standard-version.toml",
            "agent/test-map.json",
            "db/AGENTS.md",
            "ops/AGENTS.md",
            "python/ai-service/AGENTS.md",
        ]);
    }

    if level == InitLevel::Ci {
        paths.extend([
            ".github/workflows/jankurai.yml",
            "agent/security-policy.toml",
            "tools/security-lane.sh",
        ]);
    }

    paths
}

fn level_required_lanes(level: InitLevel) -> Vec<String> {
    match level {
        InitLevel::Agents => vec![],
        InitLevel::Score => vec!["audit".into(), "doctor".into()],
        InitLevel::Ci => vec!["audit".into(), "doctor".into(), "security".into()],
        InitLevel::Full => unreachable!("full keeps the profile manifest unchanged"),
    }
}

fn level_optional_lanes(level: InitLevel) -> Vec<String> {
    match level {
        InitLevel::Agents => vec![],
        InitLevel::Score => vec![],
        InitLevel::Ci => vec!["ratchet".into()],
        InitLevel::Full => unreachable!("full keeps the profile manifest unchanged"),
    }
}

fn level_validation_commands(level: InitLevel) -> Vec<String> {
    match level {
        InitLevel::Agents => vec!["jankurai adapters verify".into()],
        InitLevel::Score => vec![
            "jankurai doctor --fail-on critical".into(),
            "jankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md"
                .into(),
        ],
        InitLevel::Ci => vec![
            "jankurai doctor --fail-on critical".into(),
            "jankurai audit . --mode advisory --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md".into(),
            "jankurai ci install . --github --mode ratchet --baseline target/jankurai/baseline-score.json".into(),
        ],
        InitLevel::Full => unreachable!("full keeps the profile manifest unchanged"),
    }
}
