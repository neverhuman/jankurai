-- 006_notifications.sql
CREATE TABLE notification_outbox (
    message_id UUID PRIMARY KEY,
    recipient_id UUID NOT NULL,
    payload TEXT NOT NULL,
    status VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
