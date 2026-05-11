// Tier 2: Security-sensitive TODO/FIXME comments
// These should produce ADVISORY findings (not hard blocks).

fn auth_handler() {
    // TODO: fix authentication bypass for edge case
    validate_user();
}

fn encryption_setup() {
    // FIXME: encryption key rotation is broken
    setup_keys();
}

fn session_mgmt() {
    // HACK: session validation is incomplete
    check_session();
}

fn input_handler() {
    // XXX: sanitize this input properly
    accept_input();
}

fn validate_user() {}
fn setup_keys() {}
fn check_session() {}
fn accept_input() {}
