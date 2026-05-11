# Tuiwright: Playwright-Style TUI Testing

Tuiwright is a Rust-native, black-box testing framework for terminal user
interfaces. It spawns real applications in a real pseudo-terminal, drives
keyboard/mouse/paste/resize input, maintains an accurate virtual terminal
model, and provides Playwright-grade ergonomics.

## Quick Start

Add to your project's dev dependencies:

```toml
[dev-dependencies]
tuiwright = { path = "path/to/jankurai/crates/tuiwright" }
```

Or install the CLI:

```bash
cargo install --path crates/tuiwright-cli --locked
```

## Rust API

```rust
use std::time::Duration;
use tuiwright::{Key, Page, SpawnConfig};

#[test]
fn user_can_navigate() -> anyhow::Result<()> {
    let page = Page::spawn(
        SpawnConfig::new(env!("CARGO_BIN_EXE_my-tui"))
            .size(100, 30)
    )?;

    page.wait_for_text("Welcome", Duration::from_secs(5))?;
    page.press(Key::Enter)?;
    page.wait_for_text("Main Menu", Duration::from_secs(3))?;

    page.screenshot("target/tuiwright/main-menu.png")?;
    Ok(())
}
```

## Core Concepts

### Page

A `Page` is a live PTY-backed terminal session — the Tuiwright equivalent of
Playwright's `Page`. It owns the child process lifecycle and provides all
interaction methods.

```rust
let page = Page::spawn(
    SpawnConfig::new("my-tui")
        .arg("--demo")
        .size(80, 24)
        .env("TERM", "xterm-256color")
        .timeout(Duration::from_secs(10))
        .record(true)
)?;
```

### Input Actions

```rust
page.press(Key::Enter)?;
page.press(Key::Up)?;
page.press(Key::Ctrl('c'))?;
page.type_text("hello")?;
page.paste("multi\nline\ninput")?;
page.click_cell(10, 5)?;
page.resize(120, 40)?;
```

### Locators

```rust
let locator = page.get_by_text("Submit");
let locator = page.get_by_regex(r"Count: \d+");
let locator = page.get_by_cell(5, 10);
let locator = page.cursor();
```

### Assertions

All assertions auto-retry until timeout (default 5 seconds):

```rust
page.expect_screen().to_contain_text("Ready")?;
page.expect_screen().not_to_contain_text("Error")?;
page.expect_screen().to_match_regex(r"Saved in \d+ms")?;

let locator = page.get_by_text("Submit");
page.expect_locator(&locator).to_be_visible()?;
page.expect_locator(&locator).to_have_text("Submit")?;
```

### Waits

```rust
page.wait_for_text("Ready", Duration::from_secs(5))?;
page.wait_for_regex(r"Loaded \d+ items", Duration::from_secs(10))?;
page.wait_until_idle(Duration::from_millis(200))?;
```

### Screenshots

Screenshots are deterministic pixel renderings of the terminal cell grid.
They do not require a real terminal window, X server, or browser — they
work headlessly in CI.

```rust
page.screenshot("target/tuiwright/home.png")?;
```

### GIF Recordings

```rust
page.start_recording()?;
// ... drive the flow ...
page.stop_recording_gif("target/tuiwright/flow.gif", Default::default())?;
```

### Traces

Enable JSONL trace output for debugging:

```rust
let page = Page::spawn(
    SpawnConfig::new("my-tui")
        .trace_path("target/tuiwright/flow.trace.jsonl")
)?;
// All actions and screen changes are recorded to the trace file.
```

## CLI

```bash
# Screenshot
tuiwright screenshot --cols 80 --rows 24 --wait-text "Ready" \
  --out target/tuiwright/shot.png -- cargo run -q --bin my-tui

# Record GIF
tuiwright record --cols 80 --rows 24 --seconds 5 \
  --out target/tuiwright/flow.gif -- cargo run -q --bin my-tui
```

## CI Recommendations

Set deterministic environment for reproducible artifacts:

```bash
export TERM=xterm-256color
export COLORTERM=truecolor
export RUST_BACKTRACE=1
cargo test -p my-tui-tests
```

Upload `target/tuiwright/**` as CI artifacts on failure.

## Audit Evidence

Jankurai's audit can recognize Tuiwright-covered Rust test flows as positive rendered UX evidence. A flow counts when a Rust test uses `Page::spawn` or `SpawnConfig` together with a wait or assertion such as `wait_for_text`, `wait_for_regex`, `expect_screen`, or `expect_locator`.

Actions such as `press`, `type_text`, `paste`, `click_cell`, and `resize` strengthen the evidence. `screenshot`, `stop_recording_gif`, and `trace_path` are counted as supporting artifacts, but screenshots alone are not proof.

The audit does not run Tuiwright itself. Missing Tuiwright evidence does not create a finding unless a future explicit manifest declares required TUI flows.

## Architecture

```text
Test code / CLI
    |
    v
Page API  <── locators, assertions, timeouts, tracing
    |
    v
PTY controller  <── keyboard/mouse/paste/resize input
    |
    v
Real TUI process (Ratatui, Crossterm, any terminal app)
    |
    v
Terminal byte stream (ANSI/VT sequences)
    |
    v
vt100 emulator model  <── screen cells, cursor, modes
    |
    +── text/regex locators
    +── PNG renderer (font8x8)
    +── GIF recorder
    +── JSONL trace writer
```

## Crate Structure

| Crate | Purpose |
|---|---|
| `tuiwright` | Core library: PTY, screen model, locators, renderer, recorder |
| `tuiwright-cli` | CLI binary for screenshots, recordings, traces |
| `tuiwright-demo` | Example crossterm counter app for testing |
