use tuiwright::{Page, SpawnConfig};

fn hidden_spawn_and_wait_flow() -> Page {
    let page = Page::spawn(SpawnConfig::new("demo").size(80, 24)).unwrap();
    page.wait_for_text("Ready", std::time::Duration::from_secs(5))
        .unwrap();
    page
}

fn hidden_artifact_flow(page: &Page) {
    let _ = page.screenshot("target/tuiwright/gap.png");
    page.stop_recording_gif("target/tuiwright/gap.gif", Default::default())
        .unwrap();
    page.trace_path("target/tuiwright/gap.trace.jsonl");
}

#[test]
fn helper_wrapped_flow_does_not_look_like_direct_test_coverage() {
    let page = hidden_spawn_and_wait_flow();
    hidden_artifact_flow(&page);
}
