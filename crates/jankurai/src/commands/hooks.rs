use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct HooksInstallArgs {
    pub repo: PathBuf,
    pub yes: bool,
    pub dry_run: bool,
    pub force: bool,
}

pub fn install(args: HooksInstallArgs) -> Result<()> {
    if args.dry_run && args.yes {
        bail!("use either --dry-run or --yes, not both");
    }
    if !args.dry_run && !args.yes {
        bail!("refusing to install hooks without --yes or --dry-run");
    }

    let git_dir = git_dir(&args.repo)?;
    let hooks_dir = git_dir.join("hooks");
    let jankurai_dir = git_dir.join("jankurai");
    let managed_hooks = [
        ("pre-commit", crate::init::templates::PRE_COMMIT_HOOK),
        (
            "prepare-commit-msg",
            crate::init::templates::PREPARE_COMMIT_MSG_HOOK,
        ),
    ];

    let mut chains = Vec::new();
    for (name, _) in &managed_hooks {
        let hook_path = hooks_dir.join(name);
        let chain = existing_user_hook_backup(&hook_path, &jankurai_dir, name, args.force)?;
        chains.push((name.to_string(), chain));
    }

    if args.dry_run {
        println!("# would write {}", jankurai_dir.join("env").display());
        for (name, chain) in &chains {
            if let Some(chain) = chain {
                println!("# would chain existing {name} hook via {}", chain.display());
            }
            println!("# would write {}", hooks_dir.join(name).display());
        }
        return Ok(());
    }

    fs::create_dir_all(&hooks_dir).with_context(|| format!("create {}", hooks_dir.display()))?;
    fs::create_dir_all(jankurai_dir.join("hooks"))
        .with_context(|| format!("create {}", jankurai_dir.join("hooks").display()))?;

    let mut installed_chains = Vec::new();
    for (name, body) in &managed_hooks {
        let hook_path = hooks_dir.join(name);
        let chain = install_backup(&hook_path, &jankurai_dir, name, args.force)?;
        installed_chains.push((name.to_string(), chain));
        fs::write(&hook_path, body).with_context(|| format!("write {}", hook_path.display()))?;
        make_executable(&hook_path)?;
    }

    write_env(&jankurai_dir, &installed_chains)?;
    println!(
        "{}",
        crate::ui::paint(
            crate::ui::Style::Good,
            format!("installed jankurai hooks in {}", hooks_dir.display()),
            crate::ui::stdout_color_enabled()
        )
    );
    Ok(())
}

fn git_dir(repo: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("run git rev-parse in {}", repo.display()))?;
    if !output.status.success() {
        bail!("hooks install requires an existing git repository");
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(repo.join(path))
    }
}

fn existing_user_hook_backup(
    hook_path: &Path,
    jankurai_dir: &Path,
    name: &str,
    _force: bool,
) -> Result<Option<PathBuf>> {
    if !hook_path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(hook_path).unwrap_or_default();
    if is_managed_hook(&text) {
        return Ok(None);
    }
    Ok(Some(backup_path(jankurai_dir, name)?))
}

fn install_backup(
    hook_path: &Path,
    jankurai_dir: &Path,
    name: &str,
    force: bool,
) -> Result<Option<PathBuf>> {
    let Some(path) = existing_user_hook_backup(hook_path, jankurai_dir, name, force)? else {
        return Ok(None);
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(hook_path, &path)
        .with_context(|| format!("backup {} to {}", hook_path.display(), path.display()))?;
    make_executable(&path)?;
    Ok(Some(path))
}

fn is_managed_hook(text: &str) -> bool {
    text.contains("JANKURAI MANAGED HOOK")
}

fn backup_path(jankurai_dir: &Path, name: &str) -> Result<PathBuf> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(jankurai_dir
        .join("hooks")
        .join(format!("{name}.user.{now}")))
}

fn write_env(jankurai_dir: &Path, chains: &[(String, Option<PathBuf>)]) -> Result<()> {
    fs::create_dir_all(jankurai_dir)?;
    let current_exe = std::env::current_exe().ok();
    let bin = current_exe
        .as_deref()
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "jankurai".into());
    let mut out = String::new();
    out.push_str("# JANKURAI MANAGED ENV\n");
    out.push_str(&format!("JANKURAI_BIN={}\n", shell_quote(&bin)));
    out.push_str("JANKURAI_FALLBACK_BIN='jankurai'\n");
    for (name, chain) in chains {
        let Some(chain) = chain else {
            continue;
        };
        let key = format!(
            "JANKURAI_{}_CHAIN",
            name.replace('-', "_").to_ascii_uppercase()
        );
        out.push_str(&format!(
            "{key}={}\n",
            shell_quote(&chain.to_string_lossy())
        ));
    }
    fs::write(jankurai_dir.join("env"), out)
        .with_context(|| format!("write {}", jankurai_dir.join("env").display()))?;
    Ok(())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}
