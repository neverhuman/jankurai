use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single trace event recorded during a TUI session.
#[derive(Debug, Serialize, Deserialize)]
pub struct TraceEvent {
    pub ts_ms: u128,
    pub kind: String,
    pub data: Value,
}

/// Writes JSONL trace events to a file.
#[derive(Clone)]
pub struct TraceWriter {
    file: Arc<Mutex<File>>,
    start: Instant,
}

impl TraceWriter {
    /// Create a new trace writer at the given path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating trace dir {}", parent.display()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path.as_ref())
            .with_context(|| format!("opening trace {}", path.as_ref().display()))?;

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            start: Instant::now(),
        })
    }

    /// Record a trace event.
    pub fn event(&self, kind: impl Into<String>, data: Value) -> Result<()> {
        let event = TraceEvent {
            ts_ms: self.start.elapsed().as_millis(),
            kind: kind.into(),
            data,
        };
        let line = serde_json::to_string(&event)?;
        let mut file = self.file.lock().expect("trace mutex poisoned");
        writeln!(file, "{line}")?;
        file.flush()?;
        Ok(())
    }

    /// Elapsed time since the trace started.
    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }
}
