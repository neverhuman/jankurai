use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use tuiwright::{Key, Page, SpawnConfig};

static DEMO_TEST_LOCK: Mutex<()> = Mutex::new(());

fn demo_test_lock() -> MutexGuard<'static, ()> {
    DEMO_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Helper to get the demo binary path.
/// Builds the demo if needed and extracts the binary path from cargo output.
fn demo_bin() -> String {
    // Try the cargo-provided env var first
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_tuiwright-demo") {
        return path;
    }

    // Build the demo and parse the binary path from cargo's JSON output
    let output = std::process::Command::new("cargo")
        .args(["build", "-p", "tuiwright-demo", "--message-format=json"])
        .output()
        .expect("failed to build demo");
    assert!(
        output.status.success(),
        "demo build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Parse JSON messages to find the executable
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
            if msg.get("reason").and_then(|r| r.as_str()) == Some("compiler-artifact")
                && msg
                    .get("target")
                    .and_then(|t| t.get("name"))
                    .and_then(|n| n.as_str())
                    == Some("tuiwright-demo")
            {
                if let Some(exe) = msg.get("executable").and_then(|e| e.as_str()) {
                    return exe.to_string();
                }
            }
        }
    }

    panic!("could not find tuiwright-demo binary in cargo build output");
}

fn spawn_demo() -> Page {
    Page::spawn(SpawnConfig::new(demo_bin()).size(80, 24)).expect("failed to spawn demo")
}

#[test]
fn can_spawn_and_see_initial_text() {
    let _guard = demo_test_lock();
    let page = spawn_demo();
    page.wait_for_text("Counter", Duration::from_secs(5))
        .expect("should see 'Counter' on screen");
}

#[test]
fn can_see_counter_at_zero() {
    let _guard = demo_test_lock();
    let page = spawn_demo();
    page.wait_for_text("Counter: 0", Duration::from_secs(5))
        .expect("should see 'Counter: 0'");
}

#[test]
fn can_press_up_and_increment() {
    let _guard = demo_test_lock();
    let page = spawn_demo();
    page.wait_for_text("Counter: 0", Duration::from_secs(5))
        .unwrap();

    page.press(Key::Up).unwrap();
    page.wait_for_text("Counter: 1", Duration::from_secs(3))
        .expect("should see 'Counter: 1' after Up");
}

#[test]
fn can_press_multiple_times() {
    let _guard = demo_test_lock();
    let page = spawn_demo();
    page.wait_for_text("Counter: 0", Duration::from_secs(5))
        .unwrap();

    page.press(Key::Up).unwrap();
    page.press(Key::Up).unwrap();
    page.press(Key::Up).unwrap();
    page.wait_for_text("Counter: 3", Duration::from_secs(3))
        .expect("should see 'Counter: 3' after 3x Up");
}

#[test]
fn can_press_down_and_decrement() {
    let _guard = demo_test_lock();
    let page = spawn_demo();
    page.wait_for_text("Counter: 0", Duration::from_secs(5))
        .unwrap();

    page.press(Key::Up).unwrap();
    page.press(Key::Up).unwrap();
    page.wait_for_text("Counter: 2", Duration::from_secs(3))
        .unwrap();

    page.press(Key::Down).unwrap();
    page.wait_for_text("Counter: 1", Duration::from_secs(3))
        .expect("should see 'Counter: 1' after Down");
}

#[test]
fn screenshot_produces_valid_png() {
    let _guard = demo_test_lock();
    let page = spawn_demo();
    page.wait_for_text("Counter", Duration::from_secs(5))
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shot.png");
    page.screenshot(&path).unwrap();

    assert!(path.exists(), "screenshot file should exist");
    let metadata = std::fs::metadata(&path).unwrap();
    assert!(
        metadata.len() > 100,
        "screenshot should not be trivially small"
    );
}

#[test]
fn gif_recording_produces_valid_file() {
    let _guard = demo_test_lock();
    let page = spawn_demo();
    page.wait_for_text("Counter", Duration::from_secs(5))
        .unwrap();

    page.start_recording().unwrap();
    page.press(Key::Up).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    page.press(Key::Up).unwrap();
    std::thread::sleep(Duration::from_millis(200));

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("flow.gif");
    page.stop_recording_gif(&path, Default::default()).unwrap();

    assert!(path.exists(), "GIF file should exist");
    let metadata = std::fs::metadata(&path).unwrap();
    assert!(metadata.len() > 100, "GIF should not be trivially small");
}

#[test]
fn locator_finds_text() {
    let _guard = demo_test_lock();
    let page = spawn_demo();
    page.wait_for_text("Counter", Duration::from_secs(5))
        .unwrap();

    let locator = page.get_by_text("Counter");
    let screen = page.screen();
    let matches = locator.resolve(&screen);
    assert!(!matches.is_empty(), "should find 'Counter' on screen");
}

#[test]
fn expect_screen_assertions() {
    let _guard = demo_test_lock();
    let page = spawn_demo();
    page.wait_for_text("Counter", Duration::from_secs(5))
        .unwrap();

    page.expect_screen()
        .to_contain_text("Counter: 0")
        .expect("screen should contain 'Counter: 0'");
}

#[test]
fn screen_snapshot_has_correct_dimensions() {
    let _guard = demo_test_lock();
    let page = spawn_demo();
    page.wait_for_text("Counter", Duration::from_secs(5))
        .unwrap();

    let screen = page.screen();
    assert_eq!(screen.cols, 80);
    assert_eq!(screen.rows, 24);
    assert_eq!(
        screen.cells.len(),
        80 * 24,
        "cells should match cols * rows"
    );
}
