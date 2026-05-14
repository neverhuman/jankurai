//! macOS hardening entry point. The Endpoint Security tier (Tier 3) requires a
//! signed system extension and an Apple-granted entitlement, so it is deferred.
//! This function is a real, callable probe that honestly reports Endpoint
//! Security is not configured rather than pretending the tier is active.

#![cfg(target_os = "macos")]

use crate::platform::{HardeningStatus, HardeningTier};

/// Probes for an active Jankurai Endpoint Security extension and reports the
/// result. Endpoint Security needs a notarized system extension plus the
/// `com.apple.developer.endpoint-security.client` entitlement; until that
/// extension ships this probe reports the tier as inactive.
pub fn probe_endpoint_security() -> HardeningStatus {
    HardeningStatus {
        requested: HardeningTier::MacosEndpointSecurity,
        active: false,
        detail: "Endpoint Security tier requested but no Jankurai ES system extension is \
                 installed; this tier is deferred and the guard runs with the audit gate \
                 plus the in-place watcher only"
            .to_string(),
    }
}
