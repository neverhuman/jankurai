//! Inode bookkeeping for the FUSE backend. FUSE addresses files by opaque inode
//! numbers; the kernel hands us an inode and expects us to map it to a concrete
//! backing path. [`InodeTable`] owns that bidirectional mapping plus the lookup
//! refcounts FUSE's `lookup`/`forget` protocol requires.
//!
//! This module is Linux + `fuse`-feature only and is verified on Linux CI.

#![cfg(all(feature = "fuse", target_os = "linux"))]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The inode number FUSE reserves for the mount root.
pub const FUSE_ROOT_INODE: u64 = 1;

/// One inode entry: the backing-relative path it maps to and the kernel's
/// outstanding lookup count for it.
#[derive(Debug, Clone)]
struct InodeEntry {
    rel_path: PathBuf,
    lookup_count: u64,
}

/// A bidirectional inode <-> backing-relative-path table with lookup refcounts.
#[derive(Debug)]
pub struct InodeTable {
    by_ino: HashMap<u64, InodeEntry>,
    by_path: HashMap<PathBuf, u64>,
    next_ino: u64,
}

impl InodeTable {
    /// Creates a table pre-populated with the root inode mapped to the empty
    /// relative path (the mount root itself).
    pub fn new() -> Self {
        let mut by_ino = HashMap::new();
        let mut by_path = HashMap::new();
        by_ino.insert(
            FUSE_ROOT_INODE,
            InodeEntry {
                rel_path: PathBuf::new(),
                lookup_count: 1,
            },
        );
        by_path.insert(PathBuf::new(), FUSE_ROOT_INODE);
        Self {
            by_ino,
            by_path,
            next_ino: FUSE_ROOT_INODE + 1,
        }
    }

    /// Returns the backing-relative path for `ino`, if it is known.
    pub fn path_for(&self, ino: u64) -> Option<&Path> {
        self.by_ino.get(&ino).map(|e| e.rel_path.as_path())
    }

    /// Returns the inode for `rel_path`, if one has been allocated.
    pub fn ino_for(&self, rel_path: &Path) -> Option<u64> {
        self.by_path.get(rel_path).copied()
    }

    /// Resolves `rel_path` to an inode, allocating a fresh one on first sight,
    /// and increments its lookup count. This is the `lookup` path.
    pub fn lookup(&mut self, rel_path: &Path) -> u64 {
        if let Some(&ino) = self.by_path.get(rel_path) {
            if let Some(entry) = self.by_ino.get_mut(&ino) {
                entry.lookup_count += 1;
            }
            return ino;
        }
        let ino = self.next_ino;
        self.next_ino += 1;
        self.by_ino.insert(
            ino,
            InodeEntry {
                rel_path: rel_path.to_path_buf(),
                lookup_count: 1,
            },
        );
        self.by_path.insert(rel_path.to_path_buf(), ino);
        ino
    }

    /// Decrements `ino`'s lookup count by `nlookup`, dropping the entry when the
    /// count reaches zero. The root inode is never dropped. This is the
    /// `forget` path.
    pub fn forget(&mut self, ino: u64, nlookup: u64) {
        if ino == FUSE_ROOT_INODE {
            return;
        }
        let drop_entry = match self.by_ino.get_mut(&ino) {
            Some(entry) => {
                entry.lookup_count = entry.lookup_count.saturating_sub(nlookup);
                entry.lookup_count == 0
            }
            None => false,
        };
        if drop_entry {
            if let Some(entry) = self.by_ino.remove(&ino) {
                self.by_path.remove(&entry.rel_path);
            }
        }
    }

    /// Rebinds `ino` to a new relative path after a rename, keeping the lookup
    /// count intact.
    pub fn rebind(&mut self, ino: u64, new_rel_path: &Path) {
        if let Some(entry) = self.by_ino.get_mut(&ino) {
            self.by_path.remove(&entry.rel_path);
            entry.rel_path = new_rel_path.to_path_buf();
            self.by_path.insert(new_rel_path.to_path_buf(), ino);
        }
    }
}

impl Default for InodeTable {
    fn default() -> Self {
        Self::new()
    }
}
