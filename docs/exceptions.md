# Agent exceptions and overrides

This document defines the agent-friendly exception pattern for the jankurai
hub: how an agent or maintainer requests, records, and bounds an override of a
standard rule. Exceptions are the only sanctioned way to deviate from the audit
baseline.

## Principle

The default answer is "follow the standard." An exception is a dated, owned,
expiring waiver for a specific rule on a specific path. Exceptions are data, not
prose: they live next to the configuration they govern and are reviewed on every
audit.

## How to request an exception

1. Identify the exact `rule_id` and `path` the exception applies to (from the
   audit JSON `findings[]` written to `.jankurai/repo-score.json`).
2. Add an entry to the relevant `agent/*.toml` manifest. Scan exclusions belong
   in the `[scan]` block of [`agent/audit-policy.toml`](../agent/audit-policy.toml);
   generated-zone declarations belong in
   [`agent/generated-zones.toml`](../agent/generated-zones.toml).
3. Every exception entry MUST carry:
   - `owner` — the team or person accountable.
   - `classification` — e.g. `generated`, `transient`, `vendor`.
   - `expires` — an ISO date after which the exception is invalid and the audit
     fails again.
   - `migration_path` — the concrete plan to remove the exception.

## Example

The hub's `.fusion/` and `target/` trees are generated and transient, so they
are excluded from the audit walk in
[`agent/audit-policy.toml`](../agent/audit-policy.toml):

```toml
[scan]
excluded_paths = [".fusion", ".jankurai", "target", "tips/"]
```

This is a standing exception classified `generated`/`transient`, owned by `ops`,
removable only when those trees stop being produced locally.

## Override review

- Every exception is re-evaluated on each `just audit` run.
- An expired exception is treated as a hard finding, not a pass.
- Removing an exception requires deleting its entry and proving the underlying
  rule now passes on its own.

## What is never excepted

Secret leakage, committed cross-repo path dependencies, branch-pinned release
inputs, and hand-edits to generated zones are never granted exceptions. Fix the
underlying cause instead.
