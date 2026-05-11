# Auth/Session Security

## Token Storage

- No raw session token is stored in source, fixtures, logs, migrations, or database rows.
- Durable storage uses `token_hash` only.
- Token hash length must be at least 32 characters.
- Token generation, random number sources, and signing keys are adapter/provider responsibilities.

## Provider Boundary

The cell is provider-neutral. Local credentials, OIDC, SAML, and passkeys must enter through provider adapters. Provider adapters may verify identity, but RBAC authorization remains in the Rust domain/application layer.

## Session Lifecycle

- Session creation emits `session.created`.
- Session revocation emits `session.revoked`.
- Expiry detection emits or derives `session.expired`.
- Failed authentication emits `authentication.failed`.
- Events must not contain raw credentials or raw session tokens.

## Required Proof

- `security`: secret scan and dependency/supply-chain review.
- `db-migration-analyze`: migration safety and durable-truth review.
- `audit`: ownership, generated-zone, proof-lane, and boundary audit.
- `ux-qa`: rendered route-state proof for sign-in/session surfaces when UI is present.

## Failure Handling

Session errors must use typed problem details and stable reason codes. A failed credential check is not retryable by blind automation; agents may only repair configuration, routing, or contract drift with human-approved evidence.
