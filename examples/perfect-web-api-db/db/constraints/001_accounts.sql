-- Constraint documentation for accounts table
--
-- These constraints are enforced in the migration but documented here
-- for agent visibility and audit-lane reference.

-- 1. Email uniqueness: UNIQUE constraint on accounts.email
--    Domain invariant: no two accounts share an email address.
--    Enforced: migration 001_init.sql + domain::Account::new() validation.

-- 2. Email format: CHECK (email <> '' AND email LIKE '%@%')
--    Domain invariant: email must be non-empty and contain @.
--    Enforced: migration 001_init.sql + domain::Account::new() validation.

-- 3. Role enum: account_role ENUM ('owner', 'admin', 'member', 'viewer')
--    Domain invariant: role is one of the declared RBAC roles.
--    Enforced: migration 001_init.sql + domain::Role enum.

-- 4. Organization membership: FK accounts.organization_id → organizations.id
--    Domain invariant: an account's organization must exist.
--    Enforced: migration 001_init.sql foreign key.

-- 5. Organization member limit: CHECK (member_limit > 0)
--    Domain invariant: organizations must have a positive member limit.
--    Enforced: migration 001_init.sql + domain::Organization::can_add_member().

-- 6. Resource ownership: FK resources.owner_id → accounts.id
--    Domain invariant: every resource has a valid owner account.
--    Enforced: migration 001_init.sql foreign key.

-- 7. Audit append-only: application-layer policy, no UPDATE/DELETE on audit_events.
--    Domain invariant: audit events are immutable once written.
--    Enforced: application layer (no delete/update commands exist).
--    Recommended: add RLS or trigger to prevent UPDATE/DELETE in production.
