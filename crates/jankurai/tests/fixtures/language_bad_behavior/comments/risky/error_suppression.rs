// Tier 1: Error suppression confessions
// These should ALL produce hard findings.

fn handle_result() {
    // swallow error because we don't care
    let _ = might_fail();
}

fn process_batch() {
    // ignore error and continue
    for item in items() {
        let _ = process(item);
    }
}

fn catch_handler() {
    // empty catch block — suppress exception
    match try_something() {
        Ok(_) => {},
        Err(_) => {}, // intentionally swallowed
    }
}

fn might_fail() -> Result<(), String> { Ok(()) }
fn items() -> Vec<i32> { vec![] }
fn process(_: i32) -> Result<(), String> { Ok(()) }
fn try_something() -> Result<(), String> { Ok(()) }
