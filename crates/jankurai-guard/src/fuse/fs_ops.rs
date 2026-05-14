//! The single `fuser::Filesystem` impl for [`GuardFs`]. This file contains the
//! complete trait impl but delegates the read-path logic to helpers in
//! [`super::ops_read`] and the write/mutation logic to helpers defined here,
//! so neither file grows past the module-size budget.
//!
//! This module is Linux + `fuse`-feature only and is verified on Linux CI.

#![cfg(all(feature = "fuse", target_os = "linux"))]

use super::filesystem::GuardFs;
use super::handles::OpenHandle;
use crate::transaction::{CommitMachine, FsEvent};
use fuser::{Filesystem, ReplyCreate, ReplyEmpty, ReplyWrite, Request};
use std::ffi::OsStr;
use std::path::Path;

// ── write/mutation helpers ────────────────────────────────────────────────────

impl GuardFs {
    /// Feeds write data into the open handle's buffer.
    fn do_write(&mut self, fh: u64, offset: i64, data: &[u8], reply: ReplyWrite) {
        let mut inner = self.inner.lock().expect("guard fs mutex");
        match inner.handles.get_mut(fh) {
            Some(OpenHandle::Write { machine, .. }) => {
                machine.feed(FsEvent::Write {
                    off: offset.max(0) as u64,
                    data: data.to_vec(),
                });
                reply.written(data.len() as u32);
            }
            _ => reply.error(libc::EBADF),
        }
    }

    /// Registers a new write handle for a newly created file.
    fn do_create(&mut self, parent: u64, name: &OsStr, reply: ReplyCreate) {
        use super::filesystem::TTL;
        let mut inner = self.inner.lock().expect("guard fs mutex");
        let parent_rel = match inner.inodes.path_for(parent) {
            Some(rel) => rel.to_path_buf(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let rel = parent_rel.join(name);
        let ino = inner.inodes.lookup(&rel);
        let machine = CommitMachine::new_file(rel.clone());
        let fh = inner.handles.insert(OpenHandle::Write { machine });
        reply.created(&TTL, &GuardFs::overlay_attr(ino, 0), 0, fh, 0);
    }

    /// Runs an unlink through the commit machine.
    fn do_unlink(&mut self, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let mut inner = self.inner.lock().expect("guard fs mutex");
        let parent_rel = match inner.inodes.path_for(parent) {
            Some(rel) => rel.to_path_buf(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let rel = parent_rel.join(name);
        let base = self.read_or_empty(&rel);
        let mut machine = CommitMachine::existing_file(rel.clone(), base);
        let boundary = machine.feed(FsEvent::Unlink);
        let errno = self.process_boundary(&mut inner, boundary);
        if errno == 0 {
            let _ = std::fs::remove_file(self.backing_path(&rel));
            reply.ok();
        } else {
            reply.error(errno);
        }
    }

    /// Runs a rename through the commit machine.
    fn do_rename(
        &mut self,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        reply: ReplyEmpty,
    ) {
        let mut inner = self.inner.lock().expect("guard fs mutex");
        let from_parent = inner.inodes.path_for(parent).map(Path::to_path_buf);
        let to_parent = inner.inodes.path_for(newparent).map(Path::to_path_buf);
        let (from_parent, to_parent) = match (from_parent, to_parent) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let from = from_parent.join(name);
        let to = to_parent.join(newname);
        let base = self.read_or_empty(&from);
        let mut machine = CommitMachine::existing_file(from.clone(), base);
        let boundary = machine.feed(FsEvent::Rename {
            from: from.clone(),
            to: to.clone(),
        });
        let errno = self.process_boundary(&mut inner, boundary);
        if errno == 0 {
            let _ = std::fs::remove_file(self.backing_path(&from));
            if let Some(ino) = inner.inodes.ino_for(&from) {
                inner.inodes.rebind(ino, &to);
            }
            reply.ok();
        } else {
            reply.error(errno);
        }
    }
}

// ── Filesystem trait impl ─────────────────────────────────────────────────────
// Read-path handlers delegate to helpers defined in ops_read.rs.
// Write/mutation handlers delegate to helpers defined above.

impl Filesystem for GuardFs {
    // ── read-path ────────────────────────────────────────────────────────────

    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEntry) {
        self.handle_lookup(parent, name, reply);
    }

    fn forget(&mut self, _req: &Request<'_>, ino: u64, nlookup: u64) {
        self.handle_forget(ino, nlookup);
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: fuser::ReplyAttr) {
        self.handle_getattr(ino, reply);
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: fuser::ReplyOpen) {
        self.handle_open(ino, flags, reply);
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        self.handle_read(ino, offset, size, reply);
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        reply: fuser::ReplyDirectory,
    ) {
        self.handle_readdir(ino, offset, reply);
    }

    fn readlink(&mut self, _req: &Request<'_>, ino: u64, reply: fuser::ReplyData) {
        self.handle_readlink(ino, reply);
    }

    // ── write/mutation ────────────────────────────────────────────────────────

    fn write(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyWrite,
    ) {
        self.do_write(fh, offset, data, reply);
    }

    fn fsync(&mut self, _req: &Request<'_>, _ino: u64, fh: u64, _ds: bool, reply: ReplyEmpty) {
        self.commit_handle(fh, FsEvent::Fsync, reply);
    }

    fn flush(&mut self, _req: &Request<'_>, _ino: u64, fh: u64, _lo: u64, reply: ReplyEmpty) {
        self.commit_handle(fh, FsEvent::Flush, reply);
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        let mut inner = self.inner.lock().expect("guard fs mutex");
        let handle = inner.handles.remove(fh);
        match handle {
            Some(OpenHandle::Write { mut machine, .. }) => {
                let boundary = machine.feed(FsEvent::Release);
                let errno = self.process_boundary(&mut inner, boundary);
                if errno == 0 {
                    reply.ok();
                } else {
                    reply.error(errno);
                }
            }
            _ => reply.ok(),
        }
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        self.do_create(parent, name, reply);
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        self.do_unlink(parent, name, reply);
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        _flags: u32,
        reply: ReplyEmpty,
    ) {
        self.do_rename(parent, name, newparent, newname, reply);
    }
}
