//! Open-file-handle bookkeeping for the FUSE backend. FUSE addresses open files
//! by an opaque file handle (`fh`) the filesystem chooses at `open`/`create`
//! time. [`HandleTable`] maps each `fh` to an [`OpenHandle`], which is either a
//! passthrough read against the backing store or a buffered write driven by the
//! pure [`crate::transaction`] state machine.
//!
//! This module is Linux + `fuse`-feature only and is verified on Linux CI.

#![cfg(all(feature = "fuse", target_os = "linux"))]

use crate::transaction::CommitMachine;
use std::collections::HashMap;

/// An open file as the guard tracks it.
pub enum OpenHandle {
    /// A read-only handle. Reads are served directly from the backing store by
    /// path; no file descriptor is cached here because every `read` call uses
    /// the current backing state (including poison overlays).
    Read,
    /// A write handle whose mutations are buffered and audited on commit.
    Write {
        /// The pure commit-boundary state machine driving this handle.
        machine: CommitMachine,
    },
}

impl OpenHandle {
    /// Returns `true` for a write handle.
    #[allow(dead_code)]
    pub fn is_write(&self) -> bool {
        matches!(self, OpenHandle::Write { .. })
    }
}

/// A table of open handles keyed by FUSE file handle.
#[derive(Default)]
pub struct HandleTable {
    handles: HashMap<u64, OpenHandle>,
    next_fh: u64,
}

impl HandleTable {
    /// Creates an empty handle table.
    pub fn new() -> Self {
        Self {
            handles: HashMap::new(),
            next_fh: 1,
        }
    }

    /// Inserts `handle`, returning the freshly allocated file handle.
    pub fn insert(&mut self, handle: OpenHandle) -> u64 {
        let fh = self.next_fh;
        self.next_fh += 1;
        self.handles.insert(fh, handle);
        fh
    }

    /// Borrows the handle for `fh`.
    #[allow(dead_code)]
    pub fn get(&self, fh: u64) -> Option<&OpenHandle> {
        self.handles.get(&fh)
    }

    /// Mutably borrows the handle for `fh`.
    pub fn get_mut(&mut self, fh: u64) -> Option<&mut OpenHandle> {
        self.handles.get_mut(&fh)
    }

    /// Removes and returns the handle for `fh` (the `release` path).
    pub fn remove(&mut self, fh: u64) -> Option<OpenHandle> {
        self.handles.remove(&fh)
    }

    /// Returns `true` when `fh` refers to a dirty write handle whose buffer
    /// still holds an un-committed mutation.
    #[allow(dead_code)]
    pub fn is_dirty_write(&self, fh: u64) -> bool {
        matches!(
            self.handles.get(&fh),
            Some(OpenHandle::Write { machine, .. }) if machine.buffer().dirty
        )
    }
}
