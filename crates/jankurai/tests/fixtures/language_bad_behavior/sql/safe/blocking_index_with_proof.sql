-- jankurai:migration-safe reason=new-table (orders created in this same migration above)
CREATE INDEX idx_orders_status ON orders(status);
