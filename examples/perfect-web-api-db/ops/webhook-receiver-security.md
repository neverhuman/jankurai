# Webhook Receiver Security

1. **Signature Verification**: Every webhook provider uses a different signature mechanism (e.g., HMAC-SHA256). The signature must be verified *before* the JSON payload is parsed.
2. **Idempotency**: Webhook receipts must be durably stored. If the same provider and idempotency key are received again, the system must short-circuit and return `202 Accepted` without reprocessing.
3. **Payload Sanitization**: Webhook payloads must be treated as untrusted input.
