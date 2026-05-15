use crate::init::profiles::{MergePolicyAction, ProfileManifest};
use crate::init::{self, adapters, merge};
use crate::model::{
    AUDITOR_VERSION, PAPER_EDITION, SCHEMA_VERSION, STANDARD_VERSION, TARGET_STACK_ID,
};
use crate::validation::{self, ArtifactSchema};
use anyhow::{bail, Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const UPDATE_SCHEMA_VERSION: &str = "1.1.0";
const STATE_SCHEMA_VERSION: &str = "1.0.0";
const INSTALL_MANIFEST_SCHEMA_VERSION: &str = "1.0.0";
const CLIENT_START_TTL_SECS: u64 = 60 * 30;
const AUDIT_NETWORK_TIMEOUT_MS: u64 = 400;
const DEFAULT_INSTALL_SOURCE: &str = "github";
const DEFAULT_SOURCE_URL: &str = "https://github.com/jeppsontaylor/Jankurai.git";
const DEFAULT_UPDATE_CHANNEL: &str = "stable";
const MANUAL_UPGRADE_COMMAND: &str = "jankurai upgrade";
const NO_UPDATE_CHECK_ENV: &str = "JANKURAI_NO_UPDATE_CHECK";
const UPGRADE_REEXEC_ENV: &str = "JANKURAI_UPGRADE_REEXECED";
const TEST_LATEST_VERSION_ENV: &str = "JANKURAI_TEST_LATEST_VERSION";

static AUDIT_UPGRADE_NOTICE_EMITTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
pub struct UpdateArgs {
    pub repo: PathBuf,
    pub check: bool,
    pub apply: bool,
    pub yes: bool,
    pub self_update: bool,
    pub skip_self: bool,
    pub client_start: bool,
    pub quiet: bool,
    pub channel: String,
    pub source: String,
    pub offline: bool,
    pub fail_if_outdated: bool,
    pub install_missing: bool,
    pub profile: String,
    pub level: String,
    pub ide: String,
    pub score: bool,
    pub score_mode: String,
    pub score_json: String,
    pub score_md: String,
    pub out: String,
    pub md: String,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct UpgradeNotice {
    pub current_version: String,
    pub latest_version: String,
    pub manual_command: String,
    pub state_status: String,
    pub checked_live: bool,
    pub cache_hit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedSource {
    pub requested_source: String,
    pub resolved_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_root: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledTemplate {
    pub path: String,
    pub hash: String,
    pub merge_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallManifest {
    pub schema_version: String,
    pub installed_at: u64,
    pub jankurai_version: String,
    pub standard_version: String,
    pub auditor_version: String,
    pub schema_contract_version: String,
    pub paper_edition: String,
    pub target_stack_id: String,
    pub profile: String,
    pub level: String,
    pub ide: String,
    pub mode: String,
    pub update_channel: String,
    pub install_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub agent_request_version: String,
    pub agent_request_hash: String,
    #[serde(default)]
    pub templates: Vec<InstalledTemplate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateAction {
    pub path: String,
    pub action: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desired_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdatePlan {
    pub schema_version: String,
    pub command: String,
    pub status: String,
    pub generated_at: String,
    pub repo_root: String,
    pub current_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub standard_version: String,
    pub auditor_version: String,
    pub schema_contract_version: String,
    pub paper_edition: String,
    pub target_stack_id: String,
    pub update_channel: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_source: Option<ResolvedSource>,
    pub offline: bool,
    pub client_start: bool,
    pub self_update_requested: bool,
    pub self_update_available: bool,
    pub install_state: String,
    pub install_manifest_path: String,
    pub state_path: String,
    pub plan_path: String,
    pub md_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reexec_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_upgrade_score_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_upgrade_score_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_upgrade_score_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_upgrade_score_md: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub actions: Vec<UpdateAction>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateReceipt {
    pub schema_version: String,
    pub command: String,
    pub created_at: String,
    pub repo_root: String,
    pub current_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub update_channel: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_source: Option<ResolvedSource>,
    pub self_update_requested: bool,
    pub self_update_applied: bool,
    pub repo_update_applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reexec_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_upgrade_score_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_upgrade_score_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_upgrade_score_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_upgrade_score_md: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub actions: Vec<UpdateAction>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub commands_run: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_command: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub residual_risk: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateState {
    schema_version: String,
    checked_at: u64,
    ttl_seconds: u64,
    repo_root: String,
    status: String,
    current_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_version: Option<String>,
    install_state: String,
    plan_path: String,
    md_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest_hash: Option<String>,
}

pub fn audit_upgrade_notice(repo: &Path) -> Option<UpgradeNotice> {
    if std::env::var(NO_UPDATE_CHECK_ENV).as_deref() == Ok("1") {
        return None;
    }

    let repo = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    let args = default_audit_update_args(&repo);
    let state_path = repo.join(&args.state);
    if let Some(state) = read_state(&state_path) {
        if state_is_fresh(&state) {
            return notice_from_state(&state, false, true);
        }
    }

    match build_plan(&repo, &args) {
        Ok(plan) => {
            let _ = write_state(&repo, &args, &plan);
            notice_from_plan(&plan, true, false)
        }
        Err(_) => {
            let fallback = fallback_error_state(&repo, &args);
            let _ = write_update_state_file(&state_path, &fallback);
            None
        }
    }
}

pub fn run(args: UpdateArgs) -> Result<()> {
    let repo = args
        .repo
        .canonicalize()
        .with_context(|| format!("canonicalize {}", args.repo.display()))?;

    if args.client_start {
        return run_client_start(&repo, &args);
    }

    if args.self_update && args.apply && !args.yes {
        bail!("refusing to self-update without --yes");
    }
    if args.apply && !args.yes {
        bail!("refusing to write update changes without --yes");
    }
    if !args.apply && !args.check && !args.self_update {
        // Default update mode is a read-only check.
    }

    let plan = build_plan(&repo, &args)?;
    let plan_path = repo.join(&args.out);
    let md_path = repo.join(&args.md);
    validation::write_json(
        &repo,
        ArtifactSchema::UpdatePlan,
        &plan_path.display().to_string(),
        &plan,
    )?;
    crate::render::write_markdown(&md_path.display().to_string(), &render_plan(&plan))?;
    write_state(&repo, &args, &plan)?;

    // Outside an initialized repo: skip the plan dump and either upgrade or
    // tell the user they're already on the latest version.
    if plan.status == "not-initialized" {
        let reexeced = std::env::var(UPGRADE_REEXEC_ENV).as_deref() == Ok("1");
        if plan.self_update_available && !reexeced {
            // Treat bare `jankurai update` from any directory as an upgrade.
            let install_command = plan
                .resolved_source
                .as_ref()
                .and_then(|s| s.install_command.as_ref())
                .cloned();
            if let Some(install_command) = install_command {
                eprintln!(
                    "upgrading jankurai {} → {}",
                    plan.current_version,
                    plan.latest_version.as_deref().unwrap_or("?")
                );
                let status = std::process::Command::new(&install_command[0])
                    .args(&install_command[1..])
                    .current_dir(&repo)
                    .status()
                    .context("run self-update command")?;
                if !status.success() {
                    bail!("self-update command failed with {}", status);
                }
                eprintln!("upgrade complete — restart jankurai to use the new version");
                let receipt = build_receipt_stub(&repo, &plan, &args);
                let _ = write_receipt(&repo, &receipt);
                return Ok(());
            }
        }
        if !args.quiet {
            eprintln!(
                "jankurai {} — already on latest{}",
                plan.current_version,
                if plan.latest_version.is_some() { "" } else { " (version check skipped — offline?)" }
            );
        }
        let receipt = build_receipt_stub(&repo, &plan, &args);
        let _ = write_receipt(&repo, &receipt);
        return Ok(());
    }

    if !args.quiet {
        println!("{}", render_plan(&plan));
    }

    if args.fail_if_outdated && plan.status != "current" {
        bail!("update check found drift: {}", plan.status);
    }

    if !args.apply {
        return Ok(());
    }

    let reexeced = std::env::var(UPGRADE_REEXEC_ENV).as_deref() == Ok("1");
    let should_self_update = args.self_update
        && !args.skip_self
        && !reexeced
        && plan.self_update_available
        && plan
            .resolved_source
            .as_ref()
            .and_then(|source| source.install_command.as_ref())
            .is_some();

    if should_self_update {
        let receipt = build_receipt_stub(&repo, &plan, &args);
        let install_command = plan
            .resolved_source
            .as_ref()
            .and_then(|source| source.install_command.clone())
            .unwrap();
        let install_command_text = shell_join(&install_command);
        let install_status = Command::new(&install_command[0])
            .args(&install_command[1..])
            .current_dir(&repo)
            .status()
            .context("run self-update command")?;
        if !install_status.success() {
            let mut receipt = receipt;
            receipt.commands_run.push(install_command_text.clone());
            receipt.next_command = Some(install_command_text);
            write_receipt(&repo, &receipt)?;
            return Err(anyhow::anyhow!(
                "self-update command failed with {}",
                install_status
            ));
        }

        let reexec_command = build_reexec_command(&repo, &args)?;
        let reexec_command_text = shell_join(&reexec_command);
        let status = Command::new(&reexec_command[0])
            .args(&reexec_command[1..])
            .env(UPGRADE_REEXEC_ENV, "1")
            .current_dir(&repo)
            .status()
            .context("run upgrade reexec command")?;
        if !status.success() {
            let mut receipt = receipt;
            receipt.commands_run.push(install_command_text);
            receipt.commands_run.push(reexec_command_text.clone());
            receipt.self_update_applied = true;
            receipt.reexec_command = Some(reexec_command_text.clone());
            receipt.next_command = Some(reexec_command_text);
            write_receipt(&repo, &receipt)?;
            return Err(anyhow::anyhow!("reexec command failed with {}", status));
        }
        return Ok(());
    }

    let mut receipt = build_receipt_stub(&repo, &plan, &args);
    if reexeced && args.self_update {
        receipt.self_update_applied = true;
        if let Some(command) = plan
            .resolved_source
            .as_ref()
            .and_then(|source| source.install_command.as_ref())
        {
            receipt.commands_run.push(shell_join(command));
        }
        if let Some(command) = receipt.reexec_command.clone() {
            receipt.commands_run.push(command);
        }
    }
    let written = apply_repo_updates(&repo, &args, &plan)?;
    if !written.is_empty() {
        receipt.repo_update_applied = true;
        receipt.actions = written;
    }
    write_install_manifest(&repo, &args, &plan)?;

    if args.score {
        let score_command = build_score_command(&repo, &args)?;
        let score_command_text = shell_join(&score_command);
        receipt.commands_run.push(score_command_text.clone());
        let status = Command::new(&score_command[0])
            .args(&score_command[1..])
            .current_dir(&repo)
            .status()
            .context("run post-upgrade score command")?;
        if !status.success() {
            receipt.next_command = Some(score_command_text);
            write_receipt(&repo, &receipt)?;
            return Err(anyhow::anyhow!(
                "post-upgrade score command failed with {status}"
            ));
        }
    } else {
        receipt.next_command = Some(shell_join(&build_score_command(&repo, &args)?));
    }

    receipt.artifacts = vec![
        rel_path(&repo, &plan_path),
        rel_path(&repo, &md_path),
        rel_path(&repo, &repo.join(&args.state)),
    ];
    write_receipt(&repo, &receipt)?;
    if !args.quiet {
        println!("{}", render_receipt(&receipt));
    }
    Ok(())
}

fn run_client_start(repo: &Path, args: &UpdateArgs) -> Result<()> {
    let state_path = repo.join(&args.state);
    if let Some(state) = read_state(&state_path) {
        if state_is_fresh(&state) {
            return Ok(());
        }
    }
    match build_plan(repo, args) {
        Ok(plan) => {
            let _ = write_state(repo, args, &plan);
            Ok(())
        }
        Err(err) => {
            let fallback = fallback_error_state(repo, args);
            let _ = write_update_state_file(&state_path, &fallback);
            let _ = err;
            Ok(())
        }
    }
}

fn build_plan(repo: &Path, args: &UpdateArgs) -> Result<UpdatePlan> {
    let repo_has_jankurai = has_jankurai_files(repo);
    if !repo_has_jankurai {
        // Still resolve version even outside an initialized repo so that
        // `jankurai update` / `jankurai upgrade` work from any directory.
        let resolved_source = resolve_source_context(repo, args, None).ok();
        let cur = current_version();
        let self_update_available = resolved_source
            .as_ref()
            .and_then(|s| s.latest_version.as_ref())
            .and_then(|latest| Version::parse(latest).ok())
            .zip(Version::parse(&cur).ok())
            .map(|(latest, current)| latest > current)
            .unwrap_or(false);
        let latest_version = resolved_source
            .as_ref()
            .and_then(|s| s.latest_version.clone());
        let reexec_command = build_reexec_command(repo, args).ok().map(|v| shell_join(&v));
        return Ok(UpdatePlan {
            schema_version: UPDATE_SCHEMA_VERSION.into(),
            command: "jankurai update".into(),
            status: "not-initialized".into(),
            generated_at: now_string(),
            repo_root: repo.display().to_string(),
            current_version: cur,
            latest_version,
            standard_version: STANDARD_VERSION.into(),
            auditor_version: AUDITOR_VERSION.into(),
            schema_contract_version: SCHEMA_VERSION.into(),
            paper_edition: PAPER_EDITION.into(),
            target_stack_id: TARGET_STACK_ID.into(),
            update_channel: args.channel.clone(),
            source: args.source.clone(),
            resolved_source,
            offline: args.offline,
            client_start: args.client_start,
            self_update_requested: args.self_update,
            self_update_available,
            install_state: "not-initialized".into(),
            install_manifest_path: rel_path(repo, &install_manifest_path(repo)),
            state_path: rel_path(repo, &repo.join(&args.state)),
            plan_path: rel_path(repo, &repo.join(&args.out)),
            md_path: rel_path(repo, &repo.join(&args.md)),
            reexec_command,
            post_upgrade_score_command: Some(shell_join(&build_score_command(repo, args)?)),
            post_upgrade_score_mode: Some(args.score_mode.clone()),
            post_upgrade_score_json: Some(args.score_json.clone()),
            post_upgrade_score_md: Some(args.score_md.clone()),
            warnings: vec!["no jankurai control files found".into()],
            actions: Vec::new(),
            artifacts: vec![
                rel_path(repo, &repo.join(&args.out)),
                rel_path(repo, &repo.join(&args.md)),
                rel_path(repo, &repo.join(&args.state)),
            ],
        });
    }

    let install_manifest_path = install_manifest_path(repo);
    let install_manifest = load_install_manifest(&install_manifest_path).ok();
    let install_state = if install_manifest.is_some() {
        "installed"
    } else {
        "legacy-install"
    };
    let profile_manifest = if let Some(manifest) = install_manifest.as_ref() {
        init::profiles::resolve_profile(repo, &manifest.profile)?
    } else {
        init::profiles::resolve_profile(repo, &args.profile)?
    };
    let level = install_manifest
        .as_ref()
        .map(|manifest| manifest.level.as_str())
        .unwrap_or(args.level.as_str());
    let cargo_repo = repo.join("Cargo.toml").exists();
    let desired_paths = desired_paths(&profile_manifest, level);
    let resolved_source = resolve_source_context(repo, args, install_manifest.as_ref())?;
    let latest_version = resolved_source.latest_version.clone();
    let current_version = current_version();
    let self_update_available = latest_version
        .as_ref()
        .and_then(|latest| Version::parse(latest).ok())
        .zip(Version::parse(&current_version).ok())
        .map(|(latest, current)| latest > current)
        .unwrap_or(false);
    let installed_templates = install_manifest
        .as_ref()
        .map(|manifest| {
            manifest
                .templates
                .iter()
                .map(|template| (template.path.clone(), template.clone()))
                .collect::<std::collections::BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let mut actions = Vec::new();
    let mut warnings = Vec::new();
    for path in desired_paths {
        if path == "agent/jankurai-install.toml" {
            let action = if install_manifest.is_some() {
                "update"
            } else {
                "create"
            };
            actions.push(UpdateAction {
                path,
                action: action.into(),
                reason: "write the install manifest that records the installed scaffold".into(),
                current_hash: current_hash(&install_manifest_path),
                installed_hash: None,
                desired_hash: Some(sha256_text(&render_install_manifest_text(
                    &build_install_manifest(
                        repo,
                        &profile_manifest,
                        args,
                        &latest_version,
                        Some(&resolved_source),
                    ),
                ))),
                merge_policy: Some("keep-existing".into()),
            });
            continue;
        }
        let template = desired_body_for_path(repo, &path, &profile_manifest, level, cargo_repo)?;
        let current_path = repo.join(&path);
        let desired_hash = Some(sha256_text(&template));
        let current_hash = current_hash(&current_path);
        let installed_hash = installed_templates
            .get(&path)
            .map(|template| template.hash.clone());
        let merge_policy = profile_manifest
            .merge_policy_for_path(&path)
            .plan_action()
            .to_string();
        let action = classify_action(
            &path,
            &template,
            current_path.exists(),
            current_hash.as_deref(),
            installed_hash.as_deref(),
            profile_manifest.merge_policy_for_path(&path),
        );
        if action.action == "manual-review" {
            warnings.push(format!("{path}: {}", action.reason));
        }
        actions.push(UpdateAction {
            path,
            action: action.action,
            reason: action.reason,
            current_hash,
            installed_hash,
            desired_hash,
            merge_policy: Some(merge_policy),
        });
    }

    if install_manifest.is_none() {
        actions.insert(
            0,
            UpdateAction {
                path: "agent/jankurai-install.toml".into(),
                action: "create".into(),
                reason: "record the installed Jankurai version and file hashes".into(),
                current_hash: current_hash(&install_manifest_path),
                installed_hash: None,
                desired_hash: None,
                merge_policy: Some("keep-existing".into()),
            },
        );
    }

    let status = if actions.iter().any(|action| action.action == "conflict") {
        "conflict"
    } else if actions
        .iter()
        .any(|action| action.action == "manual-review")
    {
        "manual-review"
    } else if actions.iter().any(|action| action.action != "unchanged") {
        "outdated"
    } else {
        "current"
    };

    Ok(UpdatePlan {
        schema_version: UPDATE_SCHEMA_VERSION.into(),
        command: "jankurai update".into(),
        status: status.into(),
        generated_at: now_string(),
        repo_root: repo.display().to_string(),
        current_version,
        latest_version,
        standard_version: STANDARD_VERSION.into(),
        auditor_version: AUDITOR_VERSION.into(),
        schema_contract_version: SCHEMA_VERSION.into(),
        paper_edition: PAPER_EDITION.into(),
        target_stack_id: TARGET_STACK_ID.into(),
        update_channel: args.channel.clone(),
        source: args.source.clone(),
        resolved_source: Some(resolved_source.clone()),
        offline: args.offline,
        client_start: args.client_start,
        self_update_requested: args.self_update,
        self_update_available,
        install_state: install_state.into(),
        install_manifest_path: rel_path(repo, &install_manifest_path),
        state_path: rel_path(repo, &repo.join(&args.state)),
        plan_path: rel_path(repo, &repo.join(&args.out)),
        md_path: rel_path(repo, &repo.join(&args.md)),
        reexec_command: if args.self_update && self_update_available {
            build_reexec_command(repo, args)
                .ok()
                .map(|command| shell_join(&command))
        } else {
            None
        },
        post_upgrade_score_command: Some(shell_join(&build_score_command(repo, args)?)),
        post_upgrade_score_mode: Some(args.score_mode.clone()),
        post_upgrade_score_json: Some(args.score_json.clone()),
        post_upgrade_score_md: Some(args.score_md.clone()),
        warnings,
        actions,
        artifacts: vec![
            rel_path(repo, &repo.join(&args.out)),
            rel_path(repo, &repo.join(&args.md)),
            rel_path(repo, &repo.join(&args.state)),
        ],
    })
}

fn apply_repo_updates(
    repo: &Path,
    args: &UpdateArgs,
    plan: &UpdatePlan,
) -> Result<Vec<UpdateAction>> {
    let install_manifest_path = install_manifest_path(repo);
    let install_manifest = load_install_manifest(&install_manifest_path).ok();
    let profile_manifest = if let Some(manifest) = install_manifest.as_ref() {
        init::profiles::resolve_profile(repo, &manifest.profile)?
    } else {
        init::profiles::resolve_profile(repo, &args.profile)?
    };
    let level = install_manifest
        .as_ref()
        .map(|manifest| manifest.level.as_str())
        .unwrap_or(args.level.as_str());
    let cargo_repo = repo.join("Cargo.toml").exists();
    let mut written = Vec::new();
    for action in &plan.actions {
        if action.path == "agent/jankurai-install.toml" {
            continue;
        }
        let path = repo.join(&action.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let desired =
            desired_body_for_path(repo, &action.path, &profile_manifest, level, cargo_repo)?;
        match action.action.as_str() {
            "create" | "update" => {
                fs::write(&path, desired)?;
                written.push(action.clone());
            }
            "merge" => {
                let current = fs::read_to_string(&path).unwrap_or_default();
                let merged = match profile_manifest.merge_policy_for_path(&action.path) {
                    MergePolicyAction::MergeJson => merge::merge_json(&current, &desired)?,
                    MergePolicyAction::MergeToml
                        if action.path == "agent/standard-version.toml" =>
                    {
                        merge::merge_standard_version_toml(&current, &desired)?
                    }
                    MergePolicyAction::MergeToml => merge::merge_toml(&current, &desired)?,
                    MergePolicyAction::MergeLines => merge::merge_lines(&current, &desired)?,
                    MergePolicyAction::MergeMarker | MergePolicyAction::KeepExisting => desired,
                };
                if merged != current {
                    fs::write(&path, merged)?;
                    written.push(action.clone());
                }
            }
            "unchanged" | "user-modified" | "conflict" | "manual-review" => {}
            other => bail!("unsupported update action `{other}`"),
        }
    }
    Ok(written)
}

fn write_install_manifest(repo: &Path, args: &UpdateArgs, plan: &UpdatePlan) -> Result<()> {
    let install_manifest_path = install_manifest_path(repo);
    let profile_manifest = init::profiles::resolve_profile(repo, &args.profile)?;
    let manifest = build_install_manifest(
        repo,
        &profile_manifest,
        args,
        &plan.latest_version,
        plan.resolved_source.as_ref(),
    );
    if let Some(parent) = install_manifest_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &install_manifest_path,
        render_install_manifest_text(&manifest),
    )?;
    Ok(())
}

fn build_install_manifest(
    repo: &Path,
    profile_manifest: &ProfileManifest,
    args: &UpdateArgs,
    latest_version: &Option<String>,
    resolved_source: Option<&ResolvedSource>,
) -> InstallManifest {
    let cargo_repo = repo.join("Cargo.toml").exists();
    let templates = profile_manifest
        .generated_paths
        .iter()
        .filter(|path| path.as_str() != "agent/jankurai-install.toml")
        .filter_map(|path| {
            let body = desired_body_for_path(repo, path, profile_manifest, &args.level, cargo_repo)
                .ok()?;
            Some(InstalledTemplate {
                path: path.clone(),
                hash: sha256_text(&body),
                merge_policy: profile_manifest
                    .merge_policy_for_path(path)
                    .plan_action()
                    .into(),
            })
        })
        .collect();
    InstallManifest {
        schema_version: INSTALL_MANIFEST_SCHEMA_VERSION.into(),
        installed_at: now_secs(),
        jankurai_version: current_version(),
        standard_version: STANDARD_VERSION.into(),
        auditor_version: AUDITOR_VERSION.into(),
        schema_contract_version: SCHEMA_VERSION.into(),
        paper_edition: PAPER_EDITION.into(),
        target_stack_id: TARGET_STACK_ID.into(),
        profile: profile_manifest.id.clone(),
        level: args.level.clone(),
        ide: args.ide.clone(),
        mode: "advisory".into(),
        update_channel: args.channel.clone(),
        install_source: args.source.clone(),
        resolved_source: resolved_source
            .map(|source| source.resolved_source.clone())
            .or_else(|| {
                if args.source == "auto" {
                    latest_version
                        .as_ref()
                        .map(|_| DEFAULT_INSTALL_SOURCE.to_string())
                } else {
                    Some(args.source.clone())
                }
            }),
        source_url: resolved_source
            .and_then(|source| source.source_url.clone())
            .or_else(|| match args.source.as_str() {
                "github" | "git" => Some(DEFAULT_SOURCE_URL.into()),
                _ => None,
            }),
        agent_request_version: adapters::AGENT_REQUEST_VERSION.into(),
        agent_request_hash: adapters::AGENT_REQUEST_MARKER.into(),
        templates,
    }
}

fn render_install_manifest_text(manifest: &InstallManifest) -> String {
    let mut out = String::new();
    out.push_str("# Generated by jankurai update\n# DO NOT EDIT BY HAND.\n\n");
    out.push_str(&toml::to_string_pretty(manifest).unwrap());
    out
}

fn load_install_manifest(path: &Path) -> Result<InstallManifest> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let manifest: InstallManifest = toml::from_str(&text)?;
    Ok(manifest)
}

fn resolve_source_context(
    repo: &Path,
    args: &UpdateArgs,
    install_manifest: Option<&InstallManifest>,
) -> Result<ResolvedSource> {
    let selection = select_update_source(repo, args, install_manifest);
    let latest_version = resolve_latest_version(repo, args, install_manifest, &selection)?;
    let install_root = cargo_install_root();
    let install_command = build_install_command(
        repo,
        &selection.resolved_source,
        selection.source_url.as_deref(),
        latest_version.as_deref(),
        install_root.as_deref(),
    )
    .ok();
    Ok(ResolvedSource {
        requested_source: selection.requested_source,
        resolved_source: selection.resolved_source,
        source_url: selection.source_url,
        latest_version,
        install_command,
        install_root,
        reason: selection.reason,
    })
}

#[derive(Debug, Clone)]
struct SourceSelection {
    requested_source: String,
    resolved_source: String,
    source_url: Option<String>,
    reason: String,
}

fn select_update_source(
    repo: &Path,
    args: &UpdateArgs,
    install_manifest: Option<&InstallManifest>,
) -> SourceSelection {
    if args.source != "auto" {
        return explicit_source_selection(args.source.as_str(), install_manifest);
    }
    if local_checkout_is_newer(repo) {
        return SourceSelection {
            requested_source: args.source.clone(),
            resolved_source: "local".into(),
            source_url: None,
            reason: "newer crates/jankurai checkout is available locally".into(),
        };
    }
    if let Some(selection) = manifest_source_selection(install_manifest) {
        return selection;
    }
    SourceSelection {
        requested_source: args.source.clone(),
        resolved_source: "github".into(),
        source_url: Some(DEFAULT_SOURCE_URL.into()),
        reason: "auto falls back to immutable GitHub release tags".into(),
    }
}

fn explicit_source_selection(
    source: &str,
    install_manifest: Option<&InstallManifest>,
) -> SourceSelection {
    match source {
        "local" => SourceSelection {
            requested_source: source.into(),
            resolved_source: "local".into(),
            source_url: None,
            reason: "explicit local source checkout requested".into(),
        },
        "github" => SourceSelection {
            requested_source: source.into(),
            resolved_source: "github".into(),
            source_url: Some(DEFAULT_SOURCE_URL.into()),
            reason: "explicit GitHub release tag source requested".into(),
        },
        "git" => SourceSelection {
            requested_source: source.into(),
            resolved_source: "git".into(),
            source_url: source_url_for_source(install_manifest, "git"),
            reason: "explicit git source requested".into(),
        },
        "crates-io" => SourceSelection {
            requested_source: source.into(),
            resolved_source: "crates-io".into(),
            source_url: None,
            reason: "explicit crates.io source requested".into(),
        },
        other => SourceSelection {
            requested_source: other.into(),
            resolved_source: other.into(),
            source_url: source_url_for_source(install_manifest, other),
            reason: "explicit source requested".into(),
        },
    }
}

fn manifest_source_selection(
    install_manifest: Option<&InstallManifest>,
) -> Option<SourceSelection> {
    let manifest = install_manifest?;
    let resolved = manifest
        .resolved_source
        .as_deref()
        .unwrap_or(manifest.install_source.as_str());
    if resolved.is_empty() || resolved == "crates-io" {
        return None;
    }
    Some(SourceSelection {
        requested_source: "auto".into(),
        resolved_source: resolved.into(),
        source_url: manifest
            .source_url
            .clone()
            .filter(|url| !url.trim().is_empty())
            .or_else(|| source_url_for_source(install_manifest, resolved)),
        reason: "install manifest source remains authoritative for auto".into(),
    })
}

fn source_url_for_source(
    install_manifest: Option<&InstallManifest>,
    source: &str,
) -> Option<String> {
    match source {
        "github" | "git" => install_manifest
            .and_then(|manifest| manifest.source_url.clone())
            .filter(|url| !url.trim().is_empty())
            .or_else(|| Some(DEFAULT_SOURCE_URL.into())),
        _ => None,
    }
}

fn resolve_latest_version(
    repo: &Path,
    args: &UpdateArgs,
    install_manifest: Option<&InstallManifest>,
    selection: &SourceSelection,
) -> Result<Option<String>> {
    if let Ok(version) = std::env::var(TEST_LATEST_VERSION_ENV) {
        let version = version.trim();
        if !version.is_empty() {
            return Ok(Some(version.to_string()));
        }
    }
    if args.offline {
        return Ok(None);
    }
    match selection.resolved_source.as_str() {
        "local" => Ok(read_local_version(repo).ok()),
        "crates-io" => fetch_crates_io_version(),
        "github" | "git" => {
            let url = selection
                .source_url
                .as_deref()
                .or_else(|| install_manifest.and_then(|manifest| manifest.source_url.as_deref()))
                .filter(|url| !url.trim().is_empty())
                .unwrap_or(DEFAULT_SOURCE_URL);
            latest_git_tag(url)
        }
        _ => Ok(None),
    }
}

fn local_checkout_is_newer(repo: &Path) -> bool {
    read_local_version(repo)
        .ok()
        .and_then(|version| Version::parse(&version).ok())
        .zip(Version::parse(&current_version()).ok())
        .map(|(local, current)| local > current)
        .unwrap_or(false)
}

fn fetch_crates_io_version() -> Result<Option<String>> {
    if let Ok(version) = std::env::var(TEST_LATEST_VERSION_ENV) {
        let version = version.trim();
        return Ok((!version.is_empty()).then(|| version.to_string()));
    }

    let timeout = Duration::from_millis(AUDIT_NETWORK_TIMEOUT_MS);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(timeout)
        .timeout_read(timeout)
        .build();
    let response = agent.get("https://crates.io/api/v1/crates/jankurai").call();
    match response {
        Ok(response) => {
            let value: serde_json::Value = response.into_json()?;
            Ok(value
                .get("crate")
                .and_then(|crate_value| crate_value.get("max_version"))
                .and_then(|value| value.as_str())
                .map(|s| s.to_string()))
        }
        Err(_) => Ok(None),
    }
}

fn latest_git_tag(url: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["ls-remote", "--tags", url])
        .output()
        .with_context(|| format!("git ls-remote --tags {url}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    let mut versions = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(tag) = line.split_whitespace().nth(1) {
            if let Some(tag) = tag.rsplit('/').next() {
                if let Ok(version) = Version::parse(tag.trim_start_matches('v')) {
                    versions.push(version);
                }
            }
        }
    }
    versions.sort();
    Ok(versions.pop().map(|version| version.to_string()))
}

fn read_local_version(repo: &Path) -> Result<String> {
    let cargo = repo.join("crates/jankurai/Cargo.toml");
    let text = fs::read_to_string(&cargo)?;
    let value: toml::Value = toml::from_str(&text)?;
    let package = value
        .get("package")
        .ok_or_else(|| anyhow::anyhow!("missing crates/jankurai package"))?;
    let name = package
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if name != "jankurai" {
        bail!("crates/jankurai package.name: expected jankurai, got {name}");
    }
    package
        .get("version")
        .and_then(|value| value.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("missing crates/jankurai package version"))
}

fn desired_paths(profile_manifest: &ProfileManifest, level: &str) -> Vec<String> {
    let mut paths = profile_manifest.generated_paths.clone();
    if level == "agents" {
        paths.retain(|path| {
            matches!(
                path.as_str(),
                "AGENTS.md"
                    | "CLAUDE.md"
                    | "GEMINI.md"
                    | ".cursor/rules/jankurai.mdc"
                    | ".github/copilot-instructions.md"
                    | ".github/instructions/jankurai.instructions.md"
                    | ".github/instructions/jankurai-rust.instructions.md"
                    | ".github/instructions/jankurai-web.instructions.md"
                    | ".github/instructions/jankurai-python-ai.instructions.md"
                    | ".agents/agents.md"
                    | ".agents/skills/jankurai/SKILL.md"
                    | ".agents/workflows/jankurai-audit.md"
                    | ".claude/skills/jankurai/SKILL.md"
            )
        });
    }
    paths.sort();
    paths.dedup();
    paths
}

fn desired_body_for_path(
    repo: &Path,
    path: &str,
    profile_manifest: &ProfileManifest,
    level: &str,
    cargo_repo: bool,
) -> Result<String> {
    if path == "agent/jankurai-install.toml" {
        let manifest = build_install_manifest(
            repo,
            profile_manifest,
            &UpdateArgs {
                repo: repo.to_path_buf(),
                check: true,
                apply: false,
                yes: false,
                self_update: false,
                skip_self: false,
                client_start: false,
                quiet: true,
                channel: DEFAULT_UPDATE_CHANNEL.into(),
                source: DEFAULT_INSTALL_SOURCE.into(),
                offline: true,
                fail_if_outdated: false,
                install_missing: false,
                profile: profile_manifest.id.clone(),
                level: level.into(),
                ide: "all".into(),
                score: false,
                score_mode: "standard".into(),
                score_json: "target/jankurai/repo-score.json".into(),
                score_md: "target/jankurai/repo-score.md".into(),
                out: "target/jankurai/update/update-plan.json".into(),
                md: "target/jankurai/update/update-plan.md".into(),
                state: "target/jankurai/update/state.json".into(),
            },
            &None,
            None,
        );
        return Ok(render_install_manifest_text(&manifest));
    }
    let body = init::templates::body_for_path(path, level, cargo_repo)
        .with_context(|| format!("no template registered for `{path}`"))?;
    Ok(body.to_string())
}

fn classify_action(
    path: &str,
    desired: &str,
    exists: bool,
    current_hash: Option<&str>,
    installed_hash: Option<&str>,
    merge_policy: MergePolicyAction,
) -> UpdateAction {
    if !exists {
        return UpdateAction {
            path: path.into(),
            action: "create".into(),
            reason: "file is missing".into(),
            current_hash: current_hash.map(ToOwned::to_owned),
            installed_hash: installed_hash.map(ToOwned::to_owned),
            desired_hash: Some(sha256_text(desired)),
            merge_policy: Some(merge_policy.plan_action().into()),
        };
    }

    let desired_hash = sha256_text(desired);
    if current_hash == Some(desired_hash.as_str()) {
        return UpdateAction {
            path: path.into(),
            action: "unchanged".into(),
            reason: "current file already matches the desired template".into(),
            current_hash: current_hash.map(ToOwned::to_owned),
            installed_hash: installed_hash.map(ToOwned::to_owned),
            desired_hash: Some(desired_hash),
            merge_policy: Some(merge_policy.plan_action().into()),
        };
    }

    match merge_policy {
        MergePolicyAction::MergeJson
        | MergePolicyAction::MergeToml
        | MergePolicyAction::MergeLines => {
            return UpdateAction {
                path: path.into(),
                action: "merge".into(),
                reason: "merge policy allows additive update".into(),
                current_hash: current_hash.map(ToOwned::to_owned),
                installed_hash: installed_hash.map(ToOwned::to_owned),
                desired_hash: Some(desired_hash),
                merge_policy: Some(merge_policy.plan_action().into()),
            };
        }
        MergePolicyAction::MergeMarker => {
            if path == "AGENTS.md" || path == "agent/JANKURAI_STANDARD.md" {
                return UpdateAction {
                    path: path.into(),
                    action: "merge".into(),
                    reason: "marker-based guidance file can be refreshed".into(),
                    current_hash: current_hash.map(ToOwned::to_owned),
                    installed_hash: installed_hash.map(ToOwned::to_owned),
                    desired_hash: Some(desired_hash),
                    merge_policy: Some(merge_policy.plan_action().into()),
                };
            }
        }
        MergePolicyAction::KeepExisting => {}
    }

    if let Some(installed) = installed_hash {
        if current_hash != Some(installed) {
            return UpdateAction {
                path: path.into(),
                action: "user-modified".into(),
                reason: "current file diverges from the recorded installed hash".into(),
                current_hash: current_hash.map(ToOwned::to_owned),
                installed_hash: Some(installed.to_string()),
                desired_hash: Some(desired_hash),
                merge_policy: Some(merge_policy.plan_action().into()),
            };
        }
    }

    if path.contains("SKILL.md")
        || path.ends_with(".md")
        || path.ends_with(".toml")
        || path.ends_with(".json")
    {
        UpdateAction {
            path: path.into(),
            action: "update".into(),
            reason: "template changed since installation".into(),
            current_hash: current_hash.map(ToOwned::to_owned),
            installed_hash: installed_hash.map(ToOwned::to_owned),
            desired_hash: Some(desired_hash),
            merge_policy: Some(merge_policy.plan_action().into()),
        }
    } else {
        UpdateAction {
            path: path.into(),
            action: "manual-review".into(),
            reason: "current file differs from the template and is not safely mergeable".into(),
            current_hash: current_hash.map(ToOwned::to_owned),
            installed_hash: installed_hash.map(ToOwned::to_owned),
            desired_hash: Some(desired_hash),
            merge_policy: Some(merge_policy.plan_action().into()),
        }
    }
}

fn write_receipt(repo: &Path, receipt: &UpdateReceipt) -> Result<PathBuf> {
    let dir = repo.join("target/jankurai/receipts");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("update-{}.json", now_secs()));
    validation::validate_value(
        repo,
        ArtifactSchema::UpdateReceipt,
        &serde_json::to_value(receipt)?,
    )?;
    fs::write(&path, serde_json::to_string_pretty(receipt)?)?;
    Ok(path)
}

fn write_state(repo: &Path, args: &UpdateArgs, plan: &UpdatePlan) -> Result<()> {
    let manifest_hash = load_install_manifest(&install_manifest_path(repo))
        .ok()
        .map(|manifest| sha256_text(&toml::to_string_pretty(&manifest).unwrap_or_default()));
    let state = UpdateState {
        schema_version: STATE_SCHEMA_VERSION.into(),
        checked_at: now_secs(),
        ttl_seconds: CLIENT_START_TTL_SECS,
        repo_root: repo.display().to_string(),
        status: plan.status.clone(),
        current_version: plan.current_version.clone(),
        latest_version: plan.latest_version.clone(),
        install_state: plan.install_state.clone(),
        plan_path: plan.plan_path.clone(),
        md_path: plan.md_path.clone(),
        manifest_hash,
    };
    write_update_state_file(&repo.join(&args.state), &state)?;
    Ok(())
}

fn default_audit_update_args(repo: &Path) -> UpdateArgs {
    UpdateArgs {
        repo: repo.to_path_buf(),
        check: true,
        apply: false,
        yes: false,
        self_update: false,
        skip_self: false,
        client_start: true,
        quiet: true,
        channel: DEFAULT_UPDATE_CHANNEL.into(),
        source: "auto".into(),
        offline: false,
        fail_if_outdated: false,
        install_missing: false,
        profile: "rust-ts-postgres".into(),
        level: "full".into(),
        ide: "all".into(),
        score: false,
        score_mode: "standard".into(),
        score_json: "target/jankurai/repo-score.json".into(),
        score_md: "target/jankurai/repo-score.md".into(),
        out: "target/jankurai/update/update-plan.json".into(),
        md: "target/jankurai/update/update-plan.md".into(),
        state: "target/jankurai/update/state.json".into(),
    }
}

fn read_state(path: &Path) -> Option<UpdateState> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<UpdateState>(&text).ok()
}

fn state_is_fresh(state: &UpdateState) -> bool {
    now_secs().saturating_sub(state.checked_at) <= state.ttl_seconds
}

fn notice_from_state(
    state: &UpdateState,
    checked_live: bool,
    cache_hit: bool,
) -> Option<UpgradeNotice> {
    let latest = state.latest_version.as_deref()?;
    notice_if_newer(
        &current_version(),
        latest,
        &state.status,
        checked_live,
        cache_hit,
    )
}

fn notice_from_plan(
    plan: &UpdatePlan,
    checked_live: bool,
    cache_hit: bool,
) -> Option<UpgradeNotice> {
    let latest = plan.latest_version.as_deref()?;
    notice_if_newer(
        &plan.current_version,
        latest,
        &plan.status,
        checked_live,
        cache_hit,
    )
}

fn notice_if_newer(
    current: &str,
    latest: &str,
    state_status: &str,
    checked_live: bool,
    cache_hit: bool,
) -> Option<UpgradeNotice> {
    let current_version = Version::parse(current).ok()?;
    let latest_version = Version::parse(latest).ok()?;
    if latest_version <= current_version {
        return None;
    }
    if AUDIT_UPGRADE_NOTICE_EMITTED.swap(true, Ordering::SeqCst) {
        return None;
    }
    Some(UpgradeNotice {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        manual_command: MANUAL_UPGRADE_COMMAND.into(),
        state_status: state_status.into(),
        checked_live,
        cache_hit,
    })
}

fn fallback_error_state(repo: &Path, args: &UpdateArgs) -> UpdateState {
    UpdateState {
        schema_version: STATE_SCHEMA_VERSION.into(),
        checked_at: now_secs(),
        ttl_seconds: CLIENT_START_TTL_SECS,
        repo_root: repo.display().to_string(),
        status: "error".into(),
        current_version: current_version(),
        latest_version: None,
        install_state: "unknown".into(),
        plan_path: rel_path(repo, &repo.join(&args.out)),
        md_path: rel_path(repo, &repo.join(&args.md)),
        manifest_hash: None,
    }
}

fn write_update_state_file(path: &Path, state: &UpdateState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

fn build_receipt_stub(repo: &Path, plan: &UpdatePlan, args: &UpdateArgs) -> UpdateReceipt {
    UpdateReceipt {
        schema_version: UPDATE_SCHEMA_VERSION.into(),
        command: "jankurai update".into(),
        created_at: now_string(),
        repo_root: repo.display().to_string(),
        current_version: plan.current_version.clone(),
        latest_version: plan.latest_version.clone(),
        update_channel: plan.update_channel.clone(),
        source: plan.source.clone(),
        resolved_source: plan.resolved_source.clone(),
        self_update_requested: args.self_update,
        self_update_applied: args.self_update
            && std::env::var(UPGRADE_REEXEC_ENV).as_deref() == Ok("1"),
        repo_update_applied: false,
        reexec_command: plan.reexec_command.clone(),
        post_upgrade_score_command: plan.post_upgrade_score_command.clone(),
        post_upgrade_score_mode: plan.post_upgrade_score_mode.clone(),
        post_upgrade_score_json: plan.post_upgrade_score_json.clone(),
        post_upgrade_score_md: plan.post_upgrade_score_md.clone(),
        actions: plan.actions.clone(),
        commands_run: Vec::new(),
        next_command: None,
        residual_risk: Vec::new(),
        artifacts: vec![
            plan.plan_path.clone(),
            plan.md_path.clone(),
            plan.state_path.clone(),
        ],
    }
}

fn build_install_command(
    repo: &Path,
    resolved_source: &str,
    source_url: Option<&str>,
    latest_version: Option<&str>,
    install_root: Option<&str>,
) -> Result<Vec<String>> {
    let mut cmd = vec!["cargo".into(), "install".into()];
    match resolved_source {
        "local" => {
            cmd.push("--path".into());
            cmd.push(repo.join("crates/jankurai").display().to_string());
            cmd.push("--locked".into());
            cmd.push("--force".into());
        }
        "github" | "git" => {
            let url = source_url.unwrap_or(DEFAULT_SOURCE_URL);
            let tag = latest_version
                .map(|version| format!("v{version}"))
                .ok_or_else(|| anyhow::anyhow!("git source requires a latest version tag"))?;
            cmd.push("--git".into());
            cmd.push(url.into());
            cmd.push("--tag".into());
            cmd.push(tag);
            cmd.push("--package".into());
            cmd.push("jankurai".into());
            cmd.push("--locked".into());
            cmd.push("--force".into());
        }
        "crates-io" => {
            cmd.push("jankurai".into());
            cmd.push("--locked".into());
            cmd.push("--force".into());
        }
        other => {
            return Err(anyhow::anyhow!("unsupported install source `{other}`"));
        }
    }
    if let Some(root) = install_root {
        cmd.push("--root".into());
        cmd.push(root.into());
    }
    Ok(cmd)
}

#[cfg(test)]
fn build_self_update_command(
    repo: &Path,
    args: &UpdateArgs,
    plan: &UpdatePlan,
) -> Result<Vec<String>> {
    if let Some(source) = plan.resolved_source.as_ref() {
        if let Some(command) = source.install_command.clone() {
            return Ok(command);
        }
        return build_install_command(
            repo,
            &source.resolved_source,
            source.source_url.as_deref(),
            source.latest_version.as_deref(),
            source.install_root.as_deref(),
        );
    }
    let install_manifest = load_install_manifest(&install_manifest_path(repo)).ok();
    let source_context = resolve_source_context(repo, args, install_manifest.as_ref())?;
    build_install_command(
        repo,
        &source_context.resolved_source,
        source_context.source_url.as_deref(),
        source_context.latest_version.as_deref(),
        source_context.install_root.as_deref(),
    )
}

fn build_reexec_command(repo: &Path, args: &UpdateArgs) -> Result<Vec<String>> {
    let mut cmd = vec![command_executable()?];
    cmd.push("update".into());
    cmd.push(repo.display().to_string());
    if args.apply {
        cmd.push("--apply".into());
    }
    if args.yes {
        cmd.push("--yes".into());
    }
    if args.self_update {
        cmd.push("--self".into());
    }
    cmd.push("--skip-self".into());
    if args.quiet {
        cmd.push("--quiet".into());
    }
    cmd.push("--channel".into());
    cmd.push(args.channel.clone());
    cmd.push("--source".into());
    cmd.push(args.source.clone());
    if args.offline {
        cmd.push("--offline".into());
    }
    if args.fail_if_outdated {
        cmd.push("--fail-if-outdated".into());
    }
    if args.install_missing {
        cmd.push("--install-missing".into());
    }
    cmd.push("--profile".into());
    cmd.push(args.profile.clone());
    cmd.push("--level".into());
    cmd.push(args.level.clone());
    cmd.push("--ide".into());
    cmd.push(args.ide.clone());
    if args.score {
        cmd.push("--score".into());
        cmd.push("--score-mode".into());
        cmd.push(args.score_mode.clone());
        cmd.push("--score-json".into());
        cmd.push(args.score_json.clone());
        cmd.push("--score-md".into());
        cmd.push(args.score_md.clone());
    }
    cmd.push("--out".into());
    cmd.push(args.out.clone());
    cmd.push("--md".into());
    cmd.push(args.md.clone());
    cmd.push("--state".into());
    cmd.push(args.state.clone());
    Ok(cmd)
}

fn build_score_command(repo: &Path, args: &UpdateArgs) -> Result<Vec<String>> {
    let mut cmd = vec![command_executable()?];
    cmd.push("score".into());
    cmd.push(repo.display().to_string());
    cmd.push("--mode".into());
    cmd.push(args.score_mode.clone());
    cmd.push("--json".into());
    cmd.push(args.score_json.clone());
    cmd.push("--md".into());
    cmd.push(args.score_md.clone());
    Ok(cmd)
}

pub fn cargo_install_root() -> Option<String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            if bin_dir.file_name().and_then(|name| name.to_str()) == Some("bin") {
                if let Some(root) = bin_dir.parent() {
                    return Some(root.display().to_string());
                }
            }
        }
    }
    if let Ok(home) = std::env::var("CARGO_HOME") {
        return Some(home);
    }
    std::env::var("HOME")
        .ok()
        .map(|home| Path::new(&home).join(".cargo").display().to_string())
}

fn command_executable() -> Result<String> {
    let exe = std::env::current_exe().context("resolve current executable")?;
    Ok(exe.display().to_string())
}

fn shell_join(command: &[String]) -> String {
    command
        .iter()
        .map(|part| shell_quote(part))
        .collect::<Vec<_>>()
        .join(" ")
}

fn has_jankurai_files(repo: &Path) -> bool {
    init::detect::existing_standard_files(repo)
        .into_iter()
        .chain(adapters::ADAPTER_PATHS.iter().map(|path| path.to_string()))
        .any(|rel| repo.join(rel).exists())
        || install_manifest_path(repo).exists()
}

fn install_manifest_path(repo: &Path) -> PathBuf {
    repo.join("agent/jankurai-install.toml")
}

fn current_hash(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|text| sha256_text(&text))
        .or_else(|| {
            if path.exists() {
                fs::read(path).ok().map(|bytes| sha256_bytes(&bytes))
            } else {
                None
            }
        })
}

fn sha256_text(text: &str) -> String {
    sha256_bytes(text.as_bytes())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn rel_path(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn now_string() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn current_version() -> String {
    env!("CARGO_PKG_VERSION").into()
}

fn shell_quote(value: &str) -> String {
    if value.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '.' | '/' | '_' | '-' | ':' | '@' | '+')
    }) {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn render_plan(plan: &UpdatePlan) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Update Plan");
    let _ = writeln!(out);
    let _ = writeln!(out, "- status: `{}`", plan.status);
    let _ = writeln!(out, "- current version: `{}`", plan.current_version);
    if let Some(latest) = &plan.latest_version {
        let _ = writeln!(out, "- latest version: `{latest}`");
    }
    let _ = writeln!(out, "- channel: `{}`", plan.update_channel);
    let _ = writeln!(out, "- source: `{}`", plan.source);
    if let Some(source) = &plan.resolved_source {
        let _ = writeln!(out, "- resolved source: `{}`", source.resolved_source);
        let _ = writeln!(out, "- requested source: `{}`", source.requested_source);
        if let Some(url) = &source.source_url {
            let _ = writeln!(out, "- source url: `{url}`");
        }
        if let Some(command) = &source.install_command {
            let _ = writeln!(out, "- install command: `{}`", shell_join(command));
        }
        if let Some(root) = &source.install_root {
            let _ = writeln!(out, "- install root: `{root}`");
        }
        let _ = writeln!(out, "- source reason: `{}`", source.reason);
    }
    let _ = writeln!(out, "- install state: `{}`", plan.install_state);
    let _ = writeln!(
        out,
        "- self update requested: `{}`",
        plan.self_update_requested
    );
    let _ = writeln!(
        out,
        "- self update available: `{}`",
        plan.self_update_available
    );
    if let Some(command) = &plan.reexec_command {
        let _ = writeln!(out, "- reexec command: `{command}`");
    }
    if let Some(command) = &plan.post_upgrade_score_command {
        let _ = writeln!(out, "- post-upgrade score command: `{command}`");
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "Actions:");
    for action in &plan.actions {
        let _ = writeln!(
            out,
            "- {} {}: {}",
            action.action, action.path, action.reason
        );
    }
    if !plan.warnings.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "Warnings:");
        for warning in &plan.warnings {
            let _ = writeln!(out, "- {warning}");
        }
    }
    out
}

fn render_receipt(receipt: &UpdateReceipt) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Update Receipt");
    let _ = writeln!(out, "- current version: `{}`", receipt.current_version);
    if let Some(latest) = &receipt.latest_version {
        let _ = writeln!(out, "- latest version: `{latest}`");
    }
    if let Some(source) = &receipt.resolved_source {
        let _ = writeln!(out, "- resolved source: `{}`", source.resolved_source);
    }
    let _ = writeln!(
        out,
        "- self update applied: `{}`",
        receipt.self_update_applied
    );
    let _ = writeln!(
        out,
        "- repo update applied: `{}`",
        receipt.repo_update_applied
    );
    if let Some(command) = &receipt.reexec_command {
        let _ = writeln!(out, "- reexec command: `{command}`");
    }
    if let Some(command) = &receipt.post_upgrade_score_command {
        let _ = writeln!(out, "- post-upgrade score command: `{command}`");
    }
    if let Some(command) = &receipt.next_command {
        let _ = writeln!(out, "- next command: `{command}`");
    }
    if !receipt.commands_run.is_empty() {
        let _ = writeln!(out, "- commands:");
        for command in &receipt.commands_run {
            let _ = writeln!(out, "  - {command}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn auto_self_update_command_prefers_newer_local_checkout() {
        let repo = tempdir().unwrap();
        fs::create_dir_all(repo.path().join("crates/jankurai")).unwrap();
        fs::write(
            repo.path().join("crates/jankurai/Cargo.toml"),
            "[package]\nname = \"jankurai\"\nversion = \"999.0.0\"\n",
        )
        .unwrap();
        let args = UpdateArgs {
            repo: repo.path().to_path_buf(),
            check: false,
            apply: true,
            yes: true,
            self_update: true,
            skip_self: false,
            client_start: false,
            quiet: true,
            channel: DEFAULT_UPDATE_CHANNEL.into(),
            source: "auto".into(),
            offline: false,
            fail_if_outdated: false,
            install_missing: false,
            profile: "rust-ts-postgres".into(),
            level: "full".into(),
            ide: "all".into(),
            score: false,
            score_mode: "standard".into(),
            score_json: "target/jankurai/repo-score.json".into(),
            score_md: "target/jankurai/repo-score.md".into(),
            out: "target/jankurai/update/update-plan.json".into(),
            md: "target/jankurai/update/update-plan.md".into(),
            state: "target/jankurai/update/state.json".into(),
        };
        let plan = UpdatePlan {
            schema_version: UPDATE_SCHEMA_VERSION.into(),
            command: "jankurai update".into(),
            status: "outdated".into(),
            generated_at: now_string(),
            repo_root: repo.path().display().to_string(),
            current_version: current_version(),
            latest_version: Some("999.0.0".into()),
            standard_version: STANDARD_VERSION.into(),
            auditor_version: AUDITOR_VERSION.into(),
            schema_contract_version: SCHEMA_VERSION.into(),
            paper_edition: PAPER_EDITION.into(),
            target_stack_id: TARGET_STACK_ID.into(),
            update_channel: DEFAULT_UPDATE_CHANNEL.into(),
            source: "auto".into(),
            resolved_source: Some(ResolvedSource {
                requested_source: "auto".into(),
                resolved_source: "local".into(),
                source_url: None,
                latest_version: Some("999.0.0".into()),
                install_command: Some(vec![
                    "cargo".into(),
                    "install".into(),
                    "--path".into(),
                    repo.path().join("crates/jankurai").display().to_string(),
                    "--locked".into(),
                    "--force".into(),
                ]),
                install_root: cargo_install_root(),
                reason: "newer crates/jankurai checkout is available locally".into(),
            }),
            offline: false,
            client_start: false,
            self_update_requested: true,
            self_update_available: true,
            install_state: "legacy-install".into(),
            install_manifest_path: "agent/jankurai-install.toml".into(),
            state_path: args.state.clone(),
            plan_path: args.out.clone(),
            md_path: args.md.clone(),
            reexec_command: None,
            post_upgrade_score_command: Some(format!(
                "{} score {} --mode standard --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md",
                std::env::current_exe().unwrap().display(),
                repo.path().display()
            )),
            post_upgrade_score_mode: Some("standard".into()),
            post_upgrade_score_json: Some("target/jankurai/repo-score.json".into()),
            post_upgrade_score_md: Some("target/jankurai/repo-score.md".into()),
            warnings: Vec::new(),
            actions: Vec::new(),
            artifacts: Vec::new(),
        };

        let command = build_self_update_command(repo.path(), &args, &plan).unwrap();

        assert_eq!(command[0], "cargo");
        assert_eq!(command[1], "install");
        assert_eq!(command[2], "--path");
        assert_eq!(
            command[3],
            repo.path().join("crates/jankurai").display().to_string()
        );
        assert!(command.iter().any(|arg| arg == "--locked"));
        assert!(command.iter().any(|arg| arg == "--force"));
        if let Some(root_pos) = command.iter().position(|arg| arg == "--root") {
            assert!(!command[root_pos + 1].ends_with("/bin"));
        }
    }
}
