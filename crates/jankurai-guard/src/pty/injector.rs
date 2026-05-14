//! The banner injector. A [`BannerInjector`] subscribes to the [`DenialBus`] and
//! writes a formatted failure banner straight into the agent's PTY master, so a
//! block appears inline in the agent's terminal the moment it happens.

use crate::feedback::banner::format_banner;
use crate::feedback::{DenialBus, GuardEvent};
use std::io::Write;
use std::sync::Arc;
use std::thread::JoinHandle;

/// A running banner-injection thread. Dropping the handle does not stop the
/// thread; call [`BannerInjector::join`] after the PTY writer is closed.
pub struct BannerInjector {
    thread: JoinHandle<()>,
}

impl BannerInjector {
    /// Spawns a thread that subscribes to `bus` and writes a banner into
    /// `writer` for every [`GuardEvent::Block`]. The thread exits when the bus
    /// drops every sender (which happens when the guard shuts down) or when the
    /// writer fails (the PTY closed).
    pub fn spawn<W>(bus: &Arc<DenialBus>, mut writer: W) -> Self
    where
        W: Write + Send + 'static,
    {
        let receiver = bus.subscribe();
        let thread = std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                if let GuardEvent::Block { decision, .. } = event {
                    let banner = format_banner(&decision);
                    if writer.write_all(banner.as_bytes()).is_err() {
                        break;
                    }
                    if writer.flush().is_err() {
                        break;
                    }
                }
            }
        });
        Self { thread }
    }

    /// Waits for the injector thread to finish.
    pub fn join(self) {
        let _ = self.thread.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit_client::MockAuditClient;
    use crate::AuditClient;
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// A writer that records everything written, shareable across threads.
    #[derive(Clone, Default)]
    struct SharedSink(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedSink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn injects_banner_on_block_event() {
        let bus = Arc::new(DenialBus::new());
        let sink = SharedSink::default();
        let injector = BannerInjector::spawn(&bus, sink.clone());

        let decision = MockAuditClient::always_block()
            .audit(
                std::path::Path::new("."),
                std::path::Path::new("a.rs"),
                b"x",
            )
            .unwrap();
        bus.publish(GuardEvent::Block {
            rel_path: PathBuf::from("a.rs"),
            decision: Box::new(decision),
            report_path: PathBuf::from(".jankurai/guard/LAST_FAILURE.md"),
        });
        // Dropping the bus closes every sender so the injector thread exits.
        drop(bus);
        injector.join();

        let written = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
        assert!(written.contains("JANKURAI GUARD: BLOCKED"));
    }
}
