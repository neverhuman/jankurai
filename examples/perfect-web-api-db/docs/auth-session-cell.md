# Auth/Session Certified Cell

The `auth-session` cell is the fourth Phase 10 certified cell. It depends on:

- `rbac` for authorization decisions after identity is established.
- `audit-log` for session lifecycle and failed-authentication evidence.

## Scope

This cell owns a provider-neutral session shell:

- session identity and token-hash invariants
- session creation policy (TTL, max active sessions)
- expiry and revocation decisions
- application port traits for session persistence and session audit events
- OpenAPI session lifecycle shape
- PostgreSQL session storage migration
- UX route/state matrix for sign-in, current session, and admin revocation

It does not own provider-specific authentication integrations. OAuth, SAML, passkeys, LDAP, or password verification belong in adapters or provider-specific cells with their own security proof.

## Source Surfaces

- `backend/src/auth_session.rs` — pure/session application shell
- `contracts/auth-session.openapi.json` — source contract for session endpoints
- `db/migrations/002_auth_sessions.sql` — durable session table
- `db/constraints/002_auth_sessions.sql` — agent-visible invariant mapping
- `ux/auth-session-routes.md` — rendered state coverage
- `ops/auth-session-security.md` — threat model and proof requirements

## Certification Requirements

A certified auth/session cell must prove:

- raw session tokens are not stored
- session state is durable and migration-backed
- session creation and revocation have stable problem-detail errors
- session routes have loading, success, error, denied, expired, and revoked states
- authentication does not move authorization out of the RBAC/domain layer
- session lifecycle events are observable and audit-friendly
- all dependencies are certified before this cell claims certification

## Upgrade Policy

Provider-backed auth must be added as a separate adapter or provider cell. The shell contract may expand, but it must not add provider secrets, credential verification, or durable authorization policy to frontend code.

## Deprecation Policy

Deprecating this cell requires a replacement identity/session cell and migration guidance for active sessions. Old receipts must remain readable so historical proof still explains which session invariants were certified.
