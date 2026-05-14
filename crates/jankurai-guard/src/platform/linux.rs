//! Linux hardening entry points. Landlock (Tier 1) and fanotify (Tier 2) are
//! gated behind the `landlock` and `fanotify` Cargo features. Neither feature
//! is built in this release, so these functions honestly report the tier is not
//! active — they are real, callable functions, not stubs that pretend.

#![cfg(target_os = "linux")]

use crate::platform::{HardeningStatus, HardeningTier};

/// Attempts to apply Landlock path restrictions. Without the `landlock` feature
/// this reports the tier as inactive with a clear reason.
pub fn apply_landlock() -> HardeningStatus {
    #[cfg(feature = "landlock")]
    {
        HardeningStatus {
            requested: HardeningTier::LinuxLandlock,
            active: false,
            detail: "landlock feature is built but ruleset installation is deferred to a \
                     later release; the guard runs with the audit gate only"
                .to_string(),
        }
    }
    #[cfg(not(feature = "landlock"))]
    {
        HardeningStatus {
            requested: HardeningTier::LinuxLandlock,
            active: false,
            detail: "Landlock tier requested but the `landlock` feature is not built; \
                     rebuild with `--features landlock` once that tier ships"
                .to_string(),
        }
    }
}

/// Attempts to start fanotify monitoring. Without the `fanotify` feature this
/// reports the tier as inactive with a clear reason.
pub fn apply_fanotify() -> HardeningStatus {
    #[cfg(feature = "fanotify")]
    {
        HardeningStatus {
            requested: HardeningTier::LinuxFanotify,
            active: false,
            detail: "fanotify feature is built but the monitor is deferred to a later \
                     release; the guard runs with the audit gate only"
                .to_string(),
        }
    }
    #[cfg(not(feature = "fanotify"))]
    {
        HardeningStatus {
            requested: HardeningTier::LinuxFanotify,
            active: false,
            detail: "fanotify tier requested but the `fanotify` feature is not built; \
                     rebuild with `--features fanotify` once that tier ships"
                .to_string(),
        }
    }
}
