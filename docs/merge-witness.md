# Merge Witness

`jankurai witness` is the v0.5 merge receipt. It does not run proof commands. It reads changed paths, owner/test routes, generated zones, proof receipts, the current audit score, and an optional accepted baseline, then emits a merge decision.

```bash
jankurai witness . \
  --changed-from origin/main \
  --baseline agent/repo-score.json \
  --out target/jankurai/merge-witness.json \
  --md target/jankurai/merge-witness.md
```

Decision meanings:

- `pass`: score is not regressed, required lanes have receipts, and no new blocking findings were introduced.
- `review`: evidence exists but a reviewer should accept the baseline, new low/medium findings, or changed caps.
- `block`: required proof evidence is missing, generated zones were touched without regeneration proof, or the current audit is failing.
- `ratchet_fail`: the score dropped below the accepted baseline.
- `release_fail`: reserved for release-contract extensions.

The JSON validates against `schemas/merge-witness.schema.json`. The Markdown renderer includes a PR proof matrix and a next-repair list suitable for review comments.
