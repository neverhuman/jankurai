//! The `guard` command-line surface. [`GuardCommand`] is a `clap` subcommand
//! enum the main `jankurai` binary can embed directly; [`run`] dispatches it.
//! Mode resolution is flag > policy file > `enforce`.
//!
//! The argument structs live in [`args`] and the per-subcommand handlers in
//! [`handlers`]; this module just re-exports the public surface.

pub mod args;
pub mod handlers;
pub mod prompt;

pub use args::{
    DoctorArgs, FailuresArgs, GuardCommand, InstallArgs, MountArgs, QuarantineAction,
    QuarantineArgs, RunArgs, StatusArgs, UnmountArgs, WatchArgs,
};
pub use handlers::run;
