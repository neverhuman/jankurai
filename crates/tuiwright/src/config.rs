use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::session::Page;

/// Terminal dimensions for the PTY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl TerminalSize {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            pixel_width: cols.saturating_mul(10),
            pixel_height: rows.saturating_mul(20),
        }
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

/// Builder for configuring a PTY-backed TUI session.
#[derive(Debug, Clone)]
pub struct SpawnConfig {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub size: TerminalSize,
    pub default_timeout: Duration,
    pub idle_timeout: Duration,
    pub record: bool,
    pub trace_path: Option<PathBuf>,
    pub scrollback: usize,
}

impl SpawnConfig {
    /// Create a new spawn configuration for the given program.
    pub fn new(program: impl Into<String>) -> Self {
        let mut env = BTreeMap::new();
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());
        env.insert("TUIWRIGHT".to_string(), "1".to_string());
        env.insert("RUST_BACKTRACE".to_string(), "1".to_string());

        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env,
            size: TerminalSize::default(),
            default_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_millis(80),
            record: false,
            trace_path: None,
            scrollback: 10_000,
        }
    }

    /// Add a single argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set the working directory for the child process.
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set an environment variable for the child process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set the terminal size in columns and rows.
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.size = TerminalSize::new(cols, rows);
        self
    }

    /// Set the default timeout for assertions and waits.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Set the idle timeout for screen stability detection.
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Enable or disable GIF recording.
    pub fn record(mut self, enabled: bool) -> Self {
        self.record = enabled;
        self
    }

    /// Set the path for JSONL trace output.
    pub fn trace_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.trace_path = Some(path.into());
        self
    }

    /// Set the scrollback buffer size.
    pub fn scrollback(mut self, lines: usize) -> Self {
        self.scrollback = lines;
        self
    }

    /// Spawn the TUI session. Equivalent to `Page::spawn(self)`.
    pub fn start(self) -> anyhow::Result<Page> {
        Page::spawn(self)
    }
}
