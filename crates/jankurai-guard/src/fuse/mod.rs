//! The FUSE backend. On Linux with the `fuse` feature this module mounts a
//! guarded view of the backing store: passthrough reads, buffered mutations,
//! and audit-gated commits. On every other platform/build the module still
//! compiles — it exposes [`fuse_available`] (returning `false`) and a [`mount`]
//! that returns an informative [`GuardError::FuseUnavailable`] — so `cli.rs` and
//! `doctor.rs` can reference it unconditionally.

#[cfg(all(feature = "fuse", target_os = "linux"))]
mod filesystem;
#[cfg(all(feature = "fuse", target_os = "linux"))]
mod fs_ops;
#[cfg(all(feature = "fuse", target_os = "linux"))]
mod handles;
#[cfg(all(feature = "fuse", target_os = "linux"))]
mod inode;
#[cfg(all(feature = "fuse", target_os = "linux"))]
mod ops_read;

use crate::GuardError;

#[cfg(all(feature = "fuse", target_os = "linux"))]
mod linux_impl {
    use super::*;
    use crate::audit_client::AuditClient;
    use crate::feedback::DenialBus;
    use crate::layout::GuardLayout;
    use crate::policy::GuardPolicy;
    use std::sync::Arc;

    pub use super::filesystem::GuardFs;

    /// A live FUSE mount session. Dropping the session unmounts.
    pub struct FuseSession {
        session: fuser::BackgroundSession,
    }

    impl FuseSession {
        /// Unmounts the guarded filesystem.
        pub fn unmount(self) {
            drop(self.session);
        }
    }

    /// Returns `true` because this build links a working FUSE backend.
    pub fn fuse_available() -> bool {
        true
    }

    /// Mounts the guarded filesystem at `layout.mount`, serving content from
    /// `layout.backing` and gating mutations through `audit`.
    pub fn mount(
        layout: GuardLayout,
        policy: GuardPolicy,
        audit: Arc<dyn AuditClient>,
        bus: Arc<DenialBus>,
    ) -> Result<FuseSession, GuardError> {
        layout.ensure_dirs()?;
        let fs = GuardFs::new(layout.clone(), policy, audit, bus)?;
        let options = vec![
            fuser::MountOption::FSName("jankurai-guard".to_string()),
            fuser::MountOption::DefaultPermissions,
        ];
        let session = fuser::spawn_mount2(fs, &layout.mount, &options)
            .map_err(|e| GuardError::FuseUnavailable(format!("mount failed: {e}")))?;
        Ok(FuseSession { session })
    }
}

#[cfg(all(feature = "fuse", target_os = "linux"))]
pub use linux_impl::{fuse_available, mount, FuseSession};

#[cfg(not(all(feature = "fuse", target_os = "linux")))]
mod portable_impl {
    use super::*;
    use crate::audit_client::AuditClient;
    use crate::feedback::DenialBus;
    use crate::layout::GuardLayout;
    use crate::policy::GuardPolicy;
    use std::sync::Arc;

    /// An uninhabited session type so callers can name the return type of
    /// [`mount`] uniformly across platforms. It cannot be constructed on a
    /// non-FUSE build because [`mount`] always returns an error here.
    pub enum FuseSession {}

    impl FuseSession {
        /// Unmounts the guarded filesystem. Unreachable on a non-FUSE build.
        pub fn unmount(self) {
            match self {}
        }
    }

    /// Returns `false`: this build does not link a FUSE backend.
    pub fn fuse_available() -> bool {
        false
    }

    /// Always returns [`GuardError::FuseUnavailable`]: the FUSE backend requires
    /// Linux and the `fuse` Cargo feature. Callers should fall back to the
    /// cross-platform watcher backend.
    pub fn mount(
        _layout: GuardLayout,
        _policy: GuardPolicy,
        _audit: Arc<dyn AuditClient>,
        _bus: Arc<DenialBus>,
    ) -> Result<FuseSession, GuardError> {
        Err(GuardError::FuseUnavailable(
            "the FUSE backend is built only on Linux with the `fuse` feature; \
             use `guard watch` for in-place guarding on this platform"
                .to_string(),
        ))
    }
}

#[cfg(not(all(feature = "fuse", target_os = "linux")))]
pub use portable_impl::{fuse_available, mount, FuseSession};
