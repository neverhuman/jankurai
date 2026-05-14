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
use std::fs::File;
use std::path::PathBuf;

/// An open file as the guard tracks it.
pub enum OpenHandle {
    /// A read-only handle served straight from the backing store.
    Read {
        /// The backing-relative path being read.
        rel_path: PathBuf,
        /// The open backing file the reads are served from.
        file: File,
    },
    /// A write handle whose mutations are buffered and audited on commit.
    Write {
        /// The backing-relative path being written.
        rel_path: PathBuf,
        /// The pure commit-boundary state machine driving this handle.
        machine: CommitMachine,
    },
}

impl OpenHandle {
    /// Returns the backing-relative path the handle refers to.
    pub fn rel_path(&self) -> &PathBuf {
        match self {
            OpenHandle::Read { rel_path, .. } => rel_path,
            OpenHandle::Write { rel_path, .. } => rel_path,
        }
    }

    /// Returns `true` for a write handle.
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
    pub fn is_dirty_write(&self, fh: u64) -> bool {
        matches!(
            self.handles.get(&fh),
            Some(OpenHandle::Write { machine, .. }) if machine.buffer().dirty
        )
    }
}
