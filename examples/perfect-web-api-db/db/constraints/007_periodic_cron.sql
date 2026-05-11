-- Constraints for Periodic Cron Schedules

-- Ensure the cron expression is not empty (simple constraint, full validation belongs in Rust domain layer)
ALTER TABLE periodic_cron_schedules ADD CONSTRAINT chk_periodic_cron_expression_not_empty CHECK (length(expression) > 0);

-- Ensure payload_ref is not empty
ALTER TABLE periodic_cron_schedules ADD CONSTRAINT chk_periodic_cron_payload_ref_not_empty CHECK (length(payload_ref) > 0);

-- Ensure next_run_at is always greater than or equal to last_run_at if both are set
ALTER TABLE periodic_cron_schedules ADD CONSTRAINT chk_periodic_cron_next_run_after_last CHECK (last_run_at IS NULL OR next_run_at IS NULL OR next_run_at >= last_run_at);
