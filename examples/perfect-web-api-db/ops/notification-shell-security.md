# Notification Shell Security

1. **PII Scrubbing**: Before a notification payload is inserted into the `notification_outbox` for dispatch by third-party providers (e.g., SendGrid, APNs), it must be verified that sensitive PII (Personally Identifiable Information) or secrets (like raw password reset links without expiration) are handled securely or scrubbed.
2. **Rate Limiting**: The adapter layer must respect third-party API rate limits to prevent account suspension.
