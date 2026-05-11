# Billing Security Assumptions

1. **Local Truth Strategy**: Gated application features must rely on the local `billing_subscriptions.status`. A failure to reach the payment provider must not lock a user out if their local state is still `active`.
2. **Idempotent Webhooks**: All webhook events mutating billing state (e.g., `invoice.payment_succeeded`, `customer.subscription.deleted`) must be processed idempotently through the `webhook-receiver` cell to prevent duplicate plan transitions.
3. **Grace Periods**: Transitioning from `active` to `past_due` requires explicit domain logic (usually driven by provider webhooks). The application layer should handle `past_due` with a grace period rather than immediate hard-locking.
4. **Provider Isolation**: Raw provider identifiers (like Stripe Customer IDs or Payment Method IDs) should only be visible to the adapter layer and should never leak into client-side JWTs or core domain models.
