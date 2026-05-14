//! The `guard run` PTY supervisor. The guard launches the agent under a
//! pseudo-terminal it owns, so it can both relay the agent's I/O transparently
//! and inject realtime failure banners inline into the agent's terminal.
//!
//! For this release the supervisor runs the cross-platform [`crate::watch`]
//! backend in a background thread (the FUSE mount is the Linux-only upgrade), so
//! `guard run` works on every platform: it watches the repo, spawns the agent,
//! and injects feedback.

pub mod injector;
pub mod launcher;

pub use injector::BannerInjector;
pub use launcher::{run_agent, AgentSession, LaunchSpec};
