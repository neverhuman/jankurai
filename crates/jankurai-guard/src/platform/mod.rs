//! Platform hardening tiers. Beyond the audit gate, the guard can layer an OS
//! kernel restriction so a misbehaving agent cannot bypass the guarded view.
//! Those restrictions are platform-specific and feature-gated; this module
//! routes to the right backend and reports which tier is actually active.
//!
//! For this release every tier above [`HardeningTier::None`] is feature-gated
//! and ships as a documented, honest no-op: the functions exist and report
//! truthfully that the tier is not active rather than pretending to harden.

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;

use crate::policy::HardeningPolicy;

/// The hardening tier the guard is operating at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardeningTier {
    /// No kernel-level restriction; only the audit gate is in effect.
    None,
    /// Linux Landlock path restrictions (Tier 1).
    LinuxLandlock,
    /// Linux fanotify monitoring (Tier 2).
    LinuxFanotify,
    /// macOS Endpoint Security (Tier 3).
    MacosEndpointSecurity,
}

impl HardeningTier {
    /// A short human label for the tier.
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none (audit gate only)",
            Self::LinuxLandlock => "linux-landlock",
            Self::LinuxFanotify => "linux-fanotify",
            Self::MacosEndpointSecurity => "macos-endpoint-security",
        }
    }
}

/// The result of attempting to apply hardening: which tier the operator asked
/// for, whether it is active, and an explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardeningStatus {
    /// The tier the policy requested.
    pub requested: HardeningTier,
    /// Whether that tier is actually active.
    pub active: bool,
    /// A human-readable explanation of the status.
    pub detail: String,
}

/// Applies platform hardening as configured by `policy`, returning an honest
/// status. When no hardening is requested this is [`HardeningTier::None`] and
/// `active = true`. When hardening is requested but the corresponding feature
/// is not built, the status reports `active = false` with a clear reason.
pub fn apply_hardening(policy: &HardeningPolicy) -> HardeningStatus {
    let requested = requested_tier(policy);
    match requested {
        HardeningTier::None => HardeningStatus {
            requested,
            active: true,
            detail: "no kernel hardening requested; audit gate is the only control".to_string(),
        },
        HardeningTier::LinuxLandlock => apply_linux_landlock(),
        HardeningTier::LinuxFanotify => apply_linux_fanotify(),
        HardeningTier::MacosEndpointSecurity => apply_macos_endpoint_security(),
    }
}

/// Resolves which tier the policy asks for. The highest-numbered enabled toggle
/// wins; macOS ES and Linux tiers are mutually exclusive by platform anyway.
fn requested_tier(policy: &HardeningPolicy) -> HardeningTier {
    if policy.macos_endpoint_security {
        HardeningTier::MacosEndpointSecurity
    } else if policy.linux_fanotify {
        HardeningTier::LinuxFanotify
    } else if policy.linux_landlock {
        HardeningTier::LinuxLandlock
    } else {
        HardeningTier::None
    }
}

/// Routes a Landlock request to the Linux backend, or reports it is
/// unavailable off-Linux.
fn apply_linux_landlock() -> HardeningStatus {
    #[cfg(target_os = "linux")]
    {
        linux::apply_landlock()
    }
    #[cfg(not(target_os = "linux"))]
    {
        HardeningStatus {
            requested: HardeningTier::LinuxLandlock,
            active: false,
            detail: "Landlock is a Linux-only facility; not available on this platform".to_string(),
        }
    }
}

/// Routes a fanotify request to the Linux backend, or reports it is
/// unavailable off-Linux.
fn apply_linux_fanotify() -> HardeningStatus {
    #[cfg(target_os = "linux")]
    {
        linux::apply_fanotify()
    }
    #[cfg(not(target_os = "linux"))]
    {
        HardeningStatus {
            requested: HardeningTier::LinuxFanotify,
            active: false,
            detail: "fanotify is a Linux-only facility; not available on this platform".to_string(),
        }
    }
}

/// Routes an Endpoint Security request to the macOS backend, or reports it is
/// unavailable off-macOS.
fn apply_macos_endpoint_security() -> HardeningStatus {
    #[cfg(target_os = "macos")]
    {
        macos::probe_endpoint_security()
    }
    #[cfg(not(target_os = "macos"))]
    {
        HardeningStatus {
            requested: HardeningTier::MacosEndpointSecurity,
            active: false,
            detail: "Endpoint Security is a macOS-only facility; not available on this platform"
                .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_hardening_is_active_none() {
        let status = apply_hardening(&HardeningPolicy::default());
        assert_eq!(status.requested, HardeningTier::None);
        assert!(status.active);
    }
}
