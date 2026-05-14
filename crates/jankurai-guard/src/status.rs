//! `guard status`: reports the live state of a guarded repository — backend
//! mode, daemon liveness, mount presence, the list of currently-blocked files —
//! in either a human-readable or `--json` form.

use crate::layout::GuardLayout;
use crate::poison::PoisonState;
use crate::state::{self, GuardState};
use crate::GuardError;
use serde::Serialize;
use std::path::PathBuf;

/// A point-in-time status snapshot of a guarded repository.
#[derive(Debug, Clone, Serialize)]
pub struct GuardStatus {
    /// Stable identifier of the guarded repository.
    pub repo_id: String,
    /// Whether a guard session's state document exists on disk.
    pub session_present: bool,
    /// The backend recorded in the state document, if any.
    pub backend: Option<String>,
    /// The mode recorded in the state document, if any.
    pub mode: Option<String>,
    /// When the recorded session started, if any.
    pub started_at: Option<String>,
    /// The daemon pid from the pidfile, if present.
    pub daemon_pid: Option<u32>,
    /// Whether that daemon process appears to be alive.
    pub daemon_live: bool,
    /// Whether the strict-mode marker is present.
    pub strict_marker: bool,
    /// Whether the mount path currently exists.
    pub mount_present: bool,
    /// Repo-relative paths with an active poison view.
    pub blocked_paths: Vec<PathBuf>,
    /// Number of quarantined candidates recorded this session.
    pub quarantined_count: usize,
}

impl GuardStatus {
    /// Collects the status for `layout`.
    pub fn collect(layout: &GuardLayout) -> Result<Self, GuardError> {
        let state = GuardState::load(layout)?;
        let poison = PoisonState::load(&layout.poison_dir())?;
        let daemon_pid = state::read_pidfile(layout);
        let daemon_live = daemon_pid.map(state::pid_is_live).unwrap_or(false);

        let (session_present, backend, mode, started_at, quarantined_count) = match &state {
            Some(s) => (
                true,
                Some(s.backend.clone()),
                Some(s.mode.to_string()),
                Some(s.started_at.clone()),
                s.quarantined.len(),
            ),
            None => (false, None, None, None, 0),
        };

        Ok(Self {
            repo_id: layout.repo_id.clone(),
            session_present,
            backend,
            mode,
            started_at,
            daemon_pid,
            daemon_live,
            strict_marker: state::strict_marker_present(layout),
            mount_present: layout.mount.exists(),
            blocked_paths: poison.poisoned_paths(),
            quarantined_count,
        })
    }

    /// Renders the status as a compact human-readable block.
    pub fn render_human(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("guard status for {}\n", self.repo_id));
        out.push_str(&format!(
            "  session : {}\n",
            if self.session_present {
                "present"
            } else {
                "none"
            }
        ));
        out.push_str(&format!(
            "  backend : {}\n",
            self.backend.as_deref().unwrap_or("-")
        ));
        out.push_str(&format!(
            "  mode    : {}{}\n",
            self.mode.as_deref().unwrap_or("-"),
            if self.strict_marker {
                " (strict marker set)"
            } else {
                ""
            }
        ));
        out.push_str(&format!(
            "  started : {}\n",
            self.started_at.as_deref().unwrap_or("-")
        ));
        out.push_str(&format!(
            "  daemon  : {}\n",
            match self.daemon_pid {
                Some(pid) if self.daemon_live => format!("pid {pid} (live)"),
                Some(pid) => format!("pid {pid} (outdated)"),
                None => "not running".to_string(),
            }
        ));
        out.push_str(&format!(
            "  mount   : {}\n",
            if self.mount_present {
                "present"
            } else {
                "missing"
            }
        ));
        out.push_str(&format!(
            "  blocked : {} file(s)\n",
            self.blocked_paths.len()
        ));
        for path in &self.blocked_paths {
            out.push_str(&format!("    - {}\n", path.display()));
        }
        out.push_str(&format!(
            "  quarantined this session : {}\n",
            self.quarantined_count
        ));
        out
    }

    /// Renders the status as pretty JSON.
    pub fn render_json(&self) -> Result<String, GuardError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| GuardError::State(format!("serialize status: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn status_of_unstarted_repo_is_empty() {
        let dir = tempdir().unwrap();
        let layout = GuardLayout {
            repo_root: dir.path().to_path_buf(),
            mount: dir.path().to_path_buf(),
            backing: dir.path().to_path_buf(),
            state: dir.path().join("state"),
            repo_id: "t-0000".to_string(),
        };
        let status = GuardStatus::collect(&layout).unwrap();
        assert!(!status.session_present);
        assert!(!status.daemon_live);
        assert!(status.blocked_paths.is_empty());
        assert!(status.render_json().unwrap().contains("\"repo_id\""));
    }
}
