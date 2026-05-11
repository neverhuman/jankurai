-- Background job runs table
CREATE TABLE background_job_runs (
    id SERIAL PRIMARY KEY,
    run_at TIMESTAMP NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued'
);
