use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::io::{self, IsTerminal};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub enum Style {
    Heading,
    Accent,
    Good,
    Warn,
    Error,
    Muted,
    Create,
    Merge,
    Keep,
}

pub fn stdout_color_enabled() -> bool {
    color_enabled(io::stdout().is_terminal())
}

pub fn stderr_color_enabled() -> bool {
    color_enabled(io::stderr().is_terminal())
}

fn color_enabled(is_terminal: bool) -> bool {
    if std::env::var("JANKURAI_COLOR").as_deref() == Ok("always") {
        return true;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    is_terminal && std::env::var("TERM").as_deref() != Ok("dumb")
}

pub fn paint(style: Style, text: impl AsRef<str>, enabled: bool) -> String {
    let text = text.as_ref();
    if !enabled {
        return text.to_string();
    }
    let code = match style {
        Style::Heading => "1;38;5;45",
        Style::Accent => "1;38;5;141",
        Style::Good => "1;38;5;82",
        Style::Warn => "1;38;5;214",
        Style::Error => "1;38;5;196",
        Style::Muted => "38;5;245",
        Style::Create => "1;38;5;82",
        Style::Merge => "1;38;5;45",
        Style::Keep => "38;5;245",
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}

pub fn epaint(style: Style, text: impl AsRef<str>) -> String {
    paint(style, text, stderr_color_enabled())
}

pub fn status(style: Style, label: &str, message: impl AsRef<str>) {
    eprintln!(
        "{} {}",
        epaint(style, format!("[{label}]")),
        message.as_ref()
    );
}

pub struct CliProgress {
    bar: Option<ProgressBar>,
    forced_lines: bool,
    len: u64,
    pos: AtomicU64,
}

impl CliProgress {
    pub fn new(label: &str, len: u64) -> Self {
        let force = std::env::var("JANKURAI_PROGRESS").as_deref() == Ok("always");
        let len = len.max(1);
        if force && !io::stderr().is_terminal() {
            status(Style::Accent, "progress", label);
            return Self {
                bar: None,
                forced_lines: true,
                len,
                pos: AtomicU64::new(0),
            };
        }
        if !force && !progress_enabled() {
            return Self {
                bar: None,
                forced_lines: false,
                len,
                pos: AtomicU64::new(0),
            };
        }
        let bar = if force {
            ProgressBar::with_draw_target(Some(len), ProgressDrawTarget::stderr_with_hz(20))
        } else {
            ProgressBar::new(len)
        };
        let style = ProgressStyle::with_template(
            "{spinner:.magenta} [{elapsed_precise}] {bar:36.cyan/blue} {pos:>2}/{len:2} {msg}",
        )
        .unwrap()
        .progress_chars("=>-")
        .tick_strings(&["-", "\\", "|", "/"]);
        bar.set_style(style);
        bar.set_message(label.to_string());
        bar.enable_steady_tick(Duration::from_millis(60));
        Self {
            bar: Some(bar),
            forced_lines: false,
            len,
            pos: AtomicU64::new(0),
        }
    }

    pub fn tick(&self, message: impl Into<String>) {
        let message = message.into();
        if self.forced_lines {
            let pos = self.pos.fetch_add(1, Ordering::Relaxed) + 1;
            eprintln!("{}", forced_progress_line(pos, self.len, &message));
            return;
        }
        if let Some(bar) = &self.bar {
            bar.set_message(message);
            bar.inc(1);
        }
    }

    pub fn finish(&self, message: impl Into<String>) {
        let message = message.into();
        if self.forced_lines {
            eprintln!("{}", forced_progress_line(self.len, self.len, &message));
            return;
        }
        if let Some(bar) = &self.bar {
            bar.finish_with_message(message);
        }
    }
}

fn forced_progress_line(pos: u64, len: u64, message: &str) -> String {
    let width = 28usize;
    let ratio = (pos.min(len) as f64 / len.max(1) as f64).clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    let bar = format!(
        "{}{}",
        "=".repeat(filled),
        "-".repeat(width.saturating_sub(filled))
    );
    let spinner = ["-", "\\", "|", "/"][(pos as usize) % 4];
    epaint(
        Style::Accent,
        format!("{spinner} [{bar}] {}/{} {message}", pos.min(len), len),
    )
}

fn progress_enabled() -> bool {
    if std::env::var("JANKURAI_PROGRESS").as_deref() == Ok("always") {
        return true;
    }
    if std::env::var("JANKURAI_PROGRESS").as_deref() == Ok("never") {
        return false;
    }
    io::stderr().is_terminal() && std::env::var("TERM").as_deref() != Ok("dumb")
}
