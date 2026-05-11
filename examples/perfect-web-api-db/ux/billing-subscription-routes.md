# UX Routes: Billing & Subscription

## `/app/settings/billing`
**State**: Overview of the tenant's current subscription.
- Data required: `Subscription`, List of `Invoice`
- Subscribed state: Displays the active plan name, current period end date, and renewal cost. Includes a button to "Manage Subscription" (which may redirect to a provider-hosted portal like Stripe Customer Portal).
- Unsubscribed state: Displays "No active plan." Prompts the user to "View Plans."
- Invoice history: A table of past invoices showing Date, Amount, and Status.

## `/app/settings/billing/plans`
**State**: Plan selection and upgrade/downgrade flows.
- Data required: `[SubscriptionPlan]`, current `Subscription`
- Loaded state: Displays available pricing tiers (e.g., Hobby, Pro, Enterprise). 
- Active indicator: Highlights the user's current plan.
- Action: "Subscribe" or "Upgrade". Opens a checkout modal or redirects to a provider checkout session.
