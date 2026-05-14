//! End-to-end tests for the cross-platform watcher backend with a tempdir of
//! real files and a `MockAuditClient`. They verify the post-write reactions:
//! a blocked new file is poisoned + quarantined and a `LAST_FAILURE.md` is
//! written; a blocked modification is reverted to its snapshot; an atomic-save
//! sequence collapses to one audit call; and writes under `.jankurai/` never
//! re-trigger an audit.

use jankurai_guard::audit_client::MockAuditClient;
use jankurai_guard::feedback::{DenialBus, GuardEvent};
use jankurai_guard::layout::GuardLayout;
use jankurai_guard::policy::GuardPolicy;
use jankurai_guard::watch::{WatcherBackend, WatcherHandle};
use jankurai_guard::GuardMode;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Spins up a watcher backend over `repo` with `audit`, returning the running
/// handle, the join handle, the bus, and the resolved layout. The polling
/// watcher is used so the test does not depend on native-watcher latency.
struct Harness {
    handle: WatcherHandle,
    thread: Option<std::thread::JoinHandle<()>>,
    bus: Arc<DenialBus>,
    audit: Arc<MockAuditClient>,
}

impl Harness {
    fn start(repo: &Path, audit: MockAuditClient, mode: GuardMode) -> Self {
        let layout = GuardLayout::watcher(repo).unwrap();
        // Tight timings keep the test fast while still exercising the debounce.
        let policy = GuardPolicy {
            mode,
            debounce_ms: 40,
            stable_ms: 20,
            ..GuardPolicy::default()
        };
        let audit = Arc::new(audit);
        let bus = Arc::new(DenialBus::new());
        let mut backend = WatcherBackend::new(
            layout.clone(),
            policy,
            Arc::clone(&audit) as Arc<dyn jankurai_guard::AuditClient>,
            Arc::clone(&bus),
        )
        .unwrap();
        backend.prime_snapshots().unwrap();
        let handle = WatcherHandle::default();
        let thread = {
            let h = handle.clone();
            std::thread::spawn(move || {
                let _ = backend.run(h, true);
            })
        };
        // Give the backend's PollWatcher time to take its initial snapshot
        // before the test mutates files, so the first change is observed as an
        // event rather than folded into the baseline snapshot.
        std::thread::sleep(Duration::from_millis(500));
        Self {
            handle,
            thread: Some(thread),
            bus,
            audit,
        }
    }

    fn stop(mut self) {
        self.handle.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Polls `predicate` until it is true or the deadline passes.
fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    predicate()
}

#[test]
fn blocked_new_file_is_poisoned_and_quarantined() {
    let dir = tempfile::tempdir().unwrap();
    let harness = Harness::start(
        dir.path(),
        MockAuditClient::always_block(),
        GuardMode::Enforce,
    );
    let events = harness.bus.subscribe();

    let new_file = dir.path().join("src").join("new.rs");
    std::fs::create_dir_all(new_file.parent().unwrap()).unwrap();
    std::fs::write(&new_file, b"fn main() { bad() }\n").unwrap();

    let blocked = wait_until(Duration::from_secs(8), || {
        matches!(events.try_recv(), Ok(GuardEvent::Block { .. }))
    });
    assert!(blocked, "expected a Block event for the new file");

    // The on-disk file is now poisoned.
    let on_disk = std::fs::read(&new_file).unwrap();
    assert!(
        jankurai_guard::poison::is_poisoned(&on_disk),
        "new file should be poisoned in place"
    );
    // LAST_FAILURE.md was written.
    let last = dir.path().join(".jankurai/guard/LAST_FAILURE.md");
    assert!(last.exists(), "LAST_FAILURE.md should be written");
    // The candidate was quarantined under .jankurai/guard/rejected/.
    let rejected_root = dir.path().join(".jankurai/guard/rejected");
    assert!(rejected_root.exists(), "quarantine directory should exist");

    harness.stop();
}

#[test]
fn blocked_modification_is_reverted_to_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let tracked = dir.path().join("lib.rs");
    std::fs::write(&tracked, b"// good baseline\n").unwrap();

    let harness = Harness::start(
        dir.path(),
        MockAuditClient::always_block(),
        GuardMode::Enforce,
    );
    let events = harness.bus.subscribe();

    // Overwrite the tracked file with content the mock will block.
    std::fs::write(&tracked, b"// agent's bad change\n").unwrap();

    let blocked = wait_until(Duration::from_secs(8), || {
        matches!(events.try_recv(), Ok(GuardEvent::Block { .. }))
    });
    assert!(blocked, "expected a Block event for the modification");

    // The file was reverted to the primed baseline.
    let reverted = wait_until(Duration::from_secs(4), || {
        std::fs::read(&tracked).unwrap_or_default() == b"// good baseline\n"
    });
    assert!(reverted, "modified file should be reverted to last-good");

    harness.stop();
}

#[test]
fn passing_writes_publish_pass_events() {
    let dir = tempfile::tempdir().unwrap();
    let harness = Harness::start(
        dir.path(),
        MockAuditClient::always_pass(),
        GuardMode::Enforce,
    );
    let events = harness.bus.subscribe();

    let file = dir.path().join("ok.rs");
    std::fs::write(&file, b"// fine\n").unwrap();

    let passed = wait_until(Duration::from_secs(8), || {
        matches!(events.try_recv(), Ok(GuardEvent::Pass { .. }))
    });
    assert!(passed, "expected a Pass event for the clean write");
    // A passing write is never poisoned.
    assert!(!jankurai_guard::poison::is_poisoned(
        &std::fs::read(&file).unwrap()
    ));

    harness.stop();
}

#[test]
fn atomic_save_sequence_is_one_audit_call() {
    let dir = tempfile::tempdir().unwrap();
    let harness = Harness::start(
        dir.path(),
        MockAuditClient::always_pass(),
        GuardMode::Enforce,
    );

    // Simulate an editor's atomic save: write a temp file, then rename it over
    // the target. The debounce + stability window must collapse this to one
    // audit of the final path.
    let tmp = dir.path().join("config.toml.tmp");
    let target = dir.path().join("config.toml");
    std::fs::write(&tmp, b"key = \"value\"\n").unwrap();
    std::fs::rename(&tmp, &target).unwrap();

    // Give the watcher time to settle and audit.
    let audited = wait_until(Duration::from_secs(8), || harness.audit.call_count() >= 1);
    assert!(audited, "the atomic save should be audited");
    // Let any stray follow-up events flush through, then assert it stayed at 1.
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(
        harness.audit.call_count(),
        1,
        "an atomic-save sequence must collapse to exactly one audit call"
    );

    harness.stop();
}

#[test]
fn writes_under_jankurai_do_not_retrigger_audit() {
    let dir = tempfile::tempdir().unwrap();
    let harness = Harness::start(
        dir.path(),
        MockAuditClient::always_pass(),
        GuardMode::Enforce,
    );

    // Write directly under .jankurai/ — the guard's own report area. This must
    // never be audited, otherwise the guard's report writes would loop.
    let guard_file = dir
        .path()
        .join(".jankurai")
        .join("guard")
        .join("scratch.md");
    std::fs::create_dir_all(guard_file.parent().unwrap()).unwrap();
    std::fs::write(&guard_file, b"# internal\n").unwrap();
    // Also write under target/jankurai/.
    let target_file = dir.path().join("target").join("jankurai").join("cache.bin");
    std::fs::create_dir_all(target_file.parent().unwrap()).unwrap();
    std::fs::write(&target_file, b"cache").unwrap();

    // Wait well past the debounce window; no audit call should occur.
    std::thread::sleep(Duration::from_millis(700));
    assert_eq!(
        harness.audit.call_count(),
        0,
        "writes under .jankurai/ and target/jankurai/ must not be audited"
    );

    harness.stop();
}

#[test]
fn observe_mode_does_not_touch_blocked_files() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("watched.rs");
    std::fs::write(&file, b"original\n").unwrap();

    let harness = Harness::start(
        dir.path(),
        MockAuditClient::always_block(),
        GuardMode::Observe,
    );
    let events = harness.bus.subscribe();

    std::fs::write(&file, b"changed by agent\n").unwrap();

    let blocked = wait_until(Duration::from_secs(8), || {
        matches!(events.try_recv(), Ok(GuardEvent::Block { .. }))
    });
    assert!(blocked, "observe mode should still publish a Block event");
    // But in observe mode the file is left exactly as the agent wrote it.
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        std::fs::read(&file).unwrap(),
        b"changed by agent\n",
        "observe mode must not revert or poison files"
    );
    // The report is still written.
    assert!(dir.path().join(".jankurai/guard/LAST_FAILURE.md").exists());

    harness.stop();
}
