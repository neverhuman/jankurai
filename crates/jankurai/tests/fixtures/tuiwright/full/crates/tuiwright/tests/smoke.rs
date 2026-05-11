use std::time::Duration;
use tuiwright::{Key, Page, SpawnConfig};

#[test]
fn covered_wait_action_and_screenshot_flow() {
    let page = Page::spawn(
        SpawnConfig::new("demo")
            .trace_path("target/tuiwright/full.trace.jsonl")
            .size(80, 24),
    )
    .expect("spawn");
    page.wait_for_text("Ready", Duration::from_secs(5)).unwrap();
    page.press(Key::Enter).unwrap();
    page.expect_screen().to_contain_text("Ready").unwrap();
    page.screenshot("target/tuiwright/full.png").unwrap();
}

#[test]
fn covered_regex_locator_and_recording_flow() {
    let page = Page::spawn(SpawnConfig::new("demo").size(80, 24)).expect("spawn");
    page.wait_for_regex("Loaded \\d+ items", Duration::from_secs(5))
        .unwrap();
    page.type_text("hello").unwrap();
    page.paste("more input").unwrap();
    page.resize(120, 40).unwrap();
    let locator = page.get_by_text("Loaded");
    page.expect_locator(&locator).to_be_visible().unwrap();
    page.stop_recording_gif("target/tuiwright/full.gif", Default::default())
        .unwrap();
    page.trace_path("target/tuiwright/full.trace.jsonl");
}
