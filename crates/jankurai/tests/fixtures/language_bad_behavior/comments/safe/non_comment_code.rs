// Code that contains dangerous phrases in STRING LITERALS, not comments.
// These should produce ZERO findings because the detector only scans comments.

fn check_patterns() {
    let dangerous_phrases = vec![
        "skip auth check",
        "bypass authentication",
        "hardcoded password",
        "disable ssl",
        "fake implementation",
        "remove before production",
    ];

    for phrase in &dangerous_phrases {
        if source_code.contains(phrase) {
            report_finding(phrase);
        }
    }
}

fn error_messages() {
    let msg = "Error: skip validation failed";
    let warn = "Warning: disable security mode detected";
    println!("{} {}", msg, warn);
}

fn check_patterns_inline() {
    let pattern = "skip auth"; // This is a string pattern, not a confession
    assert!(pattern.len() > 0);
}

fn report_finding(_: &str) {}
static source_code: &str = "";
