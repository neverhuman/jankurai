# Jankurai Seed Conformance Suite

This directory contains seed fixtures for the `0.8.0` paper cut. The suite is
not a broad benchmark. It is local conformance evidence for the standard's
central claim: merge decisions should be reproducible from versioned artifacts.

Each fixture has a `jankurai-fixture.toml` manifest with changed paths,
expected audit and witness decisions, expected rules, and optional proof
receipts. Historical expectation summaries live in `expected/*.json`; the
release evidence is produced by the observed runner.

The seed suite is intentionally small:

- `hl3-pass-minimal`: known-good minimal repository shape.
- `ownerless-path-fail`: unmapped path should raise `HLT-003`.
- `unmapped-proof-fail`: path without proof route should raise `HLT-004`.
- `generated-zone-mutation-fail`: generated output changed without source proof should raise `HLT-002`.
- `secret-sprawl-fail`: secret-like material should raise `HLT-010`.
- `destructive-migration-fail`: destructive migration without safety proof should raise `HLT-021`.
- `authz-isolation-fail`: missing authorization isolation proof should raise `HLT-022`.
- `input-boundary-xss-fail`: unsafe rendering/input boundary should raise `HLT-023`.
- `overbroad-agency-fail`: overbroad agent/tool permissions should raise `HLT-012`.
- `rendered-ux-gap-fail`: user-facing UI without rendered proof should raise `HLT-013`.

Run:

```bash
cargo run -p jankurai -- conformance run \
  --fixtures conformance/fixtures \
  --expected conformance/expected \
  --out target/jankurai/conformance-results.json \
  --md target/jankurai/conformance-results.md \
  --tex paper/tex/generated/conformance_results_table.tex
```

The command emits schema-valid JSON, Markdown, and the generated paper table.
