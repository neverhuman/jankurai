# Background Job UX Routes

The background-job cell exposes operator UX proof states without making the UI
the owner of durable queue truth. Routes are reference states for certification,
not a provider-backed worker console.

## Route Inventory

| Route | Purpose | Required states |
| --- | --- | --- |
| `/admin/background-jobs` | queue overview | loading, empty, queued, running, failed, exhausted, permission denied |
| `/admin/background-jobs/:job_id` | job detail | loading, not found, running, completed, failed, exhausted, audit trail |
| `/admin/background-jobs/:job_id/retry-plan` | non-mutating replay review | eligible, blocked, provider deferred, approval required |

## Shared UX Rules

- Show `queue`, `kind`, `status`, `attempts`, `max_attempts`, and next run time.
- Show `payload_ref` as an opaque reference only; never render payload bodies.
- Distinguish `failed but retryable` from `exhausted`.
- Show permission-denied copy for non-admin users.
- Label replay/backfill controls as unavailable until provider-backed mutation is
  certified.
- Link every terminal state to audit evidence.

## State Matrix

| State | Expected proof |
| --- | --- |
| loading | skeleton rows do not claim queue health |
| empty | confirms no due jobs for selected queue/filter |
| queued | shows next run time and bounded retry policy |
| running | shows worker/lock reference without secrets |
| completed | shows completion time and audit event link |
| failed | shows last safe error summary and next retry time |
| exhausted | shows dead-letter/review guidance, no automatic replay |
| permission denied | explains required admin/worker role |

## Accessibility And Safety

- Status badges must have text equivalents.
- Failed/exhausted rows must not rely on color alone.
- Keyboard users can filter by status and queue.
- Dangerous replay/backfill controls are disabled with explanatory text until a
  future provider-backed cell certifies mutating behavior.

## Proof Commands

```bash
just ux-qa
jankurai cell . --cell-id background-job --mode prove \
  --out target/jankurai/p10-background-job-prove.json \
  --md target/jankurai/p10-background-job-prove.md
```

UX proof should be stored under `target/jankurai/` and referenced from the Phase
10 log when a rendered implementation is added.
