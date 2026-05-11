# Periodic Cron Security Assumptions

1. **Decoupled Execution**: Evaluating a cron expression does NOT execute the target code. It strictly enqueues a background job payload. This bounds the blast radius of cron parsing bugs.
2. **Leader Election / Locking**: The `claim_due_schedules` implementation MUST utilize a DB-level row lock (e.g., `FOR UPDATE SKIP LOCKED`) or explicit leader election to prevent duplicate dispatches across horizontally scaled scheduler instances.
3. **No Embedded Secrets**: Cron payloads are tracked by opaque `payload_ref` identifiers. Evaluators have no access to environment secrets or sensitive job arguments.
4. **Denial of Service (DoS)**: Very high-frequency cron expressions (e.g., sub-second evaluation) must be capped at the application boundary to prevent DB thrashing.
