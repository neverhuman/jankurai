//! Jankurai Guard — a guarded filesystem that audits every agent file write as a
//! single-file jankurai candidate and forces the agent to see failures in realtime.
//!
//! The guard sits between an AI agent and the real repository. Every write the
//! agent makes is treated as a single-file jankurai candidate: the bytes are
//! audited by `jankurai audit-file` before they are allowed to land in the real
//! repo. On a block the real repo is left untouched, the rejected candidate is
//! quarantined, the file is poisoned with an un-ignorable language-aware error
//! header, a failure report is written, and (under `guard run`) a failure banner
//! is injected into the agent's terminal.
//!
//! Two backends exist. The cross-platform [`watch`] backend operates in-place on
//! the repository and keeps last-good snapshots in the state directory. The
//! Linux-only [`fuse`] backend mounts a guarded view whose mutations are buffered
//! and never touch the backing store until they pass audit.

pub mod audit_client;
pub mod cli;
pub mod commit;
pub mod doctor;
pub mod feedback;
pub mod fuse;
pub mod layout;
pub mod platform;
pub mod poison;
pub mod policy;
pub mod pty;
pub mod state;
pub mod status;
pub mod transaction;
pub mod watch;

pub use audit_client::{
    AuditClient, CliAuditClient, GuardDecision, GuardFinding, MockAuditClient, Verdict,
};
pub use cli::{run, GuardCommand};
pub use feedback::{DenialBus, GuardEvent};
pub use layout::GuardLayout;
pub use platform::HardeningTier;
pub use policy::{GuardPolicy, OnFail};
pub use state::GuardState;
pub use transaction::{
    CandidateFile, CandidateOperation, CommitBoundary, CommitMachine, WriteBuffer,
};

use std::path::PathBuf;

/// Operating mode for the guard. Resolution order is flag > policy file >
/// [`GuardMode::Enforce`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuardMode {
    /// Audit and report, but never alter agent writes.
    Observe,
    /// Audit; on a block revert/poison the offending file.
    Enforce,
    /// Like `enforce`, plus a persisted strict marker that keeps enforcement on
    /// even if the policy file is later relaxed.
    Strict,
}

impl GuardMode {
    /// Parses a mode string, returning a [`GuardError::InvalidPolicy`] for an
    /// unrecognized value.
    pub fn parse(value: &str) -> Result<Self, GuardError> {
        match value {
            "observe" => Ok(Self::Observe),
            "enforce" => Ok(Self::Enforce),
            "strict" => Ok(Self::Strict),
            other => Err(GuardError::InvalidPolicy(format!(
                "unknown mode `{other}` (expected observe|enforce|strict)"
            ))),
        }
    }

    /// Returns the canonical lowercase name of the mode.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Enforce => "enforce",
            Self::Strict => "strict",
        }
    }

    /// Returns `true` when the mode is allowed to modify files on a block.
    pub fn enforces(self) -> bool {
        matches!(self, Self::Enforce | Self::Strict)
    }
}

impl std::fmt::Display for GuardMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for GuardMode {
    type Err = GuardError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// The crate error type. Application-level fallible code uses [`anyhow::Result`];
/// this enum models the failure conditions the guard itself raises.
#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    /// The FUSE backend is not available on this platform / build.
    #[error("FUSE backend unavailable: {0}")]
    FuseUnavailable(String),

    /// A policy file failed to parse or validate.
    #[error("invalid guard policy: {0}")]
    InvalidPolicy(String),

    /// The on-disk guard state could not be read or written.
    #[error("guard state error: {0}")]
    State(String),

    /// The audit engine could not be invoked or its output could not be parsed.
    #[error("audit engine error: {0}")]
    Audit(String),

    /// A candidate write was blocked by audit.
    #[error("write blocked for {path}: {summary}")]
    Blocked {
        /// Repo-relative path of the blocked file.
        path: PathBuf,
        /// One-line summary from the audit decision.
        summary: String,
    },

    /// A required path could not be resolved.
    #[error("layout error: {0}")]
    Layout(String),

    /// An underlying I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience result alias for crate-internal fallible operations.
pub type GuardResult<T> = Result<T, GuardError>;
