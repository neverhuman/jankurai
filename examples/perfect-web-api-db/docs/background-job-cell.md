# Background Job Certified Cell

The background-job cell is the sixth Phase 10 certified reuse primitive. It is a
bounded workflow shell for durable asynchronous work in the
`perfect-web-api-db` reference platform. It intentionally certifies the durable
contract and proof boundary, not a provider-backed queue runtime.

## What It Owns

- `BackgroundJob`, `JobId`, `PayloadRef`, `JobKind`, and `JobStatus` value objects
- `BackgroundJobRetryPolicy` and deterministic retry/exhaustion decisions
- application-level enqueue, claim, complete, and fail commands
- repository and audit-log port traits for adapters
- OpenAPI shape for queue operations
- migration and constraint shell for durable job state
- UX proof states for operators reviewing queued/running/failed work

## What It Rejects

- raw payload bodies in source, logs, proof artifacts, or manifests
- provider-specific queue clients inside domain or application layers
- wall-clock reads or random IDs inside pure job decisions
- silent retries without audit-log evidence
- worker claims that bypass auth/session and RBAC authorization
- mutating installer behavior before conflict and rollback proof exists

## Boundary Map

| Layer | Owned by this cell | Not owned by this cell |
| --- | --- | --- |
| Domain | job IDs, payload references, status transitions, retry policy | queue provider SDKs, clocks, random IDs |
| Application | enqueue/claim/complete/fail orchestration and audit events | HTTP extraction, SQL execution, cron triggers |
| Adapter | repository and audit-log traits only | provider queue clients and worker daemons |
| DB | durable tables, constraints, claimable indexes | destructive backfills or provider-specific migrations |
| UX | operator states and permission-denied proof | frontend-owned durable truth |

## Evidence Map

The Phase 10 manifest for `background-job` requires evidence from:

- `examples/perfect-web-api-db/backend/src/background_job.rs`
- `examples/perfect-web-api-db/contracts/background-job.openapi.json`
- `examples/perfect-web-api-db/db/migrations/004_background_jobs.sql`
- `examples/perfect-web-api-db/db/constraints/004_background_jobs.sql`
- `examples/perfect-web-api-db/docs/background-job-cell.md`
- `examples/perfect-web-api-db/ops/background-job-security.md`
- `examples/perfect-web-api-db/ux/background-job-routes.md`
- proof lanes: `test-cli`, `audit`, `db-migration-analyze`, `ux-qa`, `security`
- dependency closure: `audit-log`, `rbac`, `auth-session`, `organization-team`
- content marker: `BackgroundJobRetryPolicy`

## Command Surface

```bash
jankurai registry . \
  --out target/jankurai/p10-background-job-registry.json \
  --md target/jankurai/p10-background-job-registry.md

jankurai cell . --cell-id background-job --mode prove \
  --out target/jankurai/p10-background-job-prove.json \
  --md target/jankurai/p10-background-job-prove.md

jankurai cell . --cell-id background-job --mode upgrade-plan \
  --out target/jankurai/p10-background-job-upgrade.json \
  --md target/jankurai/p10-background-job-upgrade.md
```

Install mode remains dry-run only and uses `never-overwrite`. Prove mode emits
evidence and proof commands but does not execute worker jobs or mutate provider
queues.

## Operational Guarantees

- Every enqueue writes durable job state and audit evidence.
- Every claim requires an authenticated account with admin/worker authority.
- Every failure either schedules a bounded retry or records exhaustion.
- Payload material is represented by an opaque `payload_ref`; it is not copied
  into manifests, logs, or proof receipts.
- Retry math is deterministic and testable without wall-clock reads.
- DB constraints make impossible terminal/running states easier to reject.

## Upgrade Gates

Provider-backed runtime expansion is intentionally deferred until a future cell
can prove these gates:

1. idempotency keys for every mutating job kind
2. visibility-timeout and lock-expiry behavior
3. dead-letter semantics and exhaustion dashboards
4. replay-safe migration and rollback commands
5. secret handling for provider queue credentials
6. worker authorization that ties service principals to RBAC policy

## Rollback Notes

- Pause workers before rolling back schema, policy, or adapter behavior.
- Drain or dead-letter queued jobs before removing a queue provider.
- Reverse table changes only through reviewed migrations.
- Keep audit events intact even when jobs are cancelled or dead-lettered.

## Completion Receipt

- Cell: `background-job`
- Phase: 10 reuse registry certified cells
- Status: certified shell, provider-backed runtime deferred
- Dependencies: `audit-log`, `rbac`, `auth-session`, `organization-team`
- Proof: manifest evidence plus Rust smoke tests and route/security docs
- Residual risk: no mutating installer; no external provider adapter; no live worker daemon
