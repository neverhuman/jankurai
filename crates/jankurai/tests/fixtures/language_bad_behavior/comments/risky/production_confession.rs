// Tier 1: Production confession comments
// These should ALL produce hard findings.

fn main() {
    // remove before production
    setup_debug_mode();

    // not production ready — needs more testing
    initialize();

    // will fix later when we have time
    temporary_init();

    // do not ship this code
    experimental_feature();

    // testing only
    debug_handler();
}

fn setup_debug_mode() {}
fn initialize() {}
fn temporary_init() {}
fn experimental_feature() {}
fn debug_handler() {}
