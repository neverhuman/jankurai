# jankurai Exceptions

Document dated exceptions with owner, reason, expiry, migration plan, and proof lane.

Use YAML front matter at the top of each exception file:

```md
---
code: HB_CONTRACT_DRIFT
owner: platform
reason: Manual API type drift is blocked until contracts are regenerated.
expires: 2026-12-31
migration_plan: Regenerate the contract, commit the diff, and rerun the contract lane.
proof_lane: just score
repair_guidance: Keep the exception narrow and remove it after the contract fix lands.
---
```

Exception files live under `docs/exceptions/NNNN-title.md`. The expiry loop treats
missing or malformed front matter as invalid and expired dates as blocked.
