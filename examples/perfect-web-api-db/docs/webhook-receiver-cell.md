# Webhook Receiver Cell

The Webhook Receiver cell provides a durable, idempotent entrypoint for external service callbacks. It relies on the `audit-log` cell for logging and the `background-job` cell for asynchronous processing.

## Security
Signature verification must happen at the application edge before the payload is deserialized or processed. See `ops/webhook-receiver-security.md`.

## Durability
Webhooks are recorded in the `webhook_receipts` table to prevent duplicate processing. Processing is handed off to the background job queue.
