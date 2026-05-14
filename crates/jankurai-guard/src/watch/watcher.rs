//! The filesystem-event source for the watcher backend. This wraps `notify`'s
//! recommended watcher (with a `PollWatcher` alternative selectable via `--poll`)
//! and applies the guard's exclusion rules: `.gitignore` matching via the
//! `ignore` crate, the policy's `extra_excluded_paths`, and the always-on hard
//! exclusion of `.jankurai/` and `target/jankurai/` so the guard's own report
//! writes never re-trigger an audit.

use crate::policy::GuardPolicy;
use crate::GuardError;
use crossbeam_channel::Receiver;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The poll interval used by the `PollWatcher` when selected via `--poll`.
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// A running filesystem watcher over one repository root. It keeps the
/// underlying `notify` watcher alive and exposes a receiver of repo-relative
/// changed paths that have already passed exclusion filtering.
pub struct RepoWatcher {
    _watcher: WatcherKind,
    events: Receiver<PathBuf>,
}

/// The two watcher implementations the backend can use. The held watcher is
/// never read directly: it is kept alive so that dropping [`RepoWatcher`] stops
/// the underlying `notify` watch.
#[allow(dead_code)]
enum WatcherKind {
    /// The platform-native recommended watcher.
    Native(RecommendedWatcher),
    /// The portable polling watcher, used when inotify is unavailable.
    Poll(notify::PollWatcher),
}

impl RepoWatcher {
    /// Starts watching `repo_root` recursively. When `poll` is true the portable
    /// `PollWatcher` is used; otherwise the platform-native watcher is used.
    /// Events for excluded paths are dropped before they reach the receiver.
    pub fn start(repo_root: &Path, policy: &GuardPolicy, poll: bool) -> Result<Self, GuardError> {
        let gitignore = if policy.respect_gitignore {
            build_gitignore(repo_root)
        } else {
            Gitignore::empty()
        };
        let root = repo_root.to_path_buf();
        let policy = policy.clone();
        let (tx, rx) = crossbeam_channel::unbounded::<PathBuf>();

        let handler = move |result: notify::Result<Event>| {
            let event = match result {
                Ok(event) => event,
                Err(_) => return,
            };
            for path in event.paths {
                if let Some(rel) = filter_path(&path, &root, &policy, &gitignore) {
                    // A full channel only means the consumer is briefly behind;
                    // the path will be re-emitted on the next event for it.
                    let _ = tx.send(rel);
                }
            }
        };

        let watcher = if poll {
            // `with_compare_contents` makes the PollWatcher hash file contents
            // instead of trusting size+mtime, so an agent rewriting a file with
            // the same length within the same mtime tick is still observed as a
            // change.
            let config = Config::default()
                .with_poll_interval(POLL_INTERVAL)
                .with_compare_contents(true);
            let mut w = notify::PollWatcher::new(handler, config)
                .map_err(|e| GuardError::State(format!("poll watcher: {e}")))?;
            w.watch(repo_root, RecursiveMode::Recursive)
                .map_err(|e| GuardError::State(format!("watch {}: {e}", repo_root.display())))?;
            WatcherKind::Poll(w)
        } else {
            let mut w = RecommendedWatcher::new(handler, Config::default())
                .map_err(|e| GuardError::State(format!("native watcher: {e}")))?;
            w.watch(repo_root, RecursiveMode::Recursive)
                .map_err(|e| GuardError::State(format!("watch {}: {e}", repo_root.display())))?;
            WatcherKind::Native(w)
        };

        Ok(Self {
            _watcher: watcher,
            events: rx,
        })
    }

    /// Borrows the receiver of repo-relative changed paths.
    pub fn events(&self) -> &Receiver<PathBuf> {
        &self.events
    }
}

/// Builds a gitignore matcher rooted at `repo_root`, folding in both the
/// repo's `.gitignore` and a `.git/info/exclude` file when present. Missing
/// files are tolerated — they simply contribute no patterns.
fn build_gitignore(repo_root: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(repo_root);
    let gitignore = repo_root.join(".gitignore");
    if gitignore.is_file() {
        let _ = builder.add(&gitignore);
    }
    let exclude = repo_root.join(".git").join("info").join("exclude");
    if exclude.is_file() {
        let _ = builder.add(&exclude);
    }
    match builder.build() {
        Ok(gi) => gi,
        Err(_) => Gitignore::empty(),
    }
}

/// Decides whether `path` should be audited. Returns the repo-relative path on
/// keep, or `None` when the path is excluded by any rule. The exclusion order
/// is: outside-the-repo, hard-excluded prefixes, policy prefixes, gitignore.
fn filter_path(
    path: &Path,
    repo_root: &Path,
    policy: &GuardPolicy,
    gitignore: &Gitignore,
) -> Option<PathBuf> {
    let rel = path.strip_prefix(repo_root).ok()?;
    if rel.as_os_str().is_empty() {
        return None;
    }
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    if policy.is_excluded(&rel_str) {
        return None;
    }
    // Only regular files are audit candidates; directory events are ignored.
    // A deleted path also reaches here — it is kept so a delete can be handled.
    if path.is_dir() {
        return None;
    }
    let is_dir_hint = false;
    if gitignore
        .matched_path_or_any_parents(rel, is_dir_hint)
        .is_ignore()
    {
        return None;
    }
    Some(rel.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_excludes_jankurai_and_target_jankurai() {
        let policy = GuardPolicy::default();
        let gi = Gitignore::empty();
        let root = Path::new("/repo");
        assert!(filter_path(
            &root.join(".jankurai/guard/LAST_FAILURE.md"),
            root,
            &policy,
            &gi
        )
        .is_none());
        assert!(filter_path(&root.join("target/jankurai/x"), root, &policy, &gi).is_none());
    }

    #[test]
    fn gitignore_excludes_matching_paths() {
        let policy = GuardPolicy::default();
        let mut builder = GitignoreBuilder::new("/repo");
        builder.add_line(None, "*.log").unwrap();
        let gi = builder.build().unwrap();
        let root = Path::new("/repo");
        assert!(filter_path(&root.join("debug.log"), root, &policy, &gi).is_none());
    }
}
