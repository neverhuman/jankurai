//! Guard directory layout. The guard uses a three-directory model:
//!
//! - the **mount** is the guarded view the agent sees (the repo path itself in
//!   watcher mode, a FUSE mountpoint in FUSE mode),
//! - the **backing** store at `~/.jankurai/backing/<repo-id>` holds the real
//!   repository — only the daemon writes here in FUSE mode,
//! - the **state** dir at `~/.jankurai/state/<repo-id>` holds guard state:
//!   candidates, snapshots, poison views, denials and the pidfile.
//!
//! `<repo-id>` is a short hash of the repository's canonical path so two repos
//! with the same basename never collide.

use crate::GuardError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Resolved directory locations for one guarded repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardLayout {
    /// Canonicalized path of the repository (the backing content in FUSE mode,
    /// the in-place repo in watcher mode).
    pub repo_root: PathBuf,
    /// The guarded view the agent interacts with.
    pub mount: PathBuf,
    /// The real repository contents (equal to `repo_root` in watcher mode).
    pub backing: PathBuf,
    /// The guard state directory for this repo.
    pub state: PathBuf,
    /// The stable short identifier derived from the repo's canonical path.
    pub repo_id: String,
}

impl GuardLayout {
    /// Resolves the watcher-mode layout for `repo`. The mount and backing both
    /// equal the repo path; state lives under `~/.jankurai/state/<repo-id>`.
    pub fn watcher(repo: &Path) -> Result<Self, GuardError> {
        let repo_root = canonical(repo)?;
        let repo_id = repo_id(&repo_root);
        let state = jankurai_home()?.join("state").join(&repo_id);
        Ok(Self {
            mount: repo_root.clone(),
            backing: repo_root.clone(),
            repo_root,
            state,
            repo_id,
        })
    }

    /// Resolves the FUSE-mode layout for `repo` mounted at `mount_point`. The
    /// backing store is `~/.jankurai/backing/<repo-id>`; state lives under
    /// `~/.jankurai/state/<repo-id>`.
    pub fn fuse(repo: &Path, mount_point: &Path) -> Result<Self, GuardError> {
        let repo_root = canonical(repo)?;
        let repo_id = repo_id(&repo_root);
        let home = jankurai_home()?;
        let backing = home.join("backing").join(&repo_id);
        let state = home.join("state").join(&repo_id);
        let mount = if mount_point.is_absolute() {
            mount_point.to_path_buf()
        } else {
            repo_root.join(mount_point)
        };
        Ok(Self {
            repo_root,
            mount,
            backing,
            state,
            repo_id,
        })
    }

    /// Creates the state directory tree (and the backing dir for FUSE mode).
    pub fn ensure_dirs(&self) -> Result<(), GuardError> {
        for dir in [
            &self.state,
            &self.state.join("snapshots"),
            &self.state.join("poison"),
            &self.state.join("candidates"),
            &self.state.join("denials"),
        ] {
            std::fs::create_dir_all(dir)?;
        }
        if self.backing != self.repo_root {
            std::fs::create_dir_all(&self.backing)?;
        }
        Ok(())
    }

    /// Path to the daemon pidfile.
    pub fn pidfile(&self) -> PathBuf {
        self.state.join("guard.pid")
    }

    /// Path to the persisted guard-state document.
    pub fn state_file(&self) -> PathBuf {
        self.state.join("guard-state.json")
    }

    /// Path to the strict-mode marker file.
    pub fn strict_marker(&self) -> PathBuf {
        self.state.join("STRICT_MODE")
    }

    /// Directory holding per-sha snapshot blobs.
    pub fn snapshots_dir(&self) -> PathBuf {
        self.state.join("snapshots")
    }

    /// Directory holding persisted poison views.
    pub fn poison_dir(&self) -> PathBuf {
        self.state.join("poison")
    }

    /// The `.jankurai/guard/` directory inside the repository, used for human-
    /// and agent-facing artifacts (reports, quarantine).
    pub fn guard_artifacts_dir(&self) -> PathBuf {
        self.repo_root.join(".jankurai").join("guard")
    }
}

/// Canonicalizes `path`, surfacing a [`GuardError::Layout`] when it cannot be
/// resolved (for example a missing repository directory).
fn canonical(path: &Path) -> Result<PathBuf, GuardError> {
    std::fs::canonicalize(path)
        .map_err(|e| GuardError::Layout(format!("cannot resolve {}: {e}", path.display())))
}

/// Computes the stable repo identifier: the repo basename plus a short SHA-256
/// prefix of its canonical path.
pub fn repo_id(canonical_repo: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_repo.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        hex.push_str(&format!("{byte:02x}"));
    }
    let name = canonical_repo
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo");
    let sanitized: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("{sanitized}-{hex}")
}

/// Resolves the jankurai home directory, `~/.jankurai`, honoring the
/// `JANKURAI_HOME` override when present.
pub fn jankurai_home() -> Result<PathBuf, GuardError> {
    if let Ok(explicit) = std::env::var("JANKURAI_HOME") {
        if !explicit.trim().is_empty() {
            return Ok(PathBuf::from(explicit));
        }
    }
    let home = match home_dir() {
        Some(h) => h,
        None => {
            return Err(GuardError::Layout(
                "cannot determine home directory".to_string(),
            ))
        }
    };
    Ok(home.join(".jankurai"))
}

/// Resolves the current user's home directory from the environment.
fn home_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home));
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        if !profile.is_empty() {
            return Some(PathBuf::from(profile));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_id_is_stable_and_sanitized() {
        let a = repo_id(Path::new("/tmp/My Repo"));
        let b = repo_id(Path::new("/tmp/My Repo"));
        assert_eq!(a, b);
        assert!(a.starts_with("My-Repo-"));
        assert!(!a.contains(' '));
    }

    #[test]
    fn different_paths_get_different_ids() {
        assert_ne!(repo_id(Path::new("/a/repo")), repo_id(Path::new("/b/repo")));
    }
}
