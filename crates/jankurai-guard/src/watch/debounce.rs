//! Per-path debounce and stability tracking for the watcher backend. A single
//! logical save by an editor or agent is rarely a single filesystem event: it
//! is a burst of write/chmod/rename events. The [`Debouncer`] collapses that
//! burst into one audit by waiting for a path to stay quiet for `debounce_ms`
//! and for its size + mtime to stay unchanged for `stable_ms`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A snapshot of a path's observable state, used to detect stability.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileFingerprint {
    /// File size in bytes, or `None` when the path does not currently exist.
    size: Option<u64>,
    /// Modification time as seconds since the epoch, or `None` when absent.
    mtime: Option<i64>,
}

/// Pending state for one path that has seen recent events.
#[derive(Debug, Clone)]
struct Pending {
    /// When the most recent event for this path arrived.
    last_event: Instant,
    /// When the fingerprint last changed.
    last_change: Instant,
    /// The most recently observed fingerprint.
    fingerprint: FileFingerprint,
}

/// Collapses bursts of filesystem events into one settled change per path.
#[derive(Debug)]
pub struct Debouncer {
    debounce: Duration,
    stable: Duration,
    pending: HashMap<PathBuf, Pending>,
}

impl Debouncer {
    /// Creates a debouncer with the given quiet and stability windows.
    pub fn new(debounce_ms: u64, stable_ms: u64) -> Self {
        Self {
            debounce: Duration::from_millis(debounce_ms),
            stable: Duration::from_millis(stable_ms),
            pending: HashMap::new(),
        }
    }

    /// Records that an event was observed for `path` at `now`. The current
    /// on-disk fingerprint is captured so stability can be judged later.
    pub fn observe(&mut self, path: &Path, now: Instant) {
        let fingerprint = fingerprint_of(path);
        match self.pending.get_mut(path) {
            Some(entry) => {
                entry.last_event = now;
                if entry.fingerprint != fingerprint {
                    entry.fingerprint = fingerprint;
                    entry.last_change = now;
                }
            }
            None => {
                self.pending.insert(
                    path.to_path_buf(),
                    Pending {
                        last_event: now,
                        last_change: now,
                        fingerprint,
                    },
                );
            }
        }
    }

    /// Returns the paths that have settled by `now`: no event for at least the
    /// debounce window and a stable fingerprint for at least the stability
    /// window. Settled paths are removed from the pending set.
    pub fn settled(&mut self, now: Instant) -> Vec<PathBuf> {
        let mut ready = Vec::new();
        let debounce = self.debounce;
        let stable = self.stable;
        self.pending.retain(|path, entry| {
            let quiet_for = now.saturating_duration_since(entry.last_event);
            let stable_for = now.saturating_duration_since(entry.last_change);
            if quiet_for >= debounce && stable_for >= stable {
                // Re-check the fingerprint at decision time: if the path
                // changed again between the last event and now it is not
                // settled and should keep waiting.
                if fingerprint_of(path) == entry.fingerprint {
                    ready.push(path.clone());
                    return false;
                }
            }
            true
        });
        ready
    }

    /// Returns the shortest time until any pending path could settle, used to
    /// size the watcher's poll interval. `None` when nothing is pending.
    pub fn next_deadline(&self, now: Instant) -> Option<Duration> {
        self.pending
            .values()
            .map(|entry| {
                let quiet_left = self
                    .debounce
                    .saturating_sub(now.saturating_duration_since(entry.last_event));
                let stable_left = self
                    .stable
                    .saturating_sub(now.saturating_duration_since(entry.last_change));
                quiet_left.max(stable_left)
            })
            .min()
    }

    /// Returns the number of paths currently pending.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

/// Captures the size + mtime fingerprint of `path`. A path that does not exist
/// (for example, a deleted file or a rename source) yields an all-`None`
/// fingerprint, which is itself a stable, observable state.
fn fingerprint_of(path: &Path) -> FileFingerprint {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            use std::os::unix::fs::MetadataExt;
            FileFingerprint {
                size: Some(meta.len()),
                mtime: Some(meta.mtime()),
            }
        }
        Err(_) => FileFingerprint {
            size: None,
            mtime: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn settles_after_quiet_and_stable_windows() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("f.txt");
        std::fs::write(&path, b"a").unwrap();
        let mut deb = Debouncer::new(10, 5);
        let t0 = Instant::now();
        deb.observe(&path, t0);
        assert!(deb.settled(t0).is_empty());
        let later = t0 + Duration::from_millis(50);
        assert_eq!(deb.settled(later), vec![path]);
    }

    #[test]
    fn ongoing_changes_keep_path_pending() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("f.txt");
        let mut file = std::fs::File::create(&path).unwrap();
        let mut deb = Debouncer::new(10, 5);
        let t0 = Instant::now();
        deb.observe(&path, t0);
        // A new event with a changed fingerprint resets the stability clock.
        file.write_all(b"more").unwrap();
        file.sync_all().unwrap();
        let t1 = t0 + Duration::from_millis(8);
        deb.observe(&path, t1);
        assert!(deb.settled(t1 + Duration::from_millis(2)).is_empty());
    }
}
