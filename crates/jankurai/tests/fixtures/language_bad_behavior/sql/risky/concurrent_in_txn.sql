BEGIN;
CREATE INDEX CONCURRENTLY idx_orders_user_id ON orders(user_id);
COMMIT;
