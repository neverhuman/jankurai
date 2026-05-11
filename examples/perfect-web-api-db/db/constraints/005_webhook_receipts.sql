-- 005_webhook_receipts.sql constraints
ALTER TABLE webhook_receipts ADD CONSTRAINT status_check CHECK (status IN ('processing', 'completed', 'failed'));
