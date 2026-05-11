-- Migration 007: Periodic Cron Schedules

CREATE TABLE periodic_cron_schedules (
    id TEXT PRIMARY KEY,
    expression TEXT NOT NULL,
    payload_ref TEXT NOT NULL,
    is_paused BOOLEAN NOT NULL DEFAULT false,
    last_run_at TIMESTAMP WITH TIME ZONE,
    next_run_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_periodic_cron_next_run ON periodic_cron_schedules(next_run_at) WHERE is_paused = false;
