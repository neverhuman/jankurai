# Notification Shell Cell

The Notification Shell cell provides an abstraction layer for delivering messages to users without tangling business logic with third-party delivery providers (like SendGrid or Twilio).

It relies on the `background-job` cell for asynchronous processing and retries.

## Security
The domain layer must ensure that PII (Personally Identifiable Information) is appropriately scrubbed from any payloads before they are serialized into the `notification_outbox` if they are destined for insecure delivery channels. See `ops/notification-shell-security.md`.
