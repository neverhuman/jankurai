//! The pure commit-boundary state machine. This module models how a sequence of
//! filesystem events on a guarded path collapses into exactly one logical commit
//! that must be audited. It has no dependency on FUSE or the watcher: it is the
//! shared, exhaustively-tested core that both backends drive.
//!
//! A "commit boundary" is the moment the guard must materialize a candidate's
//! final bytes and submit them to audit. There are three boundary shapes:
//! a direct write boundary (first `fsync`/`flush`/`release` on a dirty write
//! handle), an atomic-save boundary (a `rename` of a staging file over a target),
//! and a delete boundary (an `unlink`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// An accumulating buffer of writes against a base image. Writes are stored
/// sparsely keyed by offset; [`WriteBuffer::materialize`] flattens them into the
/// final byte image the candidate would have on disk.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteBuffer {
    /// The pre-existing file contents the writes are layered over.
    pub base: Vec<u8>,
    /// Sparse writes keyed by their starting offset.
    pub writes: BTreeMap<u64, Vec<u8>>,
    /// Whether the file was truncated to zero before the buffered writes.
    pub truncated: bool,
    /// Whether the buffer holds an un-committed mutation.
    pub dirty: bool,
    /// A pending non-zero truncation length applied at materialize time.
    pub pending_truncate_len: Option<u64>,
}

impl WriteBuffer {
    /// Creates an empty buffer for a brand-new file.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a buffer layered over the existing `base` bytes of a file.
    pub fn with_base(base: Vec<u8>) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    /// Records a write of `data` at `offset`, marking the buffer dirty.
    pub fn apply_write(&mut self, offset: u64, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.writes.insert(offset, data.to_vec());
        self.dirty = true;
    }

    /// Records a truncation to `len`. A truncation to zero sets `truncated` so
    /// the base image is dropped on materialize; a non-zero truncation clips the
    /// materialized image.
    pub fn apply_truncate(&mut self, len: u64) {
        if len == 0 {
            self.truncated = true;
            self.base.clear();
            self.writes.clear();
            self.pending_truncate_len = None;
        } else {
            self.pending_truncate_len = Some(len);
        }
        self.dirty = true;
    }

    /// Flattens base + sparse writes into the final byte image.
    pub fn materialize(&self) -> Vec<u8> {
        let mut image = if self.truncated {
            Vec::new()
        } else {
            self.base.clone()
        };
        for (&offset, data) in &self.writes {
            let start = offset as usize;
            let end = start + data.len();
            if end > image.len() {
                image.resize(end, 0);
            }
            image[start..end].copy_from_slice(data);
        }
        if let Some(len) = self.pending_truncate_len {
            let len = len as usize;
            if image.len() > len {
                image.truncate(len);
            } else if image.len() < len {
                image.resize(len, 0);
            }
        }
        image
    }

    /// Clears the dirty flag once a commit boundary has consumed the buffer.
    pub fn mark_committed(&mut self) {
        self.dirty = false;
    }
}

/// The kind of mutation a candidate represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CandidateOperation {
    /// A file that did not exist before.
    Create,
    /// An overwrite of an existing file.
    Modify,
    /// A removal of an existing file.
    Delete,
    /// A move from another path onto this candidate's path.
    Rename {
        /// The original path the candidate was renamed from.
        from: PathBuf,
    },
}

/// A single-file jankurai candidate: the final bytes of one file together with
/// the operation that produced it. This is the unit submitted to the audit
/// engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateFile {
    /// Repo-relative path of the candidate.
    pub rel_path: PathBuf,
    /// The final byte image of the candidate (empty for a delete).
    #[serde(with = "byte_vec")]
    pub bytes: Vec<u8>,
    /// The mutation that produced the candidate.
    pub operation: CandidateOperation,
    /// SHA-256 of the file's pre-image, when it existed before the mutation.
    pub preimage_sha256: Option<String>,
}

mod byte_vec {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&String::from_utf8_lossy(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        Ok(s.into_bytes())
    }
}

/// The outcome of feeding an event to the [`CommitMachine`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitBoundary {
    /// The event produced a logical commit that must be audited.
    Audit(CandidateFile),
    /// The event did not produce a commit boundary.
    NoOp,
}

/// A filesystem event observed against a guarded path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsEvent {
    /// A handle was opened; `trunc` is true for `O_TRUNC`.
    Open {
        /// Whether the open requested truncation.
        trunc: bool,
    },
    /// Bytes were written at an offset.
    Write {
        /// Byte offset of the write.
        off: u64,
        /// The written bytes.
        data: Vec<u8>,
    },
    /// The file was truncated to `len`.
    Truncate {
        /// New length after truncation.
        len: u64,
    },
    /// The handle was fsync'd.
    Fsync,
    /// The handle was flushed.
    Flush,
    /// The handle was released (closed).
    Release,
    /// A rename from `from` onto `to`.
    Rename {
        /// Source path of the rename.
        from: PathBuf,
        /// Destination path of the rename.
        to: PathBuf,
    },
    /// The file was unlinked.
    Unlink,
}

/// Drives a single guarded path's [`WriteBuffer`] through filesystem events,
/// yielding a [`CommitBoundary`] for each event. Direct-write boundaries fire on
/// the first `fsync`/`flush`/`release` of a dirty handle and clear the dirty
/// flag so a later `release` is a [`CommitBoundary::NoOp`].
#[derive(Debug, Clone)]
pub struct CommitMachine {
    rel_path: PathBuf,
    buffer: WriteBuffer,
    existed: bool,
    preimage_sha256: Option<String>,
}

impl CommitMachine {
    /// Creates a machine for a brand-new file at `rel_path`.
    pub fn new_file(rel_path: PathBuf) -> Self {
        Self {
            rel_path,
            buffer: WriteBuffer::new(),
            existed: false,
            preimage_sha256: None,
        }
    }

    /// Creates a machine for an existing file at `rel_path` with `base` bytes.
    pub fn existing_file(rel_path: PathBuf, base: Vec<u8>) -> Self {
        let preimage_sha256 = Some(sha256_hex(&base));
        Self {
            rel_path,
            buffer: WriteBuffer::with_base(base),
            existed: true,
            preimage_sha256,
        }
    }

    /// Read-only access to the accumulating buffer.
    pub fn buffer(&self) -> &WriteBuffer {
        &self.buffer
    }

    /// Feeds one filesystem event to the machine.
    pub fn feed(&mut self, event: FsEvent) -> CommitBoundary {
        match event {
            FsEvent::Open { trunc } => {
                if trunc {
                    self.buffer.apply_truncate(0);
                }
                CommitBoundary::NoOp
            }
            FsEvent::Write { off, data } => {
                self.buffer.apply_write(off, &data);
                CommitBoundary::NoOp
            }
            FsEvent::Truncate { len } => {
                self.buffer.apply_truncate(len);
                CommitBoundary::NoOp
            }
            FsEvent::Fsync | FsEvent::Flush | FsEvent::Release => self.direct_boundary(),
            FsEvent::Rename { from, to } => self.rename_boundary(from, to),
            FsEvent::Unlink => self.delete_boundary(),
        }
    }

    fn direct_boundary(&mut self) -> CommitBoundary {
        if !self.buffer.dirty {
            return CommitBoundary::NoOp;
        }
        self.buffer.mark_committed();
        let operation = if self.existed {
            CandidateOperation::Modify
        } else {
            CandidateOperation::Create
        };
        CommitBoundary::Audit(CandidateFile {
            rel_path: self.rel_path.clone(),
            bytes: self.buffer.materialize(),
            operation,
            preimage_sha256: self.preimage_sha256.clone(),
        })
    }

    fn rename_boundary(&mut self, from: PathBuf, to: PathBuf) -> CommitBoundary {
        // An atomic-save sequence: the staging file's buffer becomes the candidate
        // keyed on the final target path. The buffer is consumed regardless of
        // whether it was already flushed, because the rename is the true commit.
        self.buffer.mark_committed();
        let bytes = self.buffer.materialize();
        self.rel_path = to.clone();
        CommitBoundary::Audit(CandidateFile {
            rel_path: to,
            bytes,
            operation: CandidateOperation::Rename { from },
            preimage_sha256: self.preimage_sha256.clone(),
        })
    }

    fn delete_boundary(&mut self) -> CommitBoundary {
        self.buffer.mark_committed();
        CommitBoundary::Audit(CandidateFile {
            rel_path: self.rel_path.clone(),
            bytes: Vec::new(),
            operation: CandidateOperation::Delete,
            preimage_sha256: self.preimage_sha256.clone(),
        })
    }
}

/// Computes the lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
