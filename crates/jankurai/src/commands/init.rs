use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::init::profiles::MergePolicyAction;
use crate::validation::{self, ArtifactSchema};

pub struct InitArgs {
    pub repo: PathBuf,
    pub apply: bool,
    pub dry_run: bool,
    pub yes: bool,
    pub profile: String,
    /// When set, load profile manifest from this JSON file (`InitProfile` schema); `--profile` is ignored for resolution.
    pub profile_file: Option<PathBuf>,
    pub level: String,
    pub ide: String,
    pub mode: String,
    pub diff: bool,
    pub ci: String,
    pub issue_backend: String,
    pub ux_qa: bool,
    pub plan_json: Option<String>,
    pub force_generated_adapters: bool,
}

pub fn run(args: InitArgs) -> Result<()> {
    let plan = crate::init::plan::build_plan(
        &args.repo,
        &args.profile,
        args.profile_file.as_deref(),
        &args.level,
        &args.ide,
        &args.mode,
        &args.ci,
        &args.issue_backend,
        args.ux_qa,
    )?;
    if let Some(path) = args.plan_json.as_deref() {
        if args.dry_run || args.diff {
            if let Some(parent) = Path::new(path).parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, serde_json::to_string_pretty(&plan)?)?;
        } else {
            bail!("--plan-json only writes during --dry-run or --diff");
        }
    }
    println!("{}", crate::init::plan::render_plan(&plan));
    if args.dry_run || args.diff {
        if args.diff {
            print_diff(&args.repo, &plan.profile_manifest, &plan.level);
        }
        println!(
            "{}",
            crate::init::plan::render_next_steps(&plan, false, None, &args.repo)
        );
        return Ok(());
    }
    if !(args.apply || args.yes) {
        bail!("refusing to write without --yes or --apply");
    }
    let actions = apply_templates(
        &args.repo,
        &plan.profile_manifest,
        &plan.level,
        args.force_generated_adapters,
    )?;
    let receipt = write_receipt(&args.repo, "init", &actions)?;
    println!(
        "{}",
        crate::init::plan::render_next_steps(&plan, true, Some(&receipt), &args.repo)
    );
    Ok(())
}

fn apply_templates(
    repo: &Path,
    manifest: &crate::init::profiles::ProfileManifest,
    level: &str,
    force_generated_adapters: bool,
) -> Result<Vec<InitAction>> {
    let cargo_repo = repo.join("Cargo.toml").exists();
    let mut paths = manifest.generated_paths.clone();
    paths.sort();
    let mut actions = vec![];
    let progress = crate::ui::CliProgress::new("installing scaffold", paths.len() as u64);
    for rel in paths {
        progress.tick(format!("apply {rel}"));
        let template = crate::init::templates::template_for_path(&rel)
            .with_context(|| format!("no template registered for profile path `{rel}`"))?;
        let body =
            crate::init::templates::body_for_path(&rel, level, cargo_repo).unwrap_or(template.body);
        let path = repo.join(&rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        ensure_automated_write_allowed(&rel)?;
        if path.exists() {
            if crate::init::adapters::is_adapter_path(&rel)
                && existing_generated_adapter_needs_write(
                    &rel,
                    &fs::read_to_string(&path).unwrap_or_default(),
                    force_generated_adapters,
                )
            {
                fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
                actions.push(InitAction {
                    path: rel,
                    action: "overwrote-generated-adapter".into(),
                });
                continue;
            }
            let existing_text = fs::read_to_string(&path).unwrap_or_default();

            match manifest.merge_policy_for_path(&rel) {
                MergePolicyAction::MergeJson => {
                    let merged = crate::init::merge::merge_json(&existing_text, body)
                        .with_context(|| format!("failed to merge JSON {}", rel))?;
                    if merged != existing_text {
                        fs::write(&path, merged)?;
                    }
                    actions.push(InitAction {
                        path: rel,
                        action: "merge-json".into(),
                    });
                }
                MergePolicyAction::MergeToml => {
                    let merged = if rel == "agent/standard-version.toml" {
                        crate::init::merge::merge_standard_version_toml(&existing_text, body)
                    } else {
                        crate::init::merge::merge_toml(&existing_text, body)
                    }
                    .with_context(|| format!("failed to merge TOML {}", rel))?;
                    if merged != existing_text {
                        fs::write(&path, merged)?;
                    }
                    actions.push(InitAction {
                        path: rel,
                        action: "merge-toml".into(),
                    });
                }
                MergePolicyAction::MergeLines => {
                    let merged = crate::init::merge::merge_lines(&existing_text, body)
                        .with_context(|| format!("failed to merge lines {}", rel))?;
                    if merged != existing_text {
                        fs::write(&path, merged)?;
                    }
                    actions.push(InitAction {
                        path: rel,
                        action: "merge-lines".into(),
                    });
                }
                MergePolicyAction::MergeMarker => {
                    if is_jankurai_controlled(&existing_text) {
                        actions.push(InitAction {
                            path: rel,
                            action: "kept-existing".into(),
                        });
                    } else {
                        append_merge_marker(&path, &rel, &existing_text)?;
                        actions.push(InitAction {
                            path: rel,
                            action: "merge-marker".into(),
                        });
                    }
                }
                MergePolicyAction::KeepExisting => {
                    actions.push(InitAction {
                        path: rel,
                        action: "kept-existing".into(),
                    });
                }
            }
            continue;
        }
        fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
        actions.push(InitAction {
            path: rel,
            action: "created".into(),
        });
    }
    progress.finish("init complete");
    Ok(actions)
}

fn ensure_automated_write_allowed(rel: &str) -> Result<()> {
    if crate::audit::fs::is_read_only_exception_path(rel) {
        bail!("automated writes to docs/exceptions are blocked; edit the exception file manually");
    }
    Ok(())
}

fn existing_generated_adapter_needs_write(
    rel: &str,
    existing_text: &str,
    force_generated_adapters: bool,
) -> bool {
    existing_text.contains(crate::init::adapters::GENERATED_MARKER)
        && (force_generated_adapters
            || crate::init::adapters::needs_generated_skill_repair(rel, existing_text))
}

fn print_diff(repo: &Path, manifest: &crate::init::profiles::ProfileManifest, level: &str) {
    let cargo_repo = repo.join("Cargo.toml").exists();
    let mut paths = manifest.generated_paths.clone();
    paths.sort();
    for rel in paths {
        let Some(template) = crate::init::templates::template_for_path(&rel) else {
            println!("--- {} missing template", rel);
            continue;
        };
        let body =
            crate::init::templates::body_for_path(&rel, level, cargo_repo).unwrap_or(template.body);
        let path_obj = repo.join(&rel);
        if path_obj.exists() {
            let existing = fs::read_to_string(&path_obj).unwrap_or_default();
            let merged = match manifest.merge_policy_for_path(&rel) {
                MergePolicyAction::MergeJson => {
                    crate::init::merge::merge_json(&existing, body).ok()
                }
                MergePolicyAction::MergeToml => {
                    if rel == "agent/standard-version.toml" {
                        crate::init::merge::merge_standard_version_toml(&existing, body).ok()
                    } else {
                        crate::init::merge::merge_toml(&existing, body).ok()
                    }
                }
                MergePolicyAction::MergeLines => {
                    crate::init::merge::merge_lines(&existing, body).ok()
                }
                MergePolicyAction::MergeMarker if !is_jankurai_controlled(&existing) => {
                    let marker = crate::init::merge::merge_marker(&rel);
                    if existing.contains("jankurai merge marker") {
                        None
                    } else {
                        Some(format!("{existing}{marker}"))
                    }
                }
                MergePolicyAction::MergeMarker | MergePolicyAction::KeepExisting => None,
            };

            if let Some(m) = merged {
                if m != existing {
                    println!("--- {}", rel);
                    println!("+++ {} (merged view)", rel);
                    println!(
                        "(Run without --diff to see actual merged results; diff output omitted)"
                    );
                } else {
                    println!("--- {} exists; no merge needed", rel);
                }
            } else {
                println!("--- {} exists; no overwrite", rel);
            }
        } else {
            println!("--- /dev/null");
            println!("+++ {}", rel);
            for line in body.lines() {
                println!("+{line}");
            }
        }
    }
}

fn is_jankurai_controlled(text: &str) -> bool {
    text.contains("agent/JANKURAI_STANDARD.md")
        || text.contains("jankurai Standard Agent Bootstrap")
        || (text.contains("Standard version:") && text.to_ascii_lowercase().contains("jankurai"))
}

fn append_merge_marker(path: &Path, rel: &str, text: &str) -> Result<()> {
    let marker = crate::init::merge::merge_marker(rel);
    if text.contains("jankurai merge marker") {
        return Ok(());
    }
    fs::write(path, format!("{text}{marker}"))?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct InitAction {
    path: String,
    action: String,
}

fn write_receipt(repo: &Path, action: &str, actions: &[InitAction]) -> Result<PathBuf> {
    let dir = repo.join("target/jankurai/receipts");
    fs::create_dir_all(&dir)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = dir.join(format!("{action}-{now}.json"));
    let payload = serde_json::json!({
        "action": action,
        "created_at": now,
        "actions": actions,
    });
    validation::validate_value(repo, ArtifactSchema::InitReceipt, &payload)?;
    fs::write(&path, serde_json::to_string_pretty(&payload)?)?;
    Ok(path)
}
