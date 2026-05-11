-- 006_notifications.sql constraints
ALTER TABLE notification_outbox ADD CONSTRAINT status_check CHECK (status IN ('queued', 'delivered', 'failed'));
