-- Migration 001: Initial schema for perfect-web-api-db reference platform.
--
-- Invariants enforced at DB level:
--   - Account email must be unique and non-empty.
--   - Account role must be one of the declared domain roles.
--   - Resource ownership references a valid account.
--   - Audit events are append-only (no UPDATE/DELETE allowed by app).
--
-- Rollback: DROP TABLE audit_events, resources, organizations, accounts;

CREATE TYPE account_role AS ENUM ('owner', 'admin', 'member', 'viewer');

CREATE TABLE accounts (
    id              TEXT PRIMARY KEY,
    email           TEXT NOT NULL UNIQUE CHECK (email <> '' AND email LIKE '%@%'),
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    role            account_role NOT NULL DEFAULT 'member',
    organization_id TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE organizations (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL CHECK (name <> ''),
    member_limit    INTEGER NOT NULL DEFAULT 50 CHECK (member_limit > 0),
    member_count    INTEGER NOT NULL DEFAULT 0 CHECK (member_count >= 0),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Add the FK after both tables exist
ALTER TABLE accounts
    ADD CONSTRAINT fk_accounts_organization
    FOREIGN KEY (organization_id) REFERENCES organizations(id);

CREATE TABLE resources (
    id              TEXT PRIMARY KEY,
    title           TEXT NOT NULL CHECK (title <> ''),
    body            TEXT NOT NULL DEFAULT '',
    owner_id        TEXT NOT NULL REFERENCES accounts(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE audit_events (
    id              BIGSERIAL PRIMARY KEY,
    actor_id        TEXT NOT NULL,
    action          TEXT NOT NULL,
    target_kind     TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    outcome         TEXT NOT NULL CHECK (outcome IN ('success', 'denied', 'error')),
    detail          TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Index for common queries
CREATE INDEX idx_resources_owner ON resources(owner_id);
CREATE INDEX idx_audit_events_actor ON audit_events(actor_id);
CREATE INDEX idx_audit_events_created ON audit_events(created_at);
CREATE INDEX idx_accounts_org ON accounts(organization_id);
