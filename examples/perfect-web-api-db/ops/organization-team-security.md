# Organization/Team Security Notes

The `organization-team` cell is tenant and authorization sensitive. Keep the
surface boring, explicit, and easy for agents to prove.

## Required Guards

- Authenticated session identity is required before any team mutation.
- RBAC `manage_members` authorization is required for create/add/archive flows.
- Every team row carries `organization_id`.
- Every membership row references a concrete team and account.
- Duplicate membership is blocked by `PRIMARY KEY (team_id, account_id)`.
- Team names are unique per organization.
- Audit events are emitted for team creation, archival, and membership changes.

## Never-Auto Changes

The following require a human-reviewed plan and fresh proof:

- cross-organization team or membership moves
- invitation token issuance
- SCIM/SAML/OIDC directory sync
- billing-seat enforcement
- role vocabulary changes that affect production authorization
- hard-deleting teams or memberships with existing audit events

## Evidence Expectations

For registry proof, collect:

- Rust invariant tests for `TeamMembershipPolicy`
- OpenAPI contract review for team and membership routes
- DB migration analysis for membership tables and constraints
- UX state coverage for list, empty, create, permission-denied, and error states
- security lane evidence showing no secrets or provider credentials in fixtures

## Failure Modes

- A UI-only membership change is a product truth leak. Fix by routing through the
  Rust application command and generated/contract-backed API.
- A migration without rollback notes is unsafe. Add rollback/backfill evidence or
  block the change.
- Provider sync text without a provider contract overclaims the shell. Move it to
  a future provider-backed cell.
- Billing-seat language without billing proof overclaims the shell. Keep it as a
  follow-on dependency.

## Residual Risk

This certified shell proves the local organization/team primitive, not a full
enterprise directory product. Provider-backed installation and runtime mutation
remain deferred by policy.
