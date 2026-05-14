//! Guard policy: the `agent/guard-policy.toml` schema, its defaults, loading and
//! validation. The policy controls which findings block, the debounce timings,
//! what happens on a block, and the platform hardening tiers.

use crate::{GuardError, GuardMode};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Action taken when audit blocks a candidate write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnFail {
    /// Only emit a warning; do not touch files.
    Warn,
    /// Emit a warning and interrupt the agent (banner injection).
    Interrupt,
    /// Revert the file to its last-good snapshot.
    Revert,
    /// Overwrite the file with a poison payload.
    Poison,
}

/// Which control files are included when auditing a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuditScope {
    /// Audit only the single changed file.
    FileOnly,
    /// Audit the changed file plus relevant control files.
    FilePlusControl,
}

/// Extra path-exclusion configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathPolicy {
    /// Path prefixes excluded from auditing, in addition to the hard-excluded
    /// `.jankurai/` and `target/jankurai/` prefixes.
    #[serde(default = "default_excluded_paths")]
    pub extra_excluded_paths: Vec<String>,
}

/// Platform hardening tier toggles. Each tier is feature-gated and ships as a
/// documented no-op in this release; the toggles record operator intent. The
/// derived `Default` leaves every tier off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HardeningPolicy {
    /// Enable Linux Landlock restrictions (requires the `landlock` feature).
    #[serde(default)]
    pub linux_landlock: bool,
    /// Enable Linux fanotify monitoring (requires the `fanotify` feature).
    #[serde(default)]
    pub linux_fanotify: bool,
    /// Enable macOS Endpoint Security probing (Tier 3, deferred).
    #[serde(default)]
    pub macos_endpoint_security: bool,
}

/// The full guard policy as parsed from `agent/guard-policy.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardPolicy {
    /// Schema version of the policy file.
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    /// Operating mode.
    #[serde(default = "default_mode")]
    pub mode: GuardMode,
    /// Severities that cause a block.
    #[serde(default = "default_block_on")]
    pub block_on: Vec<String>,
    /// Severities that produce a warning only.
    #[serde(default = "default_warn_on")]
    pub warn_on: Vec<String>,
    /// Per-path debounce window in milliseconds.
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    /// Stability window in milliseconds (size + mtime must be stable).
    #[serde(default = "default_stable_ms")]
    pub stable_ms: u64,
    /// Action taken on a block.
    #[serde(default = "default_on_fail")]
    pub on_fail: OnFail,
    /// Whether brand-new files are quarantined on a block.
    #[serde(default = "default_true")]
    pub quarantine_new_files: bool,
    /// Whether `.gitignore` entries are excluded from auditing.
    #[serde(default = "default_true")]
    pub respect_gitignore: bool,
    /// Which control files are folded into the audit.
    #[serde(default = "default_audit_scope")]
    pub audit_scope: AuditScope,
    /// Treat an unavailable audit engine as a block.
    #[serde(default = "default_true")]
    pub fail_closed: bool,
    /// Maximum time the audit engine is allowed before it is treated as failed.
    #[serde(default = "default_max_audit_ms")]
    pub max_audit_ms: u64,
    /// Path-exclusion configuration.
    #[serde(default)]
    pub paths: PathPolicy,
    /// Platform hardening configuration.
    #[serde(default)]
    pub hardening: HardeningPolicy,
}

fn default_schema_version() -> String {
    "1.0.0".to_string()
}
fn default_mode() -> GuardMode {
    GuardMode::Enforce
}
fn default_block_on() -> Vec<String> {
    vec!["critical".to_string(), "high".to_string()]
}
fn default_warn_on() -> Vec<String> {
    vec!["medium".to_string(), "low".to_string()]
}
fn default_debounce_ms() -> u64 {
    150
}
fn default_stable_ms() -> u64 {
    75
}
fn default_on_fail() -> OnFail {
    OnFail::Poison
}
fn default_true() -> bool {
    true
}
fn default_audit_scope() -> AuditScope {
    AuditScope::FilePlusControl
}
fn default_max_audit_ms() -> u64 {
    4000
}
fn default_excluded_paths() -> Vec<String> {
    vec![
        ".jankurai/".to_string(),
        "target/jankurai/".to_string(),
        "target/".to_string(),
        "node_modules/".to_string(),
        ".git/".to_string(),
    ]
}

impl Default for PathPolicy {
    fn default() -> Self {
        Self {
            extra_excluded_paths: default_excluded_paths(),
        }
    }
}

impl Default for GuardPolicy {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            mode: default_mode(),
            block_on: default_block_on(),
            warn_on: default_warn_on(),
            debounce_ms: default_debounce_ms(),
            stable_ms: default_stable_ms(),
            on_fail: default_on_fail(),
            quarantine_new_files: true,
            respect_gitignore: true,
            audit_scope: default_audit_scope(),
            fail_closed: true,
            max_audit_ms: default_max_audit_ms(),
            paths: PathPolicy::default(),
            hardening: HardeningPolicy::default(),
        }
    }
}

impl GuardPolicy {
    /// Repo-relative location of the policy file.
    pub const RELATIVE_PATH: &'static str = "agent/guard-policy.toml";

    /// Loads the policy from `<repo>/agent/guard-policy.toml`, returning
    /// [`GuardPolicy::default`] when the file is absent. A present-but-invalid
    /// file is a [`GuardError::InvalidPolicy`].
    pub fn load(repo: &Path) -> Result<Self, GuardError> {
        let path = repo.join(Self::RELATIVE_PATH);
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let policy: GuardPolicy = toml::from_str(&text)
                    .map_err(|e| GuardError::InvalidPolicy(format!("{}: {e}", path.display())))?;
                policy.validate()?;
                Ok(policy)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(GuardError::State(format!("{}: {e}", path.display()))),
        }
    }

    /// Validates the policy invariants: timings must be non-zero-bounded and the
    /// severity lists must contain only known severity names.
    pub fn validate(&self) -> Result<(), GuardError> {
        const SEVERITIES: [&str; 4] = ["critical", "high", "medium", "low"];
        for list in [&self.block_on, &self.warn_on] {
            for sev in list {
                if !SEVERITIES.contains(&sev.as_str()) {
                    return Err(GuardError::InvalidPolicy(format!(
                        "unknown severity `{sev}` (expected critical|high|medium|low)"
                    )));
                }
            }
        }
        if self.max_audit_ms == 0 {
            return Err(GuardError::InvalidPolicy(
                "max_audit_ms must be greater than zero".to_string(),
            ));
        }
        if self.schema_version.trim().is_empty() {
            return Err(GuardError::InvalidPolicy(
                "schema_version must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Returns `true` when `rel_path` is excluded from auditing. The
    /// `.jankurai/` and `target/jankurai/` prefixes are always excluded so the
    /// guard's own report writes never re-trigger an audit.
    pub fn is_excluded(&self, rel_path: &str) -> bool {
        let normalized = rel_path.trim_start_matches("./");
        const HARD: [&str; 2] = [".jankurai/", "target/jankurai/"];
        for prefix in HARD {
            if normalized == prefix.trim_end_matches('/') || normalized.starts_with(prefix) {
                return true;
            }
        }
        for prefix in &self.paths.extra_excluded_paths {
            let trimmed = prefix.trim_end_matches('/');
            if !trimmed.is_empty()
                && (normalized == trimmed || normalized.starts_with(&format!("{trimmed}/")))
            {
                return true;
            }
        }
        false
    }

    /// Returns `true` when a finding of `severity` should block a write.
    pub fn blocks(&self, severity: &str) -> bool {
        self.block_on.iter().any(|s| s == severity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid_and_enforce() {
        let p = GuardPolicy::default();
        p.validate().unwrap();
        assert_eq!(p.mode, GuardMode::Enforce);
        assert_eq!(p.on_fail, OnFail::Poison);
        assert!(p.fail_closed);
    }

    #[test]
    fn hard_exclusions_always_apply() {
        let p = GuardPolicy::default();
        assert!(p.is_excluded(".jankurai/guard/LAST_FAILURE.md"));
        assert!(p.is_excluded("target/jankurai/x"));
        assert!(p.is_excluded("./.git/HEAD"));
        assert!(!p.is_excluded("src/main.rs"));
    }
}
