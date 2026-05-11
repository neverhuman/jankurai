# Organization/Team Certified Cell

Status: certified shell
Cell ID: `organization-team`
Depends on: `audit-log`, `rbac`, `auth-session`

## Intent

This cell gives a Jankurai-native product a bounded organization/team primitive:

- create organization-scoped teams
- add team members with a small role vocabulary
- keep tenant identifiers explicit on every team and membership
- route membership decisions through RBAC and authenticated session identity
- emit audit evidence for membership-changing actions

It is intentionally a shell. It does not install SCIM, SAML directory sync,
billing seats, invitation email delivery, or provider-backed account linking.
Those are provider-dependent cells and need their own contracts, security
assumptions, and proof receipts.

## Source And Generated Boundaries

Source truth:

- `backend/src/organization_team.rs`
- `contracts/organization-team.openapi.json`
- `db/migrations/003_organization_team.sql`
- `db/constraints/003_organization_team.sql`
- `ux/organization-team-routes.md`

No generated client is committed for this shell. If a generated client is added,
declare it in `agent/generated-zones.toml`, regenerate from the OpenAPI source,
and do not hand-edit the generated output.

## Proof Contract

Required lanes:

- `test-cli` for Rust domain/application invariants
- `audit` for ownership, routing, generated-zone, and registry evidence
- `db-migration-analyze` for migration safety
- `ux-qa` for membership UI state evidence
- `security` for tenancy, secret, and CI hardening checks

The registry certification also requires dependency evidence for `audit-log`,
`rbac`, and `auth-session` because membership mutation without identity,
authorization, and audit receipts would be unsafe.

## RBAC Rules

- Only active accounts with `manage_members` authorization may create teams or
  add members.
- Membership roles are `manager`, `contributor`, and `viewer`.
- `manager` can manage team members inside the team model; broader organization
  owner/admin policy remains in the RBAC cell.
- Permission-denied states must remain visible in the UX route matrix.

## Security Assumptions

- Team and membership rows are always tenant-scoped.
- Cross-organization membership moves require explicit human review and new
  tests; do not model them as an ordinary update.
- Provider directory sync is not implied by this shell.
- Billing-seat enforcement is not implied by this shell.
- Invitation tokens must not be added without an auth/session extension and
  secret-handling proof.

## Observability Events

- `organization.team.created`
- `organization.team.archived`
- `organization.team.member_added`
- `organization.team.member_removed`

Each event should include actor ID, organization ID, team ID, outcome, and a
trace/correlation ID when wired through the API edge.

## Upgrade Path

Safe follow-ons:

1. Add invitation flows as a separate `organization-invite` cell.
2. Add SCIM/provider directory sync as a provider-backed cell.
3. Add billing-seat reconciliation only after billing contracts exist.
4. Generate TypeScript clients from `organization-team.openapi.json` and declare
   the generated zone before adding frontend API calls.

## Rollback

The install mode remains dry-run only. For applied templates, archive teams and
preserve audit events before removing membership data. Reverse role vocabulary or
membership table changes only through reviewed migrations and `db-migration-analyze`.
