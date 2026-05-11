// Webhook Receiver Cell
// Source: perfect-web-api-db
// Status: Candidate

pub struct WebhookSignaturePolicy {
    // Requires application edge to verify signature before parsing body
}

pub struct WebhookReceipt {
    pub receipt_id: String,
    pub provider: String,
    pub idempotency_key: String,
    pub status: String,
}

pub trait WebhookReceiptStore {
    fn record_receipt(&self, receipt: WebhookReceipt) -> Result<(), String>;
    fn check_duplicate(&self, provider: &str, idempotency_key: &str) -> Result<bool, String>;
}
