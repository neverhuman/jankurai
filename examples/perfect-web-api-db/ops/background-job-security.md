# Background Job Security Notes

The background-job cell treats asynchronous work as durable product truth. The
safe default is a certified shell: queue state, retry policy, audit evidence, and
operator UX are declared and provable, while provider-specific execution remains
outside the certified surface until separately reviewed.

## Assumptions

- Enqueue commands run through the auth/session cell and RBAC application policy.
- Worker claims use authenticated service or admin principals, not anonymous
  queue consumers.
- Payloads are stored elsewhere and referenced through `payload_ref` values.
- Retry and exhaustion decisions are made before adapter execution commits
  follow-up effects.
- Every terminal transition is audit-visible.

## Threats And Controls

| Threat | Control |
| --- | --- |
| payload secret leakage | only opaque `payload_ref` crosses the job boundary |
| unbounded retry storm | `BackgroundJobRetryPolicy` caps attempts and backoff |
| stolen worker credential | claims require auth/session identity and RBAC authority |
| duplicate side effects | provider-backed expansion must prove idempotency keys first |
| invisible failures | failures emit audit events and operator UX failed/exhausted states |
| destructive backfill | mutating installer/provider behavior remains deferred |

## Required Proof Lanes

- `test-cli` for manifest and domain/application smoke coverage
- `audit` for owner/proof routing and generated evidence visibility
- `db-migration-analyze` for durable queue migration and constraints
- `ux-qa` for operator states and permission-denied coverage
- `security` for credential, workflow, and supply-chain assumptions

## Provider Expansion Gates

Do not certify a provider-backed queue adapter until the patch includes:

1. idempotency contract for every job kind
2. lock expiry or visibility-timeout proof
3. dead-letter queue policy with human review route
4. credential storage and rotation notes
5. replay/backfill runbook
6. destructive rollback guard
7. dedicated proof receipt under `target/jankurai/`

## Exception Policy

Temporary exceptions must include:

- owner
- exact queue or job kind
- expiry date
- compensating control
- proof lane to remove the exception

Permanent exceptions are not accepted for raw payload logging, unbounded retries,
anonymous worker claims, or silent exhaustion.

## Operator Review Checklist

- Failed and exhausted jobs show actor, queue, job kind, and last safe error
  summary.
- Operators can distinguish retryable failures from exhausted work.
- No dashboard exposes raw payload bodies.
- Replay actions remain disabled until a future mutating/provider-backed cell is
  certified.
- Every replay/backfill plan includes rollback and audit evidence.
