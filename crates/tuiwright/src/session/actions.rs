use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use portable_pty::PtySize;
use serde_json::json;

use crate::expect::{ExpectLocator, ExpectScreen};
use crate::input::{self, Key, MouseButton};
use crate::locator::{Locator, Selector};
use crate::record::{GifOptions, GifRecorder};
use crate::screen::ScreenSnapshot;

use super::Page;

impl Page {
    /// Send a key press to the application.
    pub fn press(&self, key: Key) -> Result<()> {
        let app_cursor = {
            let state = self.state.lock().expect("state mutex poisoned");
            state.parser.screen().application_cursor()
        };
        let bytes = input::encode_key(key, app_cursor);
        self.write_pty(&bytes)?;

        if let Some(ref tw) = self.trace {
            tw.event("action", json!({"kind": "press", "key": format!("{key}")}))?;
        }

        self.maybe_record_frame();
        thread::sleep(Duration::from_millis(10));
        Ok(())
    }

    /// Type a string of text into the application.
    pub fn type_text(&self, text: &str) -> Result<()> {
        let bytes = input::encode_text(text);
        self.write_pty(&bytes)?;

        if let Some(ref tw) = self.trace {
            tw.event("action", json!({"kind": "type_text", "text": text}))?;
        }

        self.maybe_record_frame();
        thread::sleep(Duration::from_millis(10));
        Ok(())
    }

    /// Paste text using bracketed paste if the app has enabled it.
    pub fn paste(&self, text: &str) -> Result<()> {
        let bracketed = {
            let state = self.state.lock().expect("state mutex poisoned");
            state.parser.screen().bracketed_paste()
        };
        let bytes = input::encode_paste(text, bracketed);
        self.write_pty(&bytes)?;

        if let Some(ref tw) = self.trace {
            tw.event("action", json!({"kind": "paste", "length": text.len()}))?;
        }

        self.maybe_record_frame();
        thread::sleep(Duration::from_millis(10));
        Ok(())
    }

    /// Click a cell at the given coordinates using SGR mouse protocol.
    pub fn click_cell(&self, col: u16, row: u16) -> Result<()> {
        let press = input::encode_sgr_mouse(MouseButton::Left, col, row, false);
        let release = input::encode_sgr_mouse(MouseButton::Left, col, row, true);
        self.write_pty(&press)?;
        thread::sleep(Duration::from_millis(5));
        self.write_pty(&release)?;

        if let Some(ref tw) = self.trace {
            tw.event("action", json!({"kind": "click", "col": col, "row": row}))?;
        }

        self.maybe_record_frame();
        thread::sleep(Duration::from_millis(10));
        Ok(())
    }

    /// Resize the terminal.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: cols.saturating_mul(10),
                pixel_height: rows.saturating_mul(20),
            })
            .context("resizing PTY")?;

        {
            let mut state = self.state.lock().expect("state mutex poisoned");
            state.parser.screen_mut().set_size(rows, cols);
        }

        if let Some(ref tw) = self.trace {
            tw.event(
                "action",
                json!({"kind": "resize", "cols": cols, "rows": rows}),
            )?;
        }

        thread::sleep(Duration::from_millis(50));
        self.maybe_record_frame();
        Ok(())
    }

    /// Create a text locator for the given substring.
    pub fn get_by_text(&self, text: &str) -> Locator {
        Locator::new(Selector::Text(text.to_string()))
    }

    /// Create a regex locator.
    pub fn get_by_regex(&self, pattern: &str) -> Locator {
        Locator::new(Selector::Regex(pattern.to_string()))
    }

    /// Create a locator for a specific cell.
    pub fn get_by_cell(&self, row: u16, col: u16) -> Locator {
        Locator::new(Selector::Cell(row, col))
    }

    /// Create a locator for the cursor position.
    pub fn cursor(&self) -> Locator {
        Locator::new(Selector::CursorPosition)
    }

    /// Wait until the given text appears on screen.
    pub fn wait_for_text(&self, text: &str, timeout: Duration) -> Result<()> {
        self.wait_for_screen_match(timeout, |screen| screen.contains_text(text), "text", text)
    }

    /// Wait until the given regex matches on screen.
    pub fn wait_for_regex(&self, pattern: &str, timeout: Duration) -> Result<()> {
        self.wait_for_screen_match(
            timeout,
            |screen| screen.matches_regex(pattern),
            "regex",
            pattern,
        )
    }

    /// Wait until no terminal output arrives for the given duration.
    pub fn wait_until_idle(&self, quiet_for: Duration) -> Result<()> {
        let timeout = self.config.default_timeout;
        let deadline = Instant::now() + timeout;
        loop {
            let last_output = {
                let state = self.state.lock().expect("state mutex poisoned");
                state.last_output_at
            };
            if last_output.elapsed() >= quiet_for {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!("Timed out after {timeout:?} waiting for idle");
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Create a screen-level expectation builder.
    pub fn expect_screen(&self) -> ExpectScreen<'_> {
        let timeout = self.config.default_timeout;
        ExpectScreen::new(move || self.screen(), timeout)
    }

    /// Create a locator-level expectation builder.
    pub fn expect_locator<'a>(&'a self, locator: &'a Locator) -> ExpectLocator<'a> {
        let timeout = self.config.default_timeout;
        ExpectLocator::new(locator, move || self.screen(), timeout)
    }

    /// Capture a PNG screenshot of the current terminal state.
    pub fn screenshot(&self, path: impl AsRef<Path>) -> Result<()> {
        let screen = self.screen();
        self.renderer.save_screenshot(&screen, path.as_ref())?;

        if let Some(ref tw) = self.trace {
            tw.event(
                "screenshot",
                json!({"path": path.as_ref().display().to_string()}),
            )?;
        }

        Ok(())
    }

    /// Start recording frames for a GIF.
    pub fn start_recording(&self) -> Result<()> {
        let mut recorder = self.recorder.lock().expect("recorder mutex poisoned");
        *recorder = Some(GifRecorder::new());
        let screen = self.screen();
        if let Some(ref mut rec) = *recorder {
            rec.capture_frame(screen, self.elapsed_ms());
        }
        Ok(())
    }

    /// Stop recording and encode the accumulated frames as a GIF.
    pub fn stop_recording_gif(&self, path: impl AsRef<Path>, options: GifOptions) -> Result<()> {
        let mut recorder_guard = self.recorder.lock().expect("recorder mutex poisoned");
        let recorder = recorder_guard.take().context("recording was not started")?;

        recorder.encode_gif(path.as_ref(), &self.renderer, &options)?;

        if let Some(ref tw) = self.trace {
            tw.event(
                "recording",
                json!({
                    "path": path.as_ref().display().to_string(),
                    "frames": recorder.frame_count(),
                }),
            )?;
        }

        Ok(())
    }

    /// Save the accumulated trace to a JSONL file.
    pub fn save_trace(&self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(ref tw) = self.trace {
            tw.event(
                "trace_saved",
                json!({"path": path.as_ref().display().to_string()}),
            )?;
        }
        Ok(())
    }

    /// Kill the child process.
    pub fn kill(&self) -> Result<()> {
        let mut child = self.child.lock().expect("child mutex poisoned");
        child.kill().ok();
        Ok(())
    }

    fn write_pty(&self, bytes: &[u8]) -> Result<()> {
        let mut writer = self.writer.lock().expect("writer mutex poisoned");
        writer.write_all(bytes).context("writing to PTY")?;
        writer.flush().context("flushing PTY")?;
        Ok(())
    }

    fn wait_for_screen_match<F>(
        &self,
        timeout: Duration,
        mut matches: F,
        kind: &str,
        query: &str,
    ) -> Result<()>
    where
        F: FnMut(&ScreenSnapshot) -> bool,
    {
        let deadline = Instant::now() + timeout;
        loop {
            let screen = self.screen();
            if matches(&screen) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                let screen_text = screen.plain_text();
                bail!(
                    "Timed out after {timeout:?} waiting for {kind} {query:?}\n\nLast screen:\n{screen_text}"
                );
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn maybe_record_frame(&self) {
        if let Ok(mut guard) = self.recorder.lock() {
            if let Some(ref mut recorder) = *guard {
                thread::sleep(Duration::from_millis(30));
                let screen = self.screen();
                recorder.capture_frame(screen, self.elapsed_ms());
            }
        }
    }
}
