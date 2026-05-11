-- Ensure status column is not empty
ALTER TABLE background_job_runs ADD CONSTRAINT chk_status_not_empty CHECK (status <> '');
