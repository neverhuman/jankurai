//! Realtime feedback plumbing. The guard turns every audit outcome into a
//! [`GuardEvent`] and broadcasts it on a [`DenialBus`]. Subscribers include the
//! PTY injector (which renders a banner into the agent's terminal) and the
//! status/failures commands.

pub mod banner;
pub mod report;

use crate::audit_client::GuardDecision;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::PathBuf;
use std::sync::Mutex;

/// An audit outcome worth surfacing to subscribers.
#[derive(Debug, Clone)]
pub enum GuardEvent {
    /// A candidate write was blocked. The decision is boxed because it is much
    /// larger than the other variants' payloads.
    Block {
        /// Repo-relative path of the blocked file.
        rel_path: PathBuf,
        /// The full audit decision behind the block.
        decision: Box<GuardDecision>,
        /// Path to the written failure report.
        report_path: PathBuf,
    },
    /// A candidate write passed audit and was allowed to land.
    Pass {
        /// Repo-relative path of the passing file.
        rel_path: PathBuf,
    },
    /// The guard hit an error while processing a path.
    Error {
        /// Repo-relative path the error concerns.
        rel_path: PathBuf,
        /// Human-readable error message.
        message: String,
    },
}

/// A fan-out broadcaster: every subscriber receives every published event. The
/// sender list is guarded by a mutex so subscriptions can be added at any time.
#[derive(Debug, Default)]
pub struct DenialBus {
    senders: Mutex<Vec<Sender<GuardEvent>>>,
}

impl DenialBus {
    /// Creates an empty bus with no subscribers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new subscriber and returns its receiver. The subscriber will
    /// receive every event published after this call.
    pub fn subscribe(&self) -> Receiver<GuardEvent> {
        let (tx, rx) = bounded(256);
        let mut guard = match self.senders.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.push(tx);
        rx
    }

    /// Publishes `event` to every live subscriber. Subscribers whose receiver
    /// has been dropped are pruned.
    pub fn publish(&self, event: GuardEvent) {
        let mut guard = match self.senders.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.retain(|tx| tx.send(event.clone()).is_ok());
    }

    /// Returns the current number of live subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.senders.lock().map(|g| g.len()).unwrap_or(0)
    }
}

/// An RFC 3339 timestamp for the current instant, e.g. `2026-05-14T12:34:56Z`.
/// Shared by the report writer, the quarantine path builder and state records.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// A filesystem-safe variant of [`now_rfc3339`] with colons replaced, suitable
/// for use as a file or directory name.
pub fn now_stamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_subscriber_gets_every_event() {
        let bus = DenialBus::new();
        let a = bus.subscribe();
        let b = bus.subscribe();
        bus.publish(GuardEvent::Pass {
            rel_path: PathBuf::from("x.rs"),
        });
        assert!(matches!(a.recv().unwrap(), GuardEvent::Pass { .. }));
        assert!(matches!(b.recv().unwrap(), GuardEvent::Pass { .. }));
    }

    #[test]
    fn dropped_subscribers_are_pruned() {
        let bus = DenialBus::new();
        {
            let _short_lived = bus.subscribe();
        }
        bus.publish(GuardEvent::Pass {
            rel_path: PathBuf::from("x.rs"),
        });
        assert_eq!(bus.subscriber_count(), 0);
    }
}
