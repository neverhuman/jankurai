pub const DOMAIN_FORBIDDEN_IMPORTS: &[&str] = &[
    "std::fs",
    "std::env",
    "std::net",
    "std::time::SystemTime",
    "rand::",
    "sqlx::",
    "diesel::",
    "reqwest::",
    "rdkafka::",
    "tracing::",
    "log::",
];

pub fn domain_io_marker(text: &str) -> Option<&'static str> {
    DOMAIN_FORBIDDEN_IMPORTS
        .iter()
        .copied()
        .find(|marker| text.contains(marker))
}
