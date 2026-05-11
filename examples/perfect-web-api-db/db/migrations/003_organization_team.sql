-- Migration 003: Organization/team certified-cell shell.
--
-- Invariants enforced at DB level:
--   - Team names are non-empty and unique within an organization.
--   - Team rows remain tenant-scoped through organization_id.
--   - Membership role is one of the declared organization/team roles.
--   - A member can hold one membership row per team.
--
-- Rollback: DROP TABLE organization_team_memberships, organization_teams;

CREATE TYPE organization_team_role AS ENUM ('manager', 'contributor', 'viewer');

CREATE TABLE organization_teams (
    id              TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id),
    name            TEXT NOT NULL CHECK (name <> ''),
    archived        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

CREATE TABLE organization_team_memberships (
    team_id    TEXT NOT NULL REFERENCES organization_teams(id),
    account_id TEXT NOT NULL REFERENCES accounts(id),
    role       organization_team_role NOT NULL DEFAULT 'viewer',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, account_id)
);

CREATE INDEX idx_organization_teams_org
    ON organization_teams(organization_id);

CREATE INDEX idx_organization_team_memberships_account
    ON organization_team_memberships(account_id);

CREATE INDEX idx_organization_team_memberships_role
    ON organization_team_memberships(role);

-- Future provider sync and billing seat tables are intentionally omitted from
-- the certified shell. They require provider-specific contracts and security
-- receipts before the cell may mutate real installations.
