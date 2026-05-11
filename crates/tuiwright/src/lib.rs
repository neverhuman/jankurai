//! Tuiwright: black-box testing for terminal user interfaces.
//!
//! Tuiwright spawns real applications in a real pseudo-terminal, drives
//! keyboard/mouse/paste/resize input, parses terminal output into a
//! deterministic screen model, and provides locator-driven ergonomics:
//! locators, auto-waiting, polling assertions, screenshots, GIF recordings,
//! and JSONL traces.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use std::time::Duration;
//! use tuiwright::{Key, Page, SpawnConfig};
//!
//! fn main() -> anyhow::Result<()> {
//!     let page = Page::spawn(
//!         SpawnConfig::new("my-tui")
//!             .size(80, 24)
//!     )?;
//!
//!     page.wait_for_text("Ready", Duration::from_secs(5))?;
//!     page.press(Key::Enter)?;
//!     page.screenshot("target/tuiwright/ready.png")?;
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod expect;
pub mod input;
pub mod locator;
pub mod record;
pub mod render;
pub mod screen;
pub mod session;
pub mod trace;

pub use config::{SpawnConfig, TerminalSize};
pub use expect::{ExpectLocator, ExpectScreen};
pub use input::{Key, MouseButton};
pub use locator::{Locator, Selector, TextMatch};
pub use record::GifOptions;
pub use render::{RenderOptions, TerminalRenderer, Theme};
pub use screen::{CellSnapshot, Region, ScreenSnapshot};
pub use session::Page;
pub use trace::TraceWriter;
