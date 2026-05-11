use std::time::{Duration, Instant};

use anyhow::{bail, Result};

use crate::locator::Locator;
use crate::screen::ScreenSnapshot;

/// Assertion builder for screen-level expectations.
///
/// All assertions poll until timeout.
pub struct ExpectScreen<'a> {
    screen_fn: Box<dyn Fn() -> ScreenSnapshot + 'a>,
    timeout: Duration,
    poll_interval: Duration,
}

impl<'a> ExpectScreen<'a> {
    /// Create a new screen expectation that polls for the current screen state.
    pub fn new(screen_fn: impl Fn() -> ScreenSnapshot + 'a, timeout: Duration) -> Self {
        Self {
            screen_fn: Box::new(screen_fn),
            timeout,
            poll_interval: Duration::from_millis(50),
        }
    }

    /// Assert that the screen contains the given text, polling until timeout.
    pub fn to_contain_text(&self, text: &str) -> Result<()> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let screen = (self.screen_fn)();
            if screen.contains_text(text) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "Timed out after {:?} waiting for text {:?}\n\nLast screen:\n{}",
                    self.timeout,
                    text,
                    screen.plain_text()
                );
            }
            std::thread::sleep(self.poll_interval);
        }
    }

    /// Assert that the screen does NOT contain the given text.
    pub fn not_to_contain_text(&self, text: &str) -> Result<()> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let screen = (self.screen_fn)();
            if !screen.contains_text(text) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "Timed out after {:?} — text {:?} is still present",
                    self.timeout,
                    text,
                );
            }
            std::thread::sleep(self.poll_interval);
        }
    }

    /// Assert that the screen matches a regex, polling until timeout.
    pub fn to_match_regex(&self, pattern: &str) -> Result<()> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let screen = (self.screen_fn)();
            if screen.matches_regex(pattern) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "Timed out after {:?} waiting for regex {:?}\n\nLast screen:\n{}",
                    self.timeout,
                    pattern,
                    screen.plain_text()
                );
            }
            std::thread::sleep(self.poll_interval);
        }
    }
}

/// Assertion builder for locator-level expectations.
pub struct ExpectLocator<'a> {
    locator: &'a Locator,
    screen_fn: Box<dyn Fn() -> ScreenSnapshot + 'a>,
    timeout: Duration,
    poll_interval: Duration,
}

impl<'a> ExpectLocator<'a> {
    pub fn new(
        locator: &'a Locator,
        screen_fn: impl Fn() -> ScreenSnapshot + 'a,
        timeout: Duration,
    ) -> Self {
        Self {
            locator,
            screen_fn: Box::new(screen_fn),
            timeout,
            poll_interval: Duration::from_millis(50),
        }
    }

    /// Assert that the locator resolves to at least one visible match.
    pub fn to_be_visible(&self) -> Result<()> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let screen = (self.screen_fn)();
            let matches = self.locator.resolve(&screen);
            if !matches.is_empty() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "Timed out after {:?} — locator {:?} not visible\n\nLast screen:\n{}",
                    self.timeout,
                    format!("{:?}", self.locator.selector),
                    screen.plain_text()
                );
            }
            std::thread::sleep(self.poll_interval);
        }
    }

    /// Assert that the locator does NOT resolve to any match.
    pub fn not_to_be_visible(&self) -> Result<()> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let screen = (self.screen_fn)();
            let matches = self.locator.resolve(&screen);
            if matches.is_empty() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "Timed out after {:?} — locator {:?} is still visible",
                    self.timeout,
                    format!("{:?}", self.locator.selector),
                );
            }
            std::thread::sleep(self.poll_interval);
        }
    }

    /// Assert that the matched text contains the given string.
    pub fn to_have_text(&self, expected: &str) -> Result<()> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let screen = (self.screen_fn)();
            let matches = self.locator.resolve(&screen);
            if let Some(m) = matches.first() {
                if m.text.contains(expected) {
                    return Ok(());
                }
            }
            if Instant::now() >= deadline {
                bail!(
                    "Timed out after {:?} — locator text does not contain {:?}",
                    self.timeout,
                    expected,
                );
            }
            std::thread::sleep(self.poll_interval);
        }
    }
}
