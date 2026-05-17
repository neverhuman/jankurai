//! `guard doctor`: a preflight that checks whether the environment can run the
//! guard — FUSE availability, backing-directory permissions, mount presence,
//! session liveness, and whether the git hooks are installed. Output is human-
//! readable or `--json`.

use crate::layout::GuardLayout;
use crate::state;
use crate::{fuse, GuardError};
use serde::Serialize;

/// The outcome of a single doctor check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel {
    /// The check passed.
    Ok,
    /// The check passed but something is worth noting.
    Warn,
    /// The check failed.
    Fail,
}

/// One named doctor check with a result and an explanation.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    /// Short name of the check.
    pub name: String,
    /// Severity level of the result.
    pub level: CheckLevel,
    /// Human-readable detail.
    pub detail: String,
}

/// The full doctor report for one repository.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    /// Stable identifier of the guarded repository.
    pub repo_id: String,
    /// The individual checks, in run order.
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    /// Runs every doctor check for `layout` and collects the results.
    pub fn run(layout: &GuardLayout) -> Self {
        let checks = vec![
            check_fuse(),
            check_backing(layout),
            check_state_dir(layout),
            check_mount(layout),
            check_session(layout),
            check_git_hooks(layout),
        ];
        Self {
            repo_id: layout.repo_id.clone(),
            checks,
        }
    }

    /// Returns `true` when no check failed.
    pub fn healthy(&self) -> bool {
        !self.checks.iter().any(|c| c.level == CheckLevel::Fail)
    }

    /// Renders the report as a human-readable block.
    pub fn render_human(&self) -> String {
        let mut out = format!("guard doctor for {}\n", self.repo_id);
        for check in &self.checks {
            let mark = match check.level {
                CheckLevel::Ok => "ok  ",
                CheckLevel::Warn => "warn",
                CheckLevel::Fail => "fail",
            };
            out.push_str(&format!("  [{mark}] {} — {}\n", check.name, check.detail));
        }
        out.push_str(if self.healthy() {
            "result: healthy\n"
        } else {
            "result: problems found\n"
        });
        out
    }

    /// Renders the report as pretty JSON.
    pub fn render_json(&self) -> Result<String, GuardError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| GuardError::State(format!("serialize doctor report: {e}")))
    }
}

/// Checks whether a FUSE backend is linked and usable.
fn check_fuse() -> DoctorCheck {
    if fuse::fuse_available() {
        DoctorCheck {
            name: "fuse".to_string(),
            level: CheckLevel::Ok,
            detail: "FUSE backend is built and available".to_string(),
        }
    } else {
        DoctorCheck {
            name: "fuse".to_string(),
            level: CheckLevel::Warn,
            detail: "FUSE backend not available on this platform/build; \
                     `guard watch` is used instead"
                .to_string(),
        }
    }
}

/// Checks that the backing directory exists or can be created and is writable.
fn check_backing(layout: &GuardLayout) -> DoctorCheck {
    let dir = &layout.backing;
    if std::fs::create_dir_all(dir).is_err() {
        return DoctorCheck {
            name: "backing-dir".to_string(),
            level: CheckLevel::Fail,
            detail: format!("cannot create backing directory {}", dir.display()),
        };
    }
    let probe = dir.join(".jankurai-doctor-probe");
    match std::fs::write(&probe, b"probe") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            DoctorCheck {
                name: "backing-dir".to_string(),
                level: CheckLevel::Ok,
                detail: format!("{} is writable", dir.display()),
            }
        }
        Err(e) => DoctorCheck {
            name: "backing-dir".to_string(),
            level: CheckLevel::Fail,
            detail: format!("{} is not writable: {e}", dir.display()),
        },
    }
}

/// Checks that the guard state directory exists or can be created.
fn check_state_dir(layout: &GuardLayout) -> DoctorCheck {
    match std::fs::create_dir_all(&layout.state) {
        Ok(()) => DoctorCheck {
            name: "state-dir".to_string(),
            level: CheckLevel::Ok,
            detail: format!("{} is present", layout.state.display()),
        },
        Err(e) => DoctorCheck {
            name: "state-dir".to_string(),
            level: CheckLevel::Fail,
            detail: format!("cannot create {}: {e}", layout.state.display()),
        },
    }
}

/// Checks whether the guarded mount path is present.
fn check_mount(layout: &GuardLayout) -> DoctorCheck {
    if layout.mount.exists() {
        DoctorCheck {
            name: "mount".to_string(),
            level: CheckLevel::Ok,
            detail: format!("{} exists", layout.mount.display()),
        }
    } else {
        DoctorCheck {
            name: "mount".to_string(),
            level: CheckLevel::Warn,
            detail: format!("{} is not present", layout.mount.display()),
        }
    }
}

/// Checks whether a recorded guard session pid is still alive.
fn check_session(layout: &GuardLayout) -> DoctorCheck {
    match state::read_pidfile(layout) {
        Some(pid) if state::pid_is_live(pid) => DoctorCheck {
            name: "session".to_string(),
            level: CheckLevel::Ok,
            detail: format!("guard session pid {pid} is live"),
        },
        Some(pid) => DoctorCheck {
            name: "session".to_string(),
            level: CheckLevel::Warn,
            detail: format!("pidfile names pid {pid} but it is not running (outdated)"),
        },
        None => DoctorCheck {
            name: "session".to_string(),
            level: CheckLevel::Warn,
            detail: "no guard session is running".to_string(),
        },
    }
}

/// Checks whether jankurai's git hooks are installed in the repository.
fn check_git_hooks(layout: &GuardLayout) -> DoctorCheck {
    let hooks_dir = layout.repo_root.join(".git").join("hooks");
    if !hooks_dir.exists() {
        return DoctorCheck {
            name: "git-hooks".to_string(),
            level: CheckLevel::Warn,
            detail: "no .git/hooks directory; repository may not be a git checkout".to_string(),
        };
    }
    let pre_commit = hooks_dir.join("pre-commit");
    let installed = std::fs::read_to_string(&pre_commit)
        .map(|text| text.contains("jankurai"))
        .unwrap_or(false);
    if installed {
        DoctorCheck {
            name: "git-hooks".to_string(),
            level: CheckLevel::Ok,
            detail: "jankurai pre-commit hook is installed".to_string(),
        }
    } else {
        DoctorCheck {
            name: "git-hooks".to_string(),
            level: CheckLevel::Warn,
            detail: "jankurai pre-commit hook is not installed; run `guard install`".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn doctor_runs_all_checks() {
        let dir = tempdir().unwrap();
        let layout = GuardLayout {
            repo_root: dir.path().to_path_buf(),
            mount: dir.path().to_path_buf(),
            backing: dir.path().join("backing"),
            state: dir.path().join("state"),
            repo_id: "t-0000".to_string(),
        };
        let report = DoctorReport::run(&layout);
        assert_eq!(report.checks.len(), 6);
        assert!(report.render_json().unwrap().contains("\"checks\""));
    }
}
