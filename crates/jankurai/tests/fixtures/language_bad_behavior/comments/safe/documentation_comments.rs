// Documentation comments that discuss auth/security concepts but are NOT confessions.
// These should produce ZERO findings.

/// This module handles authentication bypass detection.
/// It scans for common patterns where authentication is
/// improperly skipped or disabled in production systems.
pub mod auth_scanner {
    /// Detects when code attempts to circumvent auth validation.
    /// Returns a list of findings with severity ratings.
    pub fn scan_for_bypass_patterns(code: &str) -> Vec<String> {
        let patterns = vec![
            "skip_auth",         // underscore variant — not our phrase
            "bypass_auth",       // underscore variant — not our phrase
            "disable_ssl",       // underscore variant — not our phrase
        ];
        patterns
            .iter()
            .filter(|p| code.contains(*p))
            .map(|p| p.to_string())
            .collect()
    }

    /// Validates that a security configuration does not
    /// contain any embedded credentials or preset credential values.
    pub fn validate_config(config: &str) -> bool {
        !config.is_empty()
    }

    /// This function adds a prefix later to the string. (Testing word boundaries: 'prefix-later' should not match the target phrase)
    /// It also checks the author. (Testing word boundaries: 'author' should not match the a-word)
    /// And it may invalidate the cache. (Testing word boundaries: 'invalidate' should not match the target)
    /// Finally, it runs flawlessly. (Testing word boundaries: 'flawlessly' should not match the transport layer target)
    ///
    /// WARNING: never skip auth here! (Testing negations: 'never' prefix should block detection)
    /// Please do not ignore error codes. (Testing negations: 'do not' prefix should block detection)
    pub fn complex_logic() {}
}
