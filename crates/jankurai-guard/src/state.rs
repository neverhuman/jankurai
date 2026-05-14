//! On-disk guard state. The guard persists a small JSON document plus a few
//! marker files under `state/<repo-id>/`:
//!
//! - `guard-state.json` — the [`GuardState`] document (mode, started-at, the
//!   quarantine index, the blocked-file list),
//! - `guard.pid` — the daemon pidfile,
//! - `STRICT_MODE` — a marker that keeps strict enforcement on across restarts.

use crate::feedback::now_rfc3339;
use crate::layout::GuardLayout;
use crate::{GuardError, GuardMode};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// A single entry in the quarantine index: a candidate that was rejected and
/// copied into the repo's `.jankurai/guard/rejected/` tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantineEntry {
    /// Repo-relative path of the rejected candidate.
    pub rel_path: PathBuf,
    /// Where the rejected bytes were quarantined.
    pub quarantine_path: PathBuf,
    /// Where the failure report was written.
    pub report_path: PathBuf,
    /// RFC 3339 timestamp of the block.
    pub blocked_at: String,
}

/// The persisted guard-state document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardState {
    /// Schema version of the state document.
    #[serde(default = "default_state_version")]
    pub version: String,
    /// Stable identifier of the guarded repository.
    pub repo_id: String,
    /// Operating mode the daemon is running in.
    pub mode: GuardMode,
    /// RFC 3339 timestamp the session started.
    pub started_at: String,
    /// Backend in use: `watcher` or `fuse`.
    pub backend: String,
    /// The quarantine index, newest last.
    #[serde(default)]
    pub quarantined: Vec<QuarantineEntry>,
    /// Repo-relative paths currently blocked (have an active poison view).
    #[serde(default)]
    pub blocked_paths: Vec<PathBuf>,
}

fn default_state_version() -> String {
    "1".to_string()
}

impl GuardState {
    /// Creates a fresh state document for a new session.
    pub fn new(repo_id: &str, mode: GuardMode, backend: &str) -> Self {
        Self {
            version: default_state_version(),
            repo_id: repo_id.to_string(),
            mode,
            started_at: now_rfc3339(),
            backend: backend.to_string(),
            quarantined: Vec::new(),
            blocked_paths: Vec::new(),
        }
    }

    /// Loads the state document for `layout`, returning `None` when no session
    /// state exists on disk.
    pub fn load(layout: &GuardLayout) -> Result<Option<Self>, GuardError> {
        let path = layout.state_file();
        match fs::read_to_string(&path) {
            Ok(text) => {
                let state = serde_json::from_str(&text)
                    .map_err(|e| GuardError::State(format!("{}: {e}", path.display())))?;
                Ok(Some(state))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(GuardError::State(format!("{}: {e}", path.display()))),
        }
    }

    /// Persists the state document for `layout`.
    pub fn save(&self, layout: &GuardLayout) -> Result<(), GuardError> {
        fs::create_dir_all(&layout.state)?;
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| GuardError::State(format!("serialize state: {e}")))?;
        fs::write(layout.state_file(), text)?;
        Ok(())
    }

    /// Records a quarantine entry and adds the path to the blocked list.
    pub fn record_quarantine(&mut self, entry: QuarantineEntry) {
        if !self.blocked_paths.contains(&entry.rel_path) {
            self.blocked_paths.push(entry.rel_path.clone());
        }
        self.quarantined.push(entry);
    }

    /// Removes a path from the blocked list (the block was cleared).
    pub fn clear_blocked(&mut self, rel_path: &Path) {
        self.blocked_paths.retain(|p| p != rel_path);
    }
}

/// Writes the current process id into `layout`'s pidfile.
pub fn write_pidfile(layout: &GuardLayout) -> Result<(), GuardError> {
    fs::create_dir_all(&layout.state)?;
    fs::write(layout.pidfile(), std::process::id().to_string())?;
    Ok(())
}

/// Reads the daemon pid from `layout`'s pidfile, returning `None` when the
/// pidfile is absent or unparseable.
pub fn read_pidfile(layout: &GuardLayout) -> Option<u32> {
    let text = fs::read_to_string(layout.pidfile()).ok()?;
    text.trim().parse().ok()
}

/// Removes `layout`'s pidfile if it exists.
pub fn remove_pidfile(layout: &GuardLayout) -> Result<(), GuardError> {
    match fs::remove_file(layout.pidfile()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(GuardError::Io(e)),
    }
}

/// Returns `true` when a process with `pid` appears to be alive.
pub fn pid_is_live(pid: u32) -> bool {
    // Signal 0 checks existence without delivering a signal: EPERM means alive.
    // SAFETY: kill(pid, 0) is defined POSIX — no signal is delivered; ESRCH/EPERM
    // are the only observable effects.  The u32→pid_t cast is sound: pid_t is at
    // least as wide as u32, and the pid came from a file this process wrote.
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if result == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Writes the strict-mode marker for `layout`.
pub fn set_strict_marker(layout: &GuardLayout) -> Result<(), GuardError> {
    fs::create_dir_all(&layout.state)?;
    fs::write(layout.strict_marker(), now_rfc3339())?;
    Ok(())
}

/// Clears the strict-mode marker for `layout`.
pub fn clear_strict_marker(layout: &GuardLayout) -> Result<(), GuardError> {
    match fs::remove_file(layout.strict_marker()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(GuardError::Io(e)),
    }
}

/// Returns `true` when the strict-mode marker is present.
pub fn strict_marker_present(layout: &GuardLayout) -> bool {
    layout.strict_marker().exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn layout_in(dir: &Path) -> GuardLayout {
        GuardLayout {
            repo_root: dir.to_path_buf(),
            mount: dir.to_path_buf(),
            backing: dir.to_path_buf(),
            state: dir.join("state"),
            repo_id: "test-0000".to_string(),
        }
    }

    #[test]
    fn state_roundtrips() {
        let dir = tempdir().unwrap();
        let layout = layout_in(dir.path());
        let state = GuardState::new("test-0000", GuardMode::Enforce, "watcher");
        state.save(&layout).unwrap();
        let loaded = GuardState::load(&layout).unwrap().unwrap();
        assert_eq!(loaded.repo_id, "test-0000");
        assert_eq!(loaded.mode, GuardMode::Enforce);
    }

    #[test]
    fn pidfile_and_strict_marker() {
        let dir = tempdir().unwrap();
        let layout = layout_in(dir.path());
        write_pidfile(&layout).unwrap();
        assert_eq!(read_pidfile(&layout), Some(std::process::id()));
        assert!(pid_is_live(std::process::id()));
        set_strict_marker(&layout).unwrap();
        assert!(strict_marker_present(&layout));
        clear_strict_marker(&layout).unwrap();
        assert!(!strict_marker_present(&layout));
    }
}
