-- Constraints for Billing Subscriptions

-- Ensure valid subscription status
ALTER TABLE billing_subscriptions ADD CONSTRAINT chk_billing_subscriptions_status CHECK (status IN ('trialing', 'active', 'past_due', 'canceled', 'incomplete'));

-- Ensure plan amount is non-negative
ALTER TABLE billing_plans ADD CONSTRAINT chk_billing_plans_amount CHECK (amount >= 0);

-- Ensure invoice amounts are non-negative
ALTER TABLE billing_invoices ADD CONSTRAINT chk_billing_invoices_amount CHECK (amount_due >= 0 AND amount_paid >= 0);

-- Ensure current period end is after start
ALTER TABLE billing_subscriptions ADD CONSTRAINT chk_billing_subscriptions_period CHECK (current_period_end > current_period_start);
