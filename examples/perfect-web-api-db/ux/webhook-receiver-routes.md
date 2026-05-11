# Webhook Receiver Routes

Webhooks do not typically have UI routes associated with them directly. However, administrative dashboards may show webhook receipt status.

## Admin View
- **Path**: `/admin/webhooks`
- **State**: List of recently received webhooks, grouped by provider and status.
- **Details**: Clicking a webhook receipt shows the idempotency key and processing status.
