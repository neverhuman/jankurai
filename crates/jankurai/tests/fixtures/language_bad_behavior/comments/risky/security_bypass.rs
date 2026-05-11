// Tier 1: Security bypass confessions in comments
// These should ALL produce hard findings.

fn handle_request() {
    // skip auth check for internal endpoints
    process_data();
}

fn process_login() {
    // bypass authentication for admin users
    grant_access();
}

fn setup_tls() {
    // disable ssl verification in production
    connect();
}

fn validate_input() {
    // skip validation for trusted sources
    accept_input();
}

fn cors_handler() {
    // allow all origins for convenience
    set_headers();
}

fn rate_limiter() {
    // disable rate limit temporarily
    allow_request();
}

fn process_data() {}
fn grant_access() {}
fn connect() {}
fn accept_input() {}
fn set_headers() {}
fn allow_request() {}
