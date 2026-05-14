//! The cross-platform watcher backend. Unlike the FUSE backend this is a
//! *post-write* guard: a write has already touched the repo when the guard sees
//! it. The backend reacts by either accepting the write (and refreshing the
//! last-good snapshot) or rejecting it — reverting a modified file to its
//! snapshot, or poisoning a brand-new file in place — and always writing a
//! failure report.
//!
//! This is an honest limitation of in-place guarding: the FUSE backend is the
//! Linux-only upgrade that prevents the bytes from ever landing.

pub mod debounce;
pub mod watcher;

use crate::audit_client::{AuditClient, Verdict};
use crate::feedback::{report, DenialBus, GuardEvent};
use crate::layout::GuardLayout;
use crate::poison::{self, PoisonState};
use crate::policy::{GuardPolicy, OnFail};
use crate::state::{GuardState, QuarantineEntry};
use crate::{commit, GuardError, GuardMode};
use debounce::Debouncer;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use watcher::RepoWatcher;

/// The watcher backend. It owns the snapshot index and poison state for one
/// guarded repository and runs the audit loop until [`stop`](WatcherBackend::handle)
/// is signalled.
pub struct WatcherBackend {
    layout: GuardLayout,
    policy: GuardPolicy,
    audit: Arc<dyn AuditClient>,
    bus: Arc<DenialBus>,
    poison: PoisonState,
    /// Last-good snapshot sha for each known repo-relative path.
    snapshots: HashMap<PathBuf, String>,
}

/// A handle for asking a running [`WatcherBackend`] to stop.
#[derive(Clone, Default)]
pub struct WatcherHandle {
    stop: Arc<AtomicBool>,
}

impl WatcherHandle {
    /// Signals the backend loop to exit at the next iteration.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// Returns `true` once a stop has been requested.
    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
}

impl WatcherBackend {
    /// Builds a backend for `layout`, loading any persisted poison state.
    pub fn new(
        layout: GuardLayout,
        policy: GuardPolicy,
        audit: Arc<dyn AuditClient>,
        bus: Arc<DenialBus>,
    ) -> Result<Self, GuardError> {
        layout.ensure_dirs()?;
        let poison = PoisonState::load(&layout.poison_dir())?;
        Ok(Self {
            layout,
            policy,
            audit,
            bus,
            poison,
            snapshots: HashMap::new(),
        })
    }

    /// Runs the watch loop until `handle` is stopped. When `poll` is true the
    /// portable `PollWatcher` is used. This call blocks the calling thread.
    pub fn run(mut self, handle: WatcherHandle, poll: bool) -> Result<(), GuardError> {
        let repo_watcher = RepoWatcher::start(&self.layout.repo_root, &self.policy, poll)?;
        let mut debouncer = Debouncer::new(self.policy.debounce_ms, self.policy.stable_ms);
        let events = repo_watcher.events();

        while !handle.is_stopped() {
            let now = Instant::now();
            // Drain all currently-available events into the debouncer.
            loop {
                match events.recv_timeout(idle_timeout(&debouncer, now)) {
                    Ok(rel) => {
                        let abs = self.layout.repo_root.join(&rel);
                        debouncer.observe(&abs, Instant::now());
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => break,
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        return Ok(());
                    }
                }
            }
            for abs in debouncer.settled(Instant::now()) {
                if let Ok(rel) = abs.strip_prefix(&self.layout.repo_root) {
                    self.handle_settled(rel)?;
                }
            }
        }
        Ok(())
    }

    /// Handles one settled path: read it, strip any poison header, audit it,
    /// then act on the verdict.
    fn handle_settled(&mut self, rel: &Path) -> Result<(), GuardError> {
        let abs = self.layout.repo_root.join(rel);
        let raw = match std::fs::read(&abs) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // The path was deleted; nothing to audit and nothing to revert.
                self.snapshots.remove(rel);
                return Ok(());
            }
            Err(e) => return Err(GuardError::Io(e)),
        };
        // An agent re-reading then re-saving a poisoned file would otherwise
        // feed the poison header back into the audit; strip it first.
        let candidate = poison::strip(&raw);
        let existed_before = self.snapshots.contains_key(rel);

        let decision = self.audit.audit(&self.layout.repo_root, rel, &candidate)?;

        match decision.verdict {
            Verdict::Pass | Verdict::Advisory => {
                let sha = commit::snapshot_save(&self.layout.state, &candidate)?;
                self.snapshots.insert(rel.to_path_buf(), sha);
                if self.poison.clear(rel) {
                    self.poison.save(&self.layout.poison_dir())?;
                }
                self.bus.publish(GuardEvent::Pass {
                    rel_path: rel.to_path_buf(),
                });
                Ok(())
            }
            Verdict::Block => self.handle_block(rel, &abs, &candidate, decision, existed_before),
        }
    }

    /// Reacts to a block: quarantine the candidate, write the report, then
    /// (in enforce/strict mode) revert a modified file or poison a new file.
    fn handle_block(
        &mut self,
        rel: &Path,
        abs: &Path,
        candidate: &[u8],
        decision: crate::audit_client::GuardDecision,
        existed_before: bool,
    ) -> Result<(), GuardError> {
        let report_path = report::write_failure_report(&self.layout.repo_root, &decision)?;
        let quarantine_path = commit::quarantine_candidate(&self.layout.repo_root, rel, candidate)?;

        let enforce = self.policy.mode.enforces() && !matches!(self.policy.on_fail, OnFail::Warn);
        if enforce {
            if existed_before {
                if let Some(sha) = self.snapshots.get(rel) {
                    let snapshot = commit::snapshot_path(&self.layout.state, sha);
                    commit::revert_to_last_good(abs, &snapshot)?;
                }
            } else {
                let content = crate::poison::Content {
                    path: decision.path.clone(),
                    rule_ids: decision.blocking.all().map(|f| f.rule_id.clone()).collect(),
                    problems: decision.blocking.all().map(|f| f.problem.clone()).collect(),
                    fix_steps: decision
                        .blocking
                        .all()
                        .map(|f| f.agent_fix.clone())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    rerun_command: decision.rerun_command.clone(),
                    report_path: report_path.to_string_lossy().into_owned(),
                };
                let view = poison::wrap(rel, candidate, &content);
                std::fs::write(abs, &view)?;
                self.poison.insert(rel, &view);
                self.poison.save(&self.layout.poison_dir())?;
            }
        }

        self.record_quarantine(rel, &quarantine_path, &report_path)?;
        self.bus.publish(GuardEvent::Block {
            rel_path: rel.to_path_buf(),
            decision: Box::new(decision),
            report_path,
        });
        Ok(())
    }

    /// Appends a quarantine entry to the persisted guard state.
    fn record_quarantine(
        &self,
        rel: &Path,
        quarantine_path: &Path,
        report_path: &Path,
    ) -> Result<(), GuardError> {
        let mut state = match GuardState::load(&self.layout)? {
            Some(s) => s,
            None => GuardState::new(&self.layout.repo_id, self.policy.mode, "watcher"),
        };
        state.record_quarantine(QuarantineEntry {
            rel_path: rel.to_path_buf(),
            quarantine_path: quarantine_path.to_path_buf(),
            report_path: report_path.to_path_buf(),
            blocked_at: crate::feedback::now_rfc3339(),
        });
        state.save(&self.layout)
    }

    /// Seeds the snapshot index with the repository's current on-disk content
    /// so a later modification can be reverted to this baseline. Excluded paths
    /// are skipped. Used at startup.
    pub fn prime_snapshots(&mut self) -> Result<(), GuardError> {
        let root = self.layout.repo_root.clone();
        self.prime_dir(&root)
    }

    /// Recursively snapshots every non-excluded regular file under `dir`.
    fn prime_dir(&mut self, dir: &Path) -> Result<(), GuardError> {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return Ok(()),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = match path.strip_prefix(&self.layout.repo_root) {
                Ok(rel) => rel.to_path_buf(),
                Err(_) => continue,
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if self.policy.is_excluded(&rel_str) {
                continue;
            }
            match entry.file_type() {
                Ok(ft) if ft.is_dir() => self.prime_dir(&path)?,
                Ok(ft) if ft.is_file() => {
                    if let Ok(bytes) = std::fs::read(&path) {
                        let clean = poison::strip(&bytes);
                        let sha = commit::snapshot_save(&self.layout.state, &clean)?;
                        self.snapshots.insert(rel, sha);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Read-only access to the resolved layout.
    pub fn layout(&self) -> &GuardLayout {
        &self.layout
    }

    /// Mode the backend is running in.
    pub fn mode(&self) -> GuardMode {
        self.policy.mode
    }
}

/// The receive timeout for the event drain: short enough to react to settled
/// paths promptly, but never longer than the next debounce deadline.
fn idle_timeout(debouncer: &Debouncer, now: Instant) -> Duration {
    let cap = Duration::from_millis(250);
    match debouncer.next_deadline(now) {
        Some(deadline) => deadline.min(cap).max(Duration::from_millis(5)),
        None => cap,
    }
}
