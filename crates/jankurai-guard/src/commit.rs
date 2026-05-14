//! Durable filesystem commits. This module performs the side-effecting parts of
//! a guard decision: atomically landing accepted bytes in the backing store,
//! reverting a blocked file to its last-good snapshot, quarantining a rejected
//! candidate, and saving/restoring content-addressed snapshots.
//!
//! All "atomic" writes follow the staging-write + fsync + rename + parent-fsync
//! pattern so a crash never leaves a half-written file visible.

use crate::transaction::sha256_hex;
use crate::GuardError;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Atomically writes `bytes` to `<backing_root>/<rel_path>`. The bytes are first
/// written to a sibling staging file, fsync'd, renamed over the target, and the
/// parent directory is fsync'd so the rename is durable. Parent directories are
/// created as needed.
pub fn atomic_commit(backing_root: &Path, rel_path: &Path, bytes: &[u8]) -> Result<(), GuardError> {
    let target = backing_root.join(rel_path);
    let parent = match target.parent() {
        Some(p) => p,
        None => {
            return Err(GuardError::State(format!(
                "no parent for {}",
                target.display()
            )))
        }
    };
    fs::create_dir_all(parent)?;

    let unique = sha256_hex(format!("{}-{}", rel_path.to_string_lossy(), now_nanos()).as_bytes());
    let file_stem = match target.file_name() {
        Some(n) => n.to_string_lossy().into_owned(),
        None => "candidate".to_string(),
    };
    let tmp = parent.join(format!(".{file_stem}.jankurai-tmp.{}", &unique[..16]));

    write_and_fsync(&tmp, bytes)?;
    fs::rename(&tmp, &target).map_err(|e| {
        // Best-effort cleanup of the staging file on a failed rename.
        let _ = fs::remove_file(&tmp);
        GuardError::Io(e)
    })?;
    fsync_dir(parent)?;
    Ok(())
}

/// Atomically restores the snapshot at `snapshot_path` over `target_path`.
pub fn revert_to_last_good(target_path: &Path, snapshot_path: &Path) -> Result<(), GuardError> {
    let bytes = fs::read(snapshot_path).map_err(|e| {
        GuardError::State(format!(
            "cannot read snapshot {}: {e}",
            snapshot_path.display()
        ))
    })?;
    let parent = match target_path.parent() {
        Some(p) => p,
        None => {
            return Err(GuardError::State(format!(
                "no parent for {}",
                target_path.display()
            )))
        }
    };
    fs::create_dir_all(parent)?;
    let file_stem = match target_path.file_name() {
        Some(n) => n.to_string_lossy().into_owned(),
        None => "file".to_string(),
    };
    let tmp = parent.join(format!(
        ".{file_stem}.jankurai-revert.{}",
        &sha256_hex(now_nanos().to_string().as_bytes())[..12]
    ));
    write_and_fsync(&tmp, &bytes)?;
    fs::rename(&tmp, target_path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        GuardError::Io(e)
    })?;
    fsync_dir(parent)?;
    Ok(())
}

/// Writes a rejected candidate's bytes into the repo's quarantine tree at
/// `.jankurai/guard/rejected/<rfc3339-ts>/<rel_path>` and returns the path.
/// `state_root` here is the repository root that owns the `.jankurai/` tree.
pub fn quarantine_candidate(
    state_root: &Path,
    rel_path: &Path,
    bytes: &[u8],
) -> Result<PathBuf, GuardError> {
    let stamp = sanitize_component(&crate::feedback::now_rfc3339());
    let mut dest = state_root
        .join(".jankurai")
        .join("guard")
        .join("rejected")
        .join(stamp);
    for component in rel_path.components() {
        if let std::path::Component::Normal(part) = component {
            dest = dest.join(sanitize_component(&part.to_string_lossy()));
        }
    }
    let parent = match dest.parent() {
        Some(p) => p,
        None => {
            return Err(GuardError::State(format!(
                "no parent for {}",
                dest.display()
            )))
        }
    };
    fs::create_dir_all(parent)?;
    fs::write(&dest, bytes)?;
    Ok(dest)
}

/// Saves `bytes` as a content-addressed snapshot under
/// `<state_root>/snapshots/<sha>` and returns the sha. Re-saving identical bytes
/// is a no-op.
pub fn snapshot_save(state_root: &Path, bytes: &[u8]) -> Result<String, GuardError> {
    let sha = sha256_hex(bytes);
    let path = snapshot_path(state_root, &sha);
    if !path.exists() {
        let parent = match path.parent() {
            Some(p) => p,
            None => return Err(GuardError::State("snapshot dir has no parent".to_string())),
        };
        fs::create_dir_all(parent)?;
        write_and_fsync(&path, bytes)?;
    }
    Ok(sha)
}

/// Returns the on-disk path of the snapshot blob with hash `sha`.
pub fn snapshot_path(state_root: &Path, sha: &str) -> PathBuf {
    state_root.join("snapshots").join(sha)
}

/// Reads back the snapshot blob with hash `sha`.
pub fn snapshot_restore(state_root: &Path, sha: &str) -> Result<Vec<u8>, GuardError> {
    let path = snapshot_path(state_root, sha);
    fs::read(&path)
        .map_err(|e| GuardError::State(format!("cannot read snapshot {}: {e}", path.display())))
}

/// Writes `bytes` to `path` and fsyncs the file before returning.
fn write_and_fsync(path: &Path, bytes: &[u8]) -> Result<(), GuardError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

/// Fsyncs a directory so a rename into it is durable. A directory that cannot be
/// opened for sync (some filesystems) is tolerated as a best-effort step.
fn fsync_dir(dir: &Path) -> Result<(), GuardError> {
    match File::open(dir) {
        Ok(handle) => {
            // A directory fsync is unsupported on some platforms; that is not a
            // hard failure for the commit, only a weaker durability guarantee.
            if let Err(e) = handle.sync_all() {
                if e.raw_os_error() == Some(libc::EINVAL) {
                    return Ok(());
                }
                return Err(GuardError::Io(e));
            }
            Ok(())
        }
        Err(e) => Err(GuardError::Io(e)),
    }
}

/// Replaces every character that is not alphanumeric, dash, dot or underscore so
/// timestamp and path components are safe directory names.
fn sanitize_component(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Nanoseconds since the Unix epoch, used only as a uniqueness source for staging
/// file names.
fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn atomic_commit_lands_bytes() {
        let dir = tempdir().unwrap();
        atomic_commit(dir.path(), Path::new("a/b.txt"), b"hello").unwrap();
        let got = fs::read(dir.path().join("a/b.txt")).unwrap();
        assert_eq!(got, b"hello");
    }

    #[test]
    fn snapshot_roundtrip() {
        let dir = tempdir().unwrap();
        let sha = snapshot_save(dir.path(), b"snap").unwrap();
        assert_eq!(snapshot_restore(dir.path(), &sha).unwrap(), b"snap");
    }

    #[test]
    fn quarantine_writes_under_jankurai_guard() {
        let dir = tempdir().unwrap();
        let path = quarantine_candidate(dir.path(), Path::new("src/x.rs"), b"bad").unwrap();
        assert!(path.starts_with(dir.path().join(".jankurai/guard/rejected")));
        assert_eq!(fs::read(&path).unwrap(), b"bad");
    }
}
