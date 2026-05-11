use tokio::sync::mpsc;

pub fn open() {
    let _ = mpsc::unbounded_channel::<String>();
}
