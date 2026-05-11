#[test]
fn uses_unbounded_channel_in_tests_only() {
    let _ = tokio::sync::mpsc::unbounded_channel::<String>();
}
