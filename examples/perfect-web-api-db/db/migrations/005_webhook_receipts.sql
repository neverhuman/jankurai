-- 005_webhook_receipts.sql
CREATE TABLE webhook_receipts (
    receipt_id UUID PRIMARY KEY,
    provider VARCHAR(255) NOT NULL,
    idempotency_key VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(provider, idempotency_key)
);
