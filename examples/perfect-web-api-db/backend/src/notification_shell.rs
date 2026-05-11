// Notification Shell Cell
// Source: perfect-web-api-db
// Status: Candidate

pub struct NotificationDeliveryPolicy {
    // Requires application layer to determine delivery mechanism (email vs sms) based on urgency.
    // Adapters are responsible for the actual side-effect.
}

pub struct NotificationOutbox {
    pub message_id: String,
    pub recipient_id: String,
    pub payload: String, // Scrubbed payload
    pub status: String,
}

pub trait NotificationDeliveryStore {
    fn record_outbox(&self, outbox: NotificationOutbox) -> Result<(), String>;
    fn mark_delivered(&self, message_id: &str) -> Result<(), String>;
}
