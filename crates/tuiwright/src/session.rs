use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde_json::json;

use crate::config::SpawnConfig;
use crate::record::GifRecorder;
use crate::render::{RenderOptions, TerminalRenderer, Theme};
use crate::screen::ScreenSnapshot;
use crate::trace::TraceWriter;

mod actions;
mod runtime;

struct SharedState {
    parser: vt100::Parser,
    last_output_at: Instant,
}

/// A live PTY-backed terminal session for driving one terminal app.
///
/// Provides methods for sending input, querying screen state, creating locators,
/// making assertions, capturing screenshots, recording GIFs, and writing traces.
pub struct Page {
    config: SpawnConfig,
    started_at: Instant,
    state: Arc<Mutex<SharedState>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    #[allow(dead_code)]
    master: Box<dyn MasterPty + Send>,
    child: Mutex<Box<dyn Child + Send + Sync>>,
    theme: Theme,
    renderer: TerminalRenderer,
    recorder: Mutex<Option<GifRecorder>>,
    trace: Option<TraceWriter>,
}

impl Page {
    /// Spawn a new TUI session with the given configuration.
    pub fn spawn(config: SpawnConfig) -> Result<Self> {
        if config.program.trim().is_empty() {
            bail!("SpawnConfig.program cannot be empty");
        }

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: config.size.rows,
                cols: config.size.cols,
                pixel_width: config.size.pixel_width,
                pixel_height: config.size.pixel_height,
            })
            .context("opening PTY")?;

        let mut cmd = CommandBuilder::new(&config.program);
        for arg in &config.args {
            cmd.arg(arg);
        }
        for (key, value) in &config.env {
            cmd.env(key, value);
        }
        if let Some(cwd) = &config.cwd {
            cmd.cwd(cwd);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("spawning child process")?;
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .context("cloning PTY reader")?;
        let writer = pair.master.take_writer().context("taking PTY writer")?;

        let theme = Theme::xterm_dark();
        let renderer = TerminalRenderer::new(RenderOptions::ci(), theme.clone());

        let parser = vt100::Parser::new(config.size.rows, config.size.cols, config.scrollback);

        let state = Arc::new(Mutex::new(SharedState {
            parser,
            last_output_at: Instant::now(),
        }));

        // Set up trace writer if requested
        let trace = if let Some(ref trace_path) = config.trace_path {
            Some(TraceWriter::new(trace_path)?)
        } else {
            None
        };

        if let Some(ref tw) = trace {
            tw.event(
                "spawn",
                json!({
                    "program": config.program,
                    "args": config.args,
                    "cols": config.size.cols,
                    "rows": config.size.rows,
                }),
            )?;
        }

        // Start background reader thread
        let state_clone = Arc::clone(&state);
        let trace_clone = trace.clone();
        thread::spawn(move || {
            runtime::reader_loop(reader, state_clone, trace_clone);
        });

        // Start recording if requested
        let recorder = if config.record {
            Mutex::new(Some(GifRecorder::new()))
        } else {
            Mutex::new(None)
        };

        Ok(Self {
            config,
            started_at: Instant::now(),
            state,
            writer: Arc::new(Mutex::new(writer)),
            master: pair.master,
            child: Mutex::new(child),
            theme,
            renderer,
            recorder,
            trace,
        })
    }

    /// Get the current screen snapshot.
    pub fn screen(&self) -> ScreenSnapshot {
        let state = self.state.lock().expect("state mutex poisoned");
        ScreenSnapshot::from_vt(state.parser.screen(), &self.theme)
    }

    /// Get the elapsed time since session start.
    pub fn elapsed_ms(&self) -> u128 {
        self.started_at.elapsed().as_millis()
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            child.kill().ok();
        }
    }
}
