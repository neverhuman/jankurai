use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::json;

use crate::trace::TraceWriter;

use super::SharedState;

pub(super) fn reader_loop(
    mut reader: Box<dyn Read + Send>,
    state: Arc<Mutex<SharedState>>,
    trace: Option<TraceWriter>,
) {
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let chunk = &buf[..n];
                let mut state = state.lock().expect("state mutex poisoned");
                state.parser.process(chunk);
                state.last_output_at = Instant::now();

                if let Some(ref tw) = trace {
                    tw.event("output", json!({"bytes": n})).ok();
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                break;
            }
        }
    }
}
