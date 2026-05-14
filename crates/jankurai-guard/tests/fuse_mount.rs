//! Real end-to-end FUSE mount tests. This whole file is compiled only on Linux
//! with the `fuse` feature; on every other platform the `#![cfg(...)]` makes it
//! an empty translation unit so it is never built here. Every test is also
//! `#[ignore]` because mounting a FUSE filesystem requires `/dev/fuse` access
//! that is not available in every CI sandbox: run them explicitly with
//! `cargo test -p jankurai-guard --features fuse -- --ignored`.

#![cfg(all(feature = "fuse", target_os = "linux"))]

use jankurai_guard::audit_client::MockAuditClient;
use jankurai_guard::feedback::DenialBus;
use jankurai_guard::fuse;
use jankurai_guard::layout::GuardLayout;
use jankurai_guard::policy::GuardPolicy;
use jankurai_guard::AuditClient;
use std::sync::Arc;
use std::time::Duration;

/// Builds a FUSE layout for a fresh tempdir repo plus mountpoint.
fn fuse_layout(repo: &std::path::Path) -> GuardLayout {
    let mount_point = repo.join("mnt");
    std::fs::create_dir_all(&mount_point).unwrap();
    GuardLayout::fuse(repo, &mount_point).unwrap()
}

#[test]
#[ignore]
fn fuse_is_reported_available_on_linux_feature_build() {
    // On a Linux + `fuse` build the backend reports itself available.
    assert!(fuse::fuse_available());
}

#[test]
#[ignore]
fn passing_write_lands_in_backing_store() {
    let dir = tempfile::tempdir().unwrap();
    let layout = fuse_layout(dir.path());
    layout.ensure_dirs().unwrap();
    let audit: Arc<dyn AuditClient> = Arc::new(MockAuditClient::always_pass());
    let bus = Arc::new(DenialBus::new());

    let session = fuse::mount(layout.clone(), GuardPolicy::default(), audit, bus).unwrap();
    let path = layout.mount.join("hello.txt");
    std::fs::write(&path, b"clean content\n").unwrap();
    std::thread::sleep(Duration::from_millis(200));

    // The passing write is committed into the backing store.
    let backed = layout.backing.join("hello.txt");
    assert_eq!(std::fs::read(&backed).unwrap(), b"clean content\n");
    session.unmount();
}

#[test]
#[ignore]
fn blocked_write_returns_eacces_and_leaves_backing_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let layout = fuse_layout(dir.path());
    layout.ensure_dirs().unwrap();
    let audit: Arc<dyn AuditClient> = Arc::new(MockAuditClient::always_block());
    let bus = Arc::new(DenialBus::new());
    let events = bus.subscribe();

    let session = fuse::mount(layout.clone(), GuardPolicy::default(), audit, bus).unwrap();
    let path = layout.mount.join("blocked.rs");
    // The write should fail with EACCES once the commit boundary audits.
    let err = std::fs::write(&path, b"fn main() { bad() }\n").unwrap_err();
    assert_eq!(err.raw_os_error(), Some(libc::EACCES));

    // The backing store was never touched.
    assert!(!layout.backing.join("blocked.rs").exists());
    // A Block event was published.
    assert!(matches!(
        events.recv_timeout(Duration::from_secs(2)),
        Ok(jankurai_guard::feedback::GuardEvent::Block { .. })
    ));
    session.unmount();
}

#[test]
#[ignore]
fn poisoned_path_reads_back_the_poison_overlay() {
    let dir = tempfile::tempdir().unwrap();
    let layout = fuse_layout(dir.path());
    layout.ensure_dirs().unwrap();
    let audit: Arc<dyn AuditClient> = Arc::new(MockAuditClient::always_block());
    let bus = Arc::new(DenialBus::new());

    let session = fuse::mount(layout.clone(), GuardPolicy::default(), audit, bus).unwrap();
    let path = layout.mount.join("poisoned.rs");
    let _ = std::fs::write(&path, b"rejected bytes\n");

    // After the block the path serves the poison overlay through the mount.
    let view = std::fs::read(&path).unwrap();
    assert!(jankurai_guard::poison::is_poisoned(&view));
    session.unmount();
}

#[test]
#[ignore]
fn read_passthrough_serves_backing_content() {
    let dir = tempfile::tempdir().unwrap();
    let layout = fuse_layout(dir.path());
    layout.ensure_dirs().unwrap();
    // Seed a file directly in the backing store.
    std::fs::write(layout.backing.join("seed.txt"), b"seeded\n").unwrap();
    let audit: Arc<dyn AuditClient> = Arc::new(MockAuditClient::always_pass());
    let bus = Arc::new(DenialBus::new());

    let session = fuse::mount(layout.clone(), GuardPolicy::default(), audit, bus).unwrap();
    // Reading through the mount returns the backing content unchanged.
    let through = std::fs::read(layout.mount.join("seed.txt")).unwrap();
    assert_eq!(through, b"seeded\n");
    session.unmount();
}
