//! Read-path helpers for [`GuardFs`]: the logic backing the `lookup`, `forget`,
//! `getattr`, `open`, `read`, `readdir`, and `readlink` FUSE handlers. The
//! actual `fuser::Filesystem` trait impl that calls these lives in
//! [`super::fs_ops`] so the two files together stay under the module-size
//! budget while all trait methods remain in one coherent impl block.
//!
//! This module is Linux + `fuse`-feature only.

#![cfg(all(feature = "fuse", target_os = "linux"))]

use super::filesystem::{GuardFs, TTL};
use super::handles::OpenHandle;
use crate::transaction::{CommitMachine, FsEvent};
use fuser::{FileType, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen};
use std::ffi::OsStr;

impl GuardFs {
    /// Handles the `lookup` FUSE call.
    pub(super) fn handle_lookup(&mut self, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let mut inner = self.inner.lock().expect("guard fs mutex");
        let parent_rel = match inner.inodes.path_for(parent) {
            Some(rel) => rel.to_path_buf(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let rel = parent_rel.join(name);
        if let Some(view) = inner.poison.get(&rel) {
            let ino = inner.inodes.lookup(&rel);
            reply.entry(&TTL, &GuardFs::overlay_attr(ino, view.len() as u64), 0);
            return;
        }
        match std::fs::symlink_metadata(self.backing_path(&rel)) {
            Ok(meta) => {
                let ino = inner.inodes.lookup(&rel);
                reply.entry(&TTL, &GuardFs::attr_from_meta(ino, &meta), 0);
            }
            Err(_) => reply.error(libc::ENOENT),
        }
    }

    /// Handles the `forget` FUSE call.
    pub(super) fn handle_forget(&mut self, ino: u64, nlookup: u64) {
        let mut inner = self.inner.lock().expect("guard fs mutex");
        inner.inodes.forget(ino, nlookup);
    }

    /// Handles the `getattr` FUSE call.
    pub(super) fn handle_getattr(&mut self, ino: u64, reply: ReplyAttr) {
        let inner = self.inner.lock().expect("guard fs mutex");
        let rel = match inner.inodes.path_for(ino) {
            Some(rel) => rel.to_path_buf(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        if let Some(view) = inner.poison.get(&rel) {
            reply.attr(&TTL, &GuardFs::overlay_attr(ino, view.len() as u64));
            return;
        }
        match std::fs::symlink_metadata(self.backing_path(&rel)) {
            Ok(meta) => reply.attr(&TTL, &GuardFs::attr_from_meta(ino, &meta)),
            Err(_) => reply.error(libc::ENOENT),
        }
    }

    /// Handles the `open` FUSE call.
    pub(super) fn handle_open(&mut self, ino: u64, flags: i32, reply: ReplyOpen) {
        let mut inner = self.inner.lock().expect("guard fs mutex");
        let rel = match inner.inodes.path_for(ino) {
            Some(rel) => rel.to_path_buf(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let write_intent = flags & (libc::O_WRONLY | libc::O_RDWR) != 0;
        if write_intent {
            let base = std::fs::read(self.backing_path(&rel)).unwrap_or_default();
            let existed = self.backing_path(&rel).exists();
            let mut machine = if existed {
                CommitMachine::existing_file(rel.clone(), base)
            } else {
                CommitMachine::new_file(rel.clone())
            };
            if flags & libc::O_TRUNC != 0 {
                machine.feed(FsEvent::Open { trunc: true });
            }
            let fh = inner.handles.insert(OpenHandle::Write { machine });
            reply.opened(fh, 0);
        } else {
            if self.backing_path(&rel).exists() {
                let fh = inner.handles.insert(OpenHandle::Read);
                reply.opened(fh, 0);
            } else {
                reply.error(libc::ENOENT);
            }
        }
    }

    /// Handles the `read` FUSE call.
    pub(super) fn handle_read(&mut self, ino: u64, offset: i64, size: u32, reply: ReplyData) {
        let inner = self.inner.lock().expect("guard fs mutex");
        let rel = match inner.inodes.path_for(ino) {
            Some(rel) => rel.to_path_buf(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let bytes = match inner.poison.get(&rel) {
            Some(view) => view,
            None => match std::fs::read(self.backing_path(&rel)) {
                Ok(bytes) => bytes,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            },
        };
        let start = (offset.max(0) as usize).min(bytes.len());
        let end = (start + size as usize).min(bytes.len());
        reply.data(&bytes[start..end]);
    }

    /// Handles the `readdir` FUSE call.
    pub(super) fn handle_readdir(&mut self, ino: u64, offset: i64, mut reply: ReplyDirectory) {
        let inner = self.inner.lock().expect("guard fs mutex");
        let rel = match inner.inodes.path_for(ino) {
            Some(rel) => rel.to_path_buf(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let dir = self.backing_path(&rel);
        let mut entries: Vec<(FileType, String)> = vec![
            (FileType::Directory, ".".to_string()),
            (FileType::Directory, "..".to_string()),
        ];
        if let Ok(read_dir) = std::fs::read_dir(&dir) {
            for entry in read_dir.flatten() {
                let kind = match entry.file_type() {
                    Ok(ft) if ft.is_dir() => FileType::Directory,
                    Ok(ft) if ft.is_symlink() => FileType::Symlink,
                    _ => FileType::RegularFile,
                };
                entries.push((kind, entry.file_name().to_string_lossy().into_owned()));
            }
        }
        for (idx, (kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(ino, (idx + 1) as i64, kind, &name) {
                break;
            }
        }
        reply.ok();
    }

    /// Handles the `readlink` FUSE call.
    pub(super) fn handle_readlink(&mut self, ino: u64, reply: ReplyData) {
        let inner = self.inner.lock().expect("guard fs mutex");
        let rel = match inner.inodes.path_for(ino) {
            Some(rel) => rel.to_path_buf(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        match std::fs::read_link(self.backing_path(&rel)) {
            Ok(target) => reply.data(target.to_string_lossy().as_bytes()),
            Err(_) => reply.error(libc::EINVAL),
        }
    }
}
