//! The `clap` argument surface for the `guard` subcommand. [`GuardCommand`] is
//! the subcommand enum the main `jankurai` binary embeds directly; the `*Args`
//! structs carry each subcommand's options.

use crate::GuardMode;
use clap::{Args, Subcommand};
use std::ffi::OsString;
use std::path::PathBuf;

/// The `guard` subcommand surface.
#[derive(Debug, Subcommand)]
pub enum GuardCommand {
    /// Mount a guarded FUSE view of a repository (Linux only).
    Mount(MountArgs),
    /// Run an agent under the guard, injecting realtime audit feedback.
    Run(RunArgs),
    /// Guard a repository in place using the filesystem watcher backend.
    Watch(WatchArgs),
    /// Report the live status of a guarded repository.
    Status(StatusArgs),
    /// Run environment preflight checks for the guard.
    Doctor(DoctorArgs),
    /// Install the guard's policy file and git hooks into a repository.
    Install(InstallArgs),
    /// Unmount a guarded FUSE view (Linux only).
    Unmount(UnmountArgs),
    /// Show recorded guard failures for a repository.
    Failures(FailuresArgs),
    /// Inspect and restore quarantined candidates.
    Quarantine(QuarantineArgs),
}

/// Arguments for `guard mount`.
#[derive(Debug, Args)]
pub struct MountArgs {
    /// Repository to guard.
    #[arg(default_value = ".")]
    pub repo: PathBuf,
    /// Operating mode override.
    #[arg(long)]
    pub mode: Option<GuardMode>,
    /// Mount point for the guarded view.
    #[arg(long, default_value = "mnt/jankurai-guard")]
    pub mount_point: PathBuf,
    /// Stay in the foreground instead of daemonizing.
    #[arg(long)]
    pub foreground: bool,
}

/// Arguments for `guard run`.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Repository to guard.
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,
    /// Operating mode override.
    #[arg(long)]
    pub mode: Option<GuardMode>,
    /// Use PollWatcher when inotify is unavailable.
    #[arg(long)]
    pub poll: bool,
    /// The agent command to run, given after a trailing `--`.
    #[arg(trailing_var_arg = true, required = true)]
    pub agent: Vec<OsString>,
}

/// Arguments for `guard watch`.
#[derive(Debug, Args)]
pub struct WatchArgs {
    /// Repository to guard.
    #[arg(default_value = ".")]
    pub repo: PathBuf,
    /// Operating mode override.
    #[arg(long)]
    pub mode: Option<GuardMode>,
    /// Use PollWatcher when inotify is unavailable.
    #[arg(long)]
    pub poll: bool,
}

/// Arguments for `guard status`.
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Repository to inspect.
    #[arg(default_value = ".")]
    pub repo: PathBuf,
    /// Emit JSON instead of human-readable output.
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `guard doctor`.
#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Repository to inspect.
    #[arg(default_value = ".")]
    pub repo: PathBuf,
    /// Emit JSON instead of human-readable output.
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `guard install`.
#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Repository to install into.
    #[arg(default_value = ".")]
    pub repo: PathBuf,
    /// Proceed without prompting.
    #[arg(long)]
    pub yes: bool,
    /// Describe what would be done without writing anything.
    #[arg(long)]
    pub dry_run: bool,
    /// Default mode to write into the policy file.
    #[arg(long)]
    pub mode: Option<GuardMode>,
}

/// Arguments for `guard unmount`.
#[derive(Debug, Args)]
pub struct UnmountArgs {
    /// Repository whose guarded view should be unmounted.
    #[arg(default_value = ".")]
    pub repo: PathBuf,
}

/// Arguments for `guard failures`.
#[derive(Debug, Args)]
pub struct FailuresArgs {
    /// Repository to inspect.
    #[arg(default_value = ".")]
    pub repo: PathBuf,
    /// Show only the most recent failure.
    #[arg(long)]
    pub last: bool,
    /// Emit JSON instead of human-readable output.
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `guard quarantine`.
#[derive(Debug, Args)]
pub struct QuarantineArgs {
    /// The quarantine action to perform.
    #[command(subcommand)]
    pub action: QuarantineAction,
}

/// Sub-actions of `guard quarantine`.
#[derive(Debug, Subcommand)]
pub enum QuarantineAction {
    /// List quarantined candidates.
    List {
        /// Repository to inspect.
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// Restore a quarantined candidate back into the working tree.
    Restore {
        /// Repository to restore into.
        #[arg(default_value = ".")]
        repo: PathBuf,
        /// Repo-relative path of the candidate to restore.
        path: PathBuf,
    },
}
