//! Exhaustive tests for the pure commit-boundary state machine in
//! `jankurai_guard::transaction`. These verify that every event sequence the
//! FUSE and watcher backends can produce collapses into the correct single
//! logical commit.

use jankurai_guard::transaction::{
    CandidateOperation, CommitBoundary, CommitMachine, FsEvent, WriteBuffer,
};
use std::path::PathBuf;

/// Extracts the candidate from an `Audit` boundary, panicking on `NoOp`.
fn expect_audit(boundary: CommitBoundary) -> jankurai_guard::CandidateFile {
    match boundary {
        CommitBoundary::Audit(candidate) => candidate,
        CommitBoundary::NoOp => panic!("expected an Audit boundary, got NoOp"),
    }
}

#[test]
fn otrunc_write_release_is_one_create() {
    let mut machine = CommitMachine::new_file(PathBuf::from("src/new.rs"));
    assert_eq!(
        machine.feed(FsEvent::Open { trunc: true }),
        CommitBoundary::NoOp
    );
    assert_eq!(
        machine.feed(FsEvent::Write {
            off: 0,
            data: b"fn main() {}".to_vec()
        }),
        CommitBoundary::NoOp
    );
    let candidate = expect_audit(machine.feed(FsEvent::Release));
    assert_eq!(candidate.operation, CandidateOperation::Create);
    assert_eq!(candidate.bytes, b"fn main() {}");
    // A second release after the dirty flag is cleared is a NoOp.
    assert_eq!(machine.feed(FsEvent::Release), CommitBoundary::NoOp);
}

#[test]
fn partial_overwrite_materializes_correct_bytes() {
    let mut machine =
        CommitMachine::existing_file(PathBuf::from("data.txt"), b"AAAAAAAAAA".to_vec());
    machine.feed(FsEvent::Write {
        off: 3,
        data: b"ZZ".to_vec(),
    });
    let candidate = expect_audit(machine.feed(FsEvent::Flush));
    assert_eq!(candidate.operation, CandidateOperation::Modify);
    assert_eq!(candidate.bytes, b"AAAZZAAAAA");
    assert!(candidate.preimage_sha256.is_some());
}

#[test]
fn write_tmp_then_rename_keys_on_target() {
    // An editor's atomic save: write everything into a temp file, then rename
    // it over the real target. The candidate must be keyed on the FINAL path.
    let mut machine = CommitMachine::new_file(PathBuf::from("src/.foo.rs.tmp"));
    machine.feed(FsEvent::Write {
        off: 0,
        data: b"final contents".to_vec(),
    });
    let candidate = expect_audit(machine.feed(FsEvent::Rename {
        from: PathBuf::from("src/.foo.rs.tmp"),
        to: PathBuf::from("src/foo.rs"),
    }));
    assert_eq!(candidate.rel_path, PathBuf::from("src/foo.rs"));
    assert_eq!(candidate.bytes, b"final contents");
    match candidate.operation {
        CandidateOperation::Rename { from } => {
            assert_eq!(from, PathBuf::from("src/.foo.rs.tmp"));
        }
        other => panic!("expected Rename operation, got {other:?}"),
    }
}

#[test]
fn fsync_then_release_is_exactly_one_boundary() {
    let mut machine = CommitMachine::new_file(PathBuf::from("a.rs"));
    machine.feed(FsEvent::Write {
        off: 0,
        data: b"x".to_vec(),
    });
    // The first fsync is the commit boundary.
    let first = machine.feed(FsEvent::Fsync);
    assert!(matches!(first, CommitBoundary::Audit(_)));
    // A following flush and release on the now-clean handle are NoOps.
    assert_eq!(machine.feed(FsEvent::Flush), CommitBoundary::NoOp);
    assert_eq!(machine.feed(FsEvent::Release), CommitBoundary::NoOp);
}

#[test]
fn unlink_is_audit_delete() {
    let mut machine = CommitMachine::existing_file(PathBuf::from("gone.rs"), b"old".to_vec());
    let candidate = expect_audit(machine.feed(FsEvent::Unlink));
    assert_eq!(candidate.operation, CandidateOperation::Delete);
    assert!(candidate.bytes.is_empty());
    assert!(candidate.preimage_sha256.is_some());
}

#[test]
fn clean_handle_release_is_noop() {
    // Opening a file read-only and closing it never yields a commit boundary.
    let mut machine = CommitMachine::existing_file(PathBuf::from("readonly.rs"), b"data".to_vec());
    assert_eq!(
        machine.feed(FsEvent::Open { trunc: false }),
        CommitBoundary::NoOp
    );
    assert_eq!(machine.feed(FsEvent::Release), CommitBoundary::NoOp);
}

#[test]
fn write_buffer_extends_and_zero_fills_on_sparse_write() {
    let mut buffer = WriteBuffer::new();
    buffer.apply_write(4, b"abcd");
    let image = buffer.materialize();
    assert_eq!(image.len(), 8);
    assert_eq!(&image[0..4], &[0, 0, 0, 0]);
    assert_eq!(&image[4..8], b"abcd");
}

#[test]
fn write_buffer_truncate_clips_image() {
    let mut buffer = WriteBuffer::with_base(b"0123456789".to_vec());
    buffer.apply_truncate(4);
    assert_eq!(buffer.materialize(), b"0123");
}

#[test]
fn write_buffer_truncate_zero_drops_base() {
    let mut buffer = WriteBuffer::with_base(b"existing".to_vec());
    buffer.apply_truncate(0);
    buffer.apply_write(0, b"fresh");
    assert_eq!(buffer.materialize(), b"fresh");
}

#[test]
fn multiple_writes_at_distinct_offsets_all_land() {
    let mut machine = CommitMachine::existing_file(PathBuf::from("multi.txt"), vec![b'.'; 12]);
    machine.feed(FsEvent::Write {
        off: 0,
        data: b"HEAD".to_vec(),
    });
    machine.feed(FsEvent::Write {
        off: 8,
        data: b"TAIL".to_vec(),
    });
    let candidate = expect_audit(machine.feed(FsEvent::Release));
    assert_eq!(candidate.bytes, b"HEAD....TAIL");
}
