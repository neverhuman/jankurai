# Billing & Subscription Cell

The `billing-subscription` cell provides the core application boundary for monetizing the product. It explicitly separates the local representation of the subscription truth (which determines what features a tenant can access) from the external payment provider's state (which determines how much they pay and when).

## Architecture

1. **State Ownership**: The local database (`billing_subscriptions` table) acts as the source of truth for authorization decisions. If the local state says `active`, the tenant gets access.
2. **Provider Sync**: External providers (like Stripe) communicate changes through webhooks (ingested via the `webhook-receiver` cell). The application translates these events into state transitions (`trialing`, `active`, `past_due`, `canceled`).
3. **Plan Catalog**: Product features are gated by `plan_id`. The application maintains a synchronized `billing_plans` table to avoid querying the provider for basic feature tier logic.

## Integration

Use the `BillingSubscriptionStatePolicy` marker to enforce that your application logic checks the local `SubscriptionStatus` before authorizing gated resources. Providers SDKs should be strictly isolated to the adapter layer.
