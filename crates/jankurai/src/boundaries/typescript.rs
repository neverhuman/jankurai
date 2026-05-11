pub const WEB_FORBIDDEN_IMPORTS: &[&str] = &[
    " from \"pg\"",
    " from 'pg'",
    " from \"postgres\"",
    " from 'postgres'",
    "better-sqlite3",
    "mysql2",
    "@aws-sdk/client-s3",
];

pub fn web_storage_marker(text: &str) -> Option<&'static str> {
    WEB_FORBIDDEN_IMPORTS
        .iter()
        .copied()
        .find(|marker| text.contains(marker))
}
