//! The dispatch and per-subcommand handlers for the `guard` command-line
//! surface. [`run`] routes a parsed [`GuardCommand`] to one of the `run_*`
//! handlers. Mode resolution is flag > policy file > `enforce`, with a
//! persisted strict marker overriding everything.

use super::args::{
    DoctorArgs, FailuresArgs, GuardCommand, InstallArgs, MountArgs, QuarantineAction,
    QuarantineArgs, RunArgs, StatusArgs, UnmountArgs, WatchArgs,
};
use super::prompt::maybe_show_fuse_prompt;
use crate::audit_client::CliAuditClient;
use crate::layout::GuardLayout;
use crate::policy::GuardPolicy;
use crate::pty::{run_agent, LaunchSpec};
use crate::state::{self, GuardState};
use crate::status::GuardStatus;
use crate::watch::{WatcherBackend, WatcherHandle};
use crate::{doctor::DoctorReport, fuse, GuardMode};
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::sync::Arc;

/// Dispatches a parsed [`GuardCommand`].
pub fn run(cmd: GuardCommand) -> Result<()> {
    maybe_show_fuse_prompt();
    match cmd {
        GuardCommand::Mount(args) => run_mount(args),
        GuardCommand::Run(args) => run_run(args),
        GuardCommand::Watch(args) => run_watch(args),
        GuardCommand::Status(args) => run_status(args),
        GuardCommand::Doctor(args) => run_doctor(args),
        GuardCommand::Install(args) => run_install(args),
        GuardCommand::Unmount(args) => run_unmount(args),
        GuardCommand::Failures(args) => run_failures(args),
        GuardCommand::Quarantine(args) => run_quarantine(args),
    }
}

/// Resolves the effective mode: flag override, then the policy file, then a
/// persisted strict marker forces strict regardless.
fn resolve_mode(layout: &GuardLayout, policy: &GuardPolicy, flag: Option<GuardMode>) -> GuardMode {
    if state::strict_marker_present(layout) {
        return GuardMode::Strict;
    }
    flag.unwrap_or(policy.mode)
}

/// Loads the policy and layout for a watcher-mode command, applying a mode
/// override.
fn prepare_watcher(
    repo: &std::path::Path,
    mode: Option<GuardMode>,
) -> Result<(GuardLayout, GuardPolicy)> {
    let layout = GuardLayout::watcher(repo).context("resolving guard layout")?;
    let mut policy = GuardPolicy::load(&layout.repo_root).context("loading guard policy")?;
    policy.mode = resolve_mode(&layout, &policy, mode);
    Ok((layout, policy))
}

/// `guard mount`: FUSE is Linux-only, so off-Linux this prints guidance and
/// exits without error so callers are not surprised.
fn run_mount(args: MountArgs) -> Result<()> {
    let layout =
        GuardLayout::fuse(&args.repo, &args.mount_point).context("resolving guard layout")?;
    if !fuse::fuse_available() {
        println!(
            "guard mount: the FUSE backend is built only on Linux with the `fuse` feature.\n\
             Use `guard watch {}` for in-place guarding on this platform.",
            args.repo.display()
        );
        return Ok(());
    }
    let mut policy = GuardPolicy::load(&layout.repo_root).context("loading guard policy")?;
    policy.mode = resolve_mode(&layout, &policy, args.mode);
    let audit = Arc::new(CliAuditClient::from_policy(&policy));
    let bus = Arc::new(crate::feedback::DenialBus::new());
    let session = fuse::mount(layout.clone(), policy.clone(), audit, bus)?;
    state::write_pidfile(&layout)?;
    GuardState::new(&layout.repo_id, policy.mode, "fuse").save(&layout)?;
    println!("guard mount: mounted at {}", layout.mount.display());
    if args.foreground {
        // Hold the session open until interrupted.
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }
    session.unmount();
    state::remove_pidfile(&layout)?;
    Ok(())
}

/// `guard run`: launch an agent under the guard.
fn run_run(args: RunArgs) -> Result<()> {
    let (layout, policy) = prepare_watcher(&args.repo, args.mode)?;
    let audit = Arc::new(CliAuditClient::from_policy(&policy));
    let session_id = crate::feedback::now_stamp();
    state::write_pidfile(&layout)?;
    GuardState::new(&layout.repo_id, policy.mode, "watcher").save(&layout)?;
    let spec = LaunchSpec {
        layout: layout.clone(),
        policy,
        agent: args.agent,
        poll: args.poll,
        session_id,
    };
    let result = run_agent(spec, audit);
    let _ = state::remove_pidfile(&layout);
    let session = result?;
    std::process::exit(session.exit_code);
}

/// `guard watch`: guard a repository in place; blocks until interrupted.
fn run_watch(args: WatchArgs) -> Result<()> {
    let (layout, policy) = prepare_watcher(&args.repo, args.mode)?;
    if policy.mode == GuardMode::Strict {
        state::set_strict_marker(&layout)?;
    }
    let audit = Arc::new(CliAuditClient::from_policy(&policy));
    let bus = Arc::new(crate::feedback::DenialBus::new());
    let mut backend = WatcherBackend::new(layout.clone(), policy.clone(), audit, bus)?;
    backend.prime_snapshots()?;
    state::write_pidfile(&layout)?;
    GuardState::new(&layout.repo_id, policy.mode, "watcher").save(&layout)?;
    println!(
        "guard watch: guarding {} in {} mode (Ctrl-C to stop)",
        layout.repo_root.display(),
        policy.mode
    );
    let handle = WatcherHandle::default();
    let result = backend.run(handle, args.poll);
    let _ = state::remove_pidfile(&layout);
    result.map_err(|e| anyhow!(e))
}

/// `guard status`.
fn run_status(args: StatusArgs) -> Result<()> {
    let layout = GuardLayout::watcher(&args.repo).context("resolving guard layout")?;
    let status = GuardStatus::collect(&layout)?;
    if args.json {
        println!("{}", status.render_json()?);
    } else {
        print!("{}", status.render_human());
    }
    Ok(())
}

/// `guard doctor`.
fn run_doctor(args: DoctorArgs) -> Result<()> {
    let layout = GuardLayout::watcher(&args.repo).context("resolving guard layout")?;
    let report = DoctorReport::run(&layout);
    if args.json {
        println!("{}", report.render_json()?);
    } else {
        print!("{}", report.render_human());
    }
    if report.healthy() {
        Ok(())
    } else {
        Err(anyhow!("guard doctor found problems"))
    }
}

/// `guard install`: write the default policy file and the pre-commit hook.
fn run_install(args: InstallArgs) -> Result<()> {
    let layout = GuardLayout::watcher(&args.repo).context("resolving guard layout")?;
    let mut policy = GuardPolicy::default();
    if let Some(mode) = args.mode {
        policy.mode = mode;
    }
    let policy_path = layout.repo_root.join(GuardPolicy::RELATIVE_PATH);
    let policy_toml =
        toml::to_string_pretty(&policy).map_err(|e| anyhow!("serialize default policy: {e}"))?;
    let hook_path = layout
        .repo_root
        .join(".git")
        .join("hooks")
        .join("pre-commit");
    let hook_body = "#!/bin/sh\n# installed by jankurai guard\njankurai-guard status --json >/dev/null 2>&1 || true\n";

    if args.dry_run {
        println!("guard install (dry run):");
        println!("  would write {}", policy_path.display());
        println!("  would write {}", hook_path.display());
        return Ok(());
    }
    if policy_path.exists() && !args.yes {
        return Err(anyhow!(
            "{} already exists; pass --yes to overwrite",
            policy_path.display()
        ));
    }
    if let Some(parent) = policy_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&policy_path, policy_toml)?;
    println!("guard install: wrote {}", policy_path.display());
    if let Some(parent) = hook_path.parent() {
        if parent.exists() {
            std::fs::write(&hook_path, hook_body)?;
            set_executable(&hook_path)?;
            println!("guard install: wrote {}", hook_path.display());
        } else {
            println!(
                "guard install: skipped git hook ({} absent)",
                parent.display()
            );
        }
    }
    Ok(())
}

/// Makes a file user-executable.
fn set_executable(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

/// `guard unmount`.
fn run_unmount(args: UnmountArgs) -> Result<()> {
    let layout = GuardLayout::watcher(&args.repo).context("resolving guard layout")?;
    if !fuse::fuse_available() {
        println!("guard unmount: no FUSE backend on this platform; nothing to unmount");
    }
    match state::read_pidfile(&layout) {
        Some(pid) if state::pid_is_live(pid) => {
            // Signal the daemon to tear the mount down.
            // SAFETY: `pid` is live (confirmed by `pid_is_live` signal-0 check above).
            // SIGTERM is always safe to deliver to our own process; ESRCH is benign.
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGTERM);
            }
            println!("guard unmount: signalled daemon pid {pid}");
        }
        _ => println!("guard unmount: no live guard daemon recorded"),
    }
    state::remove_pidfile(&layout)?;
    Ok(())
}

/// `guard failures`: show recorded failure reports.
fn run_failures(args: FailuresArgs) -> Result<()> {
    let layout = GuardLayout::watcher(&args.repo).context("resolving guard layout")?;
    let guard_dir = layout.guard_artifacts_dir();
    if args.last {
        let last = guard_dir.join("LAST_FAILURE.md");
        match std::fs::read_to_string(&last) {
            Ok(text) => print!("{text}"),
            Err(_) => println!("guard failures: no failures recorded"),
        }
        return Ok(());
    }
    let failures_dir = guard_dir.join("failures");
    let mut entries: Vec<PathBuf> = match std::fs::read_dir(&failures_dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
            .collect(),
        Err(_) => Vec::new(),
    };
    entries.sort();
    if args.json {
        let mut docs = Vec::new();
        for path in &entries {
            if let Ok(text) = std::fs::read_to_string(path) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    docs.push(value);
                }
            }
        }
        println!("{}", serde_json::to_string_pretty(&docs)?);
    } else if entries.is_empty() {
        println!("guard failures: no failures recorded");
    } else {
        println!("guard failures: {} recorded", entries.len());
        for path in &entries {
            let md = path.with_extension("md");
            if let Ok(text) = std::fs::read_to_string(&md) {
                if let Some(first) = text.lines().next() {
                    println!("  {first}");
                }
            }
        }
    }
    Ok(())
}

/// `guard quarantine`.
fn run_quarantine(args: QuarantineArgs) -> Result<()> {
    match args.action {
        QuarantineAction::List { repo } => run_quarantine_list(&repo),
        QuarantineAction::Restore { repo, path } => run_quarantine_restore(&repo, &path),
    }
}

/// `guard quarantine list`.
fn run_quarantine_list(repo: &std::path::Path) -> Result<()> {
    let layout = GuardLayout::watcher(repo).context("resolving guard layout")?;
    match GuardState::load(&layout)? {
        Some(state) if !state.quarantined.is_empty() => {
            println!("quarantined candidates: {}", state.quarantined.len());
            for entry in &state.quarantined {
                println!(
                    "  {} -> {} ({})",
                    entry.rel_path.display(),
                    entry.quarantine_path.display(),
                    entry.blocked_at
                );
            }
        }
        _ => println!("guard quarantine: nothing quarantined"),
    }
    Ok(())
}

/// `guard quarantine restore`.
fn run_quarantine_restore(repo: &std::path::Path, path: &std::path::Path) -> Result<()> {
    let layout = GuardLayout::watcher(repo).context("resolving guard layout")?;
    let state = match GuardState::load(&layout)? {
        Some(s) => s,
        None => return Err(anyhow!("no guard state recorded for this repository")),
    };
    let entry = match state.quarantined.iter().rev().find(|e| e.rel_path == path) {
        Some(e) => e,
        None => return Err(anyhow!("{} is not quarantined", path.display())),
    };
    let bytes = std::fs::read(&entry.quarantine_path)
        .with_context(|| format!("reading {}", entry.quarantine_path.display()))?;
    let target = layout.repo_root.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, bytes)?;
    println!(
        "guard quarantine: restored {} into the working tree",
        path.display()
    );
    Ok(())
}
