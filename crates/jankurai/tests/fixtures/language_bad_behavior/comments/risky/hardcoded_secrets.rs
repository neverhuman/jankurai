// Tier 1: Hardcoded secret confessions
// These should ALL produce hard findings.

fn connect_db() {
    // hardcoded password for the database
    let _conn = "postgres://admin:secret@localhost/db";
}

fn api_call() {
    // hardcoded api key — replace with env var
    let _key = "sk-abc123";
}

fn auth_setup() {
    // default password is "admin123"
    let _pwd = "admin123";
}
