//! The guarded FUSE filesystem. [`GuardFs`] passes reads through to the backing
//! store (or the poison overlay for a blocked path) and buffers every mutation
//! until a commit boundary, at which point the [`crate::transaction`] state
//! machine yields a candidate that is audited. A passing candidate is committed
//! atomically to the backing store; a blocked candidate leaves the backing
//! store untouched, is quarantined, gets a poison overlay installed, and the
//! mutating syscall returns `EACCES`.
//!
//! This module is Linux + `fuse`-feature only and is verified on Linux CI.

#![cfg(all(feature = "fuse", target_os = "linux"))]

use super::handles::{HandleTable, OpenHandle};
use super::inode::InodeTable;
use crate::audit_client::{AuditClient, Verdict};
use crate::feedback::{report, DenialBus, GuardEvent};
use crate::layout::GuardLayout;
use crate::poison::{self, PoisonState};
use crate::policy::GuardPolicy;
use crate::transaction::CommitBoundary;
use crate::{commit, GuardError};
use fuser::{FileAttr, FileType};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// How long the kernel may cache attributes/entries before re-asking. The
/// `impl fuser::Filesystem` block lives in the sibling `fs_ops` module.
pub(super) const TTL: Duration = Duration::from_secs(1);

/// The guarded filesystem. All shared state is behind a single mutex because
/// FUSE may dispatch requests from multiple threads. The `fuser::Filesystem`
/// implementation is in [`super::fs_ops`].
pub struct GuardFs {
    pub(super) inner: Mutex<FsInner>,
    pub(super) layout: GuardLayout,
    pub(super) policy: GuardPolicy,
    pub(super) audit: Arc<dyn AuditClient>,
    pub(super) bus: Arc<DenialBus>,
}

/// The mutable interior of [`GuardFs`].
pub(super) struct FsInner {
    pub(super) inodes: InodeTable,
    pub(super) handles: HandleTable,
    pub(super) poison: PoisonState,
}

impl GuardFs {
    /// Builds a guarded filesystem over `layout`'s backing store.
    pub fn new(
        layout: GuardLayout,
        policy: GuardPolicy,
        audit: Arc<dyn AuditClient>,
        bus: Arc<DenialBus>,
    ) -> Result<Self, GuardError> {
        let poison = PoisonState::load(&layout.poison_dir())?;
        Ok(Self {
            inner: Mutex::new(FsInner {
                inodes: InodeTable::new(),
                handles: HandleTable::new(),
                poison,
            }),
            layout,
            policy,
            audit,
            bus,
        })
    }

    /// Resolves a backing-relative path to an absolute path in the backing store.
    pub(super) fn backing_path(&self, rel: &Path) -> std::path::PathBuf {
        self.layout.backing.join(rel)
    }

    /// Builds a [`FileAttr`] for `ino` from a backing-store `metadata`.
    pub(super) fn attr_from_meta(ino: u64, meta: &std::fs::Metadata) -> FileAttr {
        use std::os::unix::fs::MetadataExt;
        let kind = if meta.is_dir() {
            FileType::Directory
        } else if meta.file_type().is_symlink() {
            FileType::Symlink
        } else {
            FileType::RegularFile
        };
        FileAttr {
            ino,
            size: meta.len(),
            blocks: meta.blocks(),
            atime: SystemTime::UNIX_EPOCH + Duration::from_secs(meta.atime().max(0) as u64),
            mtime: SystemTime::UNIX_EPOCH + Duration::from_secs(meta.mtime().max(0) as u64),
            ctime: SystemTime::UNIX_EPOCH + Duration::from_secs(meta.ctime().max(0) as u64),
            crtime: SystemTime::UNIX_EPOCH,
            kind,
            perm: (meta.mode() & 0o7777) as u16,
            nlink: meta.nlink() as u32,
            uid: meta.uid(),
            gid: meta.gid(),
            rdev: meta.rdev() as u32,
            blksize: 512,
            flags: 0,
        }
    }

    /// Synthesizes a regular-file [`FileAttr`] of `size` for an overlay-served
    /// (poisoned) path that has no committed backing content.
    pub(super) fn overlay_attr(ino: u64, size: u64) -> FileAttr {
        // SAFETY: `getuid` and `getgid` have no preconditions, never fail, and
        // always return the process's real UID/GID.  Calling them is always sound.
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        FileAttr {
            ino,
            size,
            blocks: size.div_ceil(512),
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid,
            gid,
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }

    /// Drives a commit-boundary event on a write handle and replies with the
    /// resulting errno. Used by the `fsync` and `flush` syscall handlers.
    pub(super) fn commit_handle(
        &self,
        fh: u64,
        event: crate::transaction::FsEvent,
        reply: fuser::ReplyEmpty,
    ) {
        let mut inner = self.inner.lock().expect("guard fs mutex");
        let boundary = match inner.handles.get_mut(fh) {
            Some(OpenHandle::Write { machine, .. }) => machine.feed(event),
            _ => {
                reply.ok();
                return;
            }
        };
        let errno = self.process_boundary(&mut inner, boundary);
        if errno == 0 {
            reply.ok();
        } else {
            reply.error(errno);
        }
    }

    /// Processes a commit boundary: stages the candidate, audits it, and either
    /// commits it to the backing store or quarantines + poisons it. Returns the
    /// errno the mutating syscall should report (`0` on success).
    pub(super) fn process_boundary(&self, inner: &mut FsInner, boundary: CommitBoundary) -> i32 {
        let candidate = match boundary {
            CommitBoundary::NoOp => return 0,
            CommitBoundary::Audit(candidate) => candidate,
        };
        let rel = candidate.rel_path.clone();
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if self.policy.is_excluded(&rel_str) {
            // Excluded paths bypass audit and land directly.
            if commit::atomic_commit(&self.layout.backing, &rel, &candidate.bytes).is_err() {
                return libc::EIO;
            }
            return 0;
        }

        let decision = match self
            .audit
            .audit(&self.layout.backing, &rel, &candidate.bytes)
        {
            Ok(decision) => decision,
            Err(e) => {
                self.bus.publish(GuardEvent::Error {
                    rel_path: rel.clone(),
                    message: e.to_string(),
                });
                return libc::EACCES;
            }
        };

        match decision.verdict {
            Verdict::Pass | Verdict::Advisory => {
                if commit::atomic_commit(&self.layout.backing, &rel, &candidate.bytes).is_err() {
                    return libc::EIO;
                }
                if inner.poison.clear(&rel) {
                    let _ = inner.poison.save(&self.layout.poison_dir());
                }
                self.bus.publish(GuardEvent::Pass { rel_path: rel });
                0
            }
            Verdict::Block => {
                let report_path =
                    match report::write_failure_report(&self.layout.repo_root, &decision) {
                        Ok(path) => path,
                        Err(_) => self
                            .layout
                            .repo_root
                            .join(".jankurai/guard/LAST_FAILURE.md"),
                    };
                let _ =
                    commit::quarantine_candidate(&self.layout.repo_root, &rel, &candidate.bytes);
                let content = poison_content(&decision, &report_path);
                let view = poison::wrap(&rel, &candidate.bytes, &content);
                inner.poison.insert(&rel, &view);
                let _ = inner.poison.save(&self.layout.poison_dir());
                self.bus.publish(GuardEvent::Block {
                    rel_path: rel,
                    decision: Box::new(decision),
                    report_path,
                });
                libc::EACCES
            }
        }
    }
}

/// Builds the poison header content from an audit decision.
fn poison_content(
    decision: &crate::audit_client::GuardDecision,
    report_path: &Path,
) -> poison::Content {
    let blocking: Vec<_> = decision.blocking.all().collect();
    poison::Content {
        path: decision.path.clone(),
        rule_ids: blocking.iter().map(|f| f.rule_id.clone()).collect(),
        problems: blocking.iter().map(|f| f.problem.clone()).collect(),
        fix_steps: blocking
            .iter()
            .map(|f| f.agent_fix.clone())
            .filter(|s| !s.is_empty())
            .collect(),
        rerun_command: decision.rerun_command.clone(),
        report_path: report_path.to_string_lossy().into_owned(),
    }
}
