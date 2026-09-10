// OS primitives for lock recovery. Compiled directly by the pinned rustc;
// no Cargo, package bootstrap, dependency resolution, or inherited environment.
use std::ffi::{CString, c_char};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};
use std::process::Command;

#[cfg(target_os = "linux")]
const NOFOLLOW: i32 = 0x20000;
#[cfg(target_os = "macos")]
const NOFOLLOW: i32 = 0x100;
#[cfg(target_os = "linux")]
const CREATE: i32 = 0x40;
#[cfg(target_os = "macos")]
const CREATE: i32 = 0x200;

unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
    fn fcntl(fd: i32, command: i32, ...) -> i32;
    fn openat(fd: i32, name: *const c_char, flags: i32, ...) -> i32;
    #[cfg(target_os = "linux")]
    fn renameat2(left_dir: i32, left: *const c_char, right_dir: i32, right: *const c_char, flags: u32) -> i32;
    #[cfg(target_os = "macos")]
    fn renameatx_np(left_dir: i32, left: *const c_char, right_dir: i32, right: *const c_char, flags: u32) -> i32;
}

fn open_directory(path: &Path) -> io::Result<File> {
    if !path.is_absolute() { return Err(io::Error::other("native directory must be absolute")); }
    let mut directory = File::open("/")?;
    for component in path.components() {
        let name = match component {
            Component::RootDir => continue,
            Component::Normal(name) => CString::new(name.as_bytes()).map_err(io::Error::other)?,
            _ => return Err(io::Error::other("unsafe native directory component")),
        };
        // Walk from directory descriptors: an ancestor rename cannot redirect
        // the remaining walk, and no component may be a symlink.
        let fd = unsafe { openat(directory.as_raw_fd(), name.as_ptr(), NOFOLLOW) };
        if fd < 0 { return Err(io::Error::last_os_error()); }
        let next = unsafe { File::from_raw_fd(fd) };
        if !next.metadata()?.is_dir() { return Err(io::Error::other("native parent is not a directory")); }
        if unsafe { fcntl(fd, 2, 1) } < 0 { return Err(io::Error::last_os_error()); }
        directory = next;
    }
    Ok(directory)
}

fn exchange(left: &Path, right: &Path) -> io::Result<()> {
    let open_parent = |file: &Path| -> io::Result<File> {
        let directory = open_directory(file.parent().ok_or_else(|| io::Error::other("missing parent"))?)?;
        if !directory.metadata()?.is_dir() || !std::fs::symlink_metadata(file)?.is_file() {
            return Err(io::Error::other("atomic exchange requires directories and regular files"));
        }
        Ok(directory)
    };
    let left_dir = open_parent(left)?;
    let right_dir = open_parent(right)?;
    let name = |p: &Path| -> io::Result<CString> {
        CString::new(p.file_name().ok_or_else(|| io::Error::other("missing file name"))?.as_bytes())
            .map_err(io::Error::other)
    };
    let left_name = name(left)?;
    let right_name = name(right)?;
    // Both names survive the syscall, including post-check concurrent edits.
    #[cfg(target_os = "linux")]
    let result = unsafe { renameat2(left_dir.as_raw_fd(), left_name.as_ptr(), right_dir.as_raw_fd(), right_name.as_ptr(), 2) };
    #[cfg(target_os = "macos")]
    let result = unsafe { renameatx_np(left_dir.as_raw_fd(), left_name.as_ptr(), right_dir.as_raw_fd(), right_name.as_ptr(), 2) };
    if result != 0 { return Err(io::Error::last_os_error()); }
    left_dir.sync_all()?;
    right_dir.sync_all()
}

fn lock_identity(file: &File, lock: &Path) -> io::Result<()> {
    let held = file.metadata()?;
    if !held.is_file() { return Err(io::Error::other("refusing non-regular recovery lock")); }
    if unsafe { flock(file.as_raw_fd(), 2 | 4) } != 0 { return Err(io::Error::last_os_error()); }
    let current = std::fs::symlink_metadata(lock)?;
    if !current.is_file() || (held.dev(), held.ino()) != (current.dev(), current.ino()) {
        return Err(io::Error::other("recovery lock inode changed"));
    }
    Ok(())
}

fn recover(lock: &Path, node: &str, module: &str, hub: &str, command: &str) -> io::Result<i32> {
    // The permanent inode is never removed. The child inherits this flock, so
    // interrupting the supervisor cannot admit another recovery writer.
    let directory = open_directory(lock.parent().ok_or_else(|| io::Error::other("missing lock parent"))?)?;
    let name = CString::new(lock.file_name().ok_or_else(|| io::Error::other("missing lock name"))?.as_bytes())
        .map_err(io::Error::other)?;
    let fd = unsafe { openat(directory.as_raw_fd(), name.as_ptr(), 2 | CREATE | NOFOLLOW, 0o600u32) };
    if fd < 0 { return Err(io::Error::last_os_error()); }
    let file = unsafe { File::from_raw_fd(fd) };
    lock_identity(&file, lock)?;
    // File opens with CLOEXEC. Clear only that flag for the one owned lease.
    let flags = unsafe { fcntl(file.as_raw_fd(), 1) };
    if flags < 0 || unsafe { fcntl(file.as_raw_fd(), 2, flags & !1) } < 0 {
        return Err(io::Error::last_os_error());
    }
    let status = Command::new(node).args([module, "--recovery-worker", hub, command])
        .env("JANKURAI_RECOVERY_FD", file.as_raw_fd().to_string())
        .env("JANKURAI_RECOVERY_PARENT", std::process::id().to_string())
        .status()?;
    Ok(status.code().unwrap_or(1))
}

#[cfg(target_os = "macos")]
fn darwin_process(pid: i32) -> io::Result<()> {
    // Public PROC_PIDTBSDINFO has kernel start seconds and microseconds.
    // A coarse ps timestamp cannot disambiguate PID reuse.
    #[repr(C)]
    #[derive(Default)]
    struct BsdInfo {
        prefix: [u32; 12], comm: [u8; 16], name: [u8; 32], details: [u32; 6],
        start_seconds: u64, start_microseconds: u64,
    }
    #[link(name = "proc")]
    unsafe extern "C" {
        fn proc_pidinfo(pid: i32, flavor: i32, arg: u64, buffer: *mut BsdInfo, size: i32) -> i32;
    }
    let mut info = BsdInfo::default();
    let size = std::mem::size_of::<BsdInfo>() as i32;
    let result = unsafe { proc_pidinfo(pid, 3, 0, &mut info, size) };
    if result != size {
        let error = io::Error::last_os_error();
        if result == 0 && error.raw_os_error() == Some(3) {
            println!("{{\"stopped\":true}}");
            return Ok(());
        }
        return Err(io::Error::other(format!("cannot establish Darwin process identity: result={result}, {error}")));
    }
    if info.prefix[3] != pid as u32 || info.start_seconds == 0 {
        return Err(io::Error::other("invalid Darwin process identity"));
    }
    println!("{{\"startTime\":\"{}:{}\"}}", info.start_seconds, info.start_microseconds);
    Ok(())
}

fn run(args: &[String]) -> io::Result<i32> {
    match args {
        [_, operation, left, right] if operation == "exchange" => exchange(Path::new(left), Path::new(right)).map(|()| 0),
        [_, operation, lock, node, module, hub, command] if operation == "recover" => recover(Path::new(lock), node, module, hub, command),
        [_, operation, lock] if operation == "validate-lease" => {
            // Node explicitly passes its lease as fd 3. Re-locking the shared
            // open file description is idempotent; a separately opened fd
            // cannot evade another writer's lock with forged environment data.
            if unsafe { fcntl(3, 1) } < 0 { return Err(io::Error::last_os_error()); }
            let file = unsafe { File::from_raw_fd(3) };
            lock_identity(&file, Path::new(lock)).map(|()| 0)
        }
        #[cfg(target_os = "macos")]
        [_, operation, pid] if operation == "darwin-process" => {
            let pid = pid.parse::<i32>().map_err(io::Error::other)?;
            if pid <= 0 { return Err(io::Error::other("invalid process id")); }
            darwin_process(pid).map(|()| 0)
        }
        _ => Err(io::Error::other("invalid family native operation")),
    }
}

fn main() {
    match run(&std::env::args().collect::<Vec<_>>()) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("family native operation refused: {error}");
            std::process::exit(1);
        }
    }
}
