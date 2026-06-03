# Generated Zones

Generated artifacts are governed by `agent/generated-zones.toml`. Treat that
manifest as the source of truth before editing reports, paper tables, generated
contracts, figures, lockfiles, and accepted score baselines.

## Rules

- Do not hand-edit generated artifacts outside their declared source command.
- Review the source contract, data file, or generator before reviewing generated
  output diffs.
- Generated files must identify their source, command, and write policy in
  `agent/generated-zones.toml`.
- Regeneration commands must be local, deterministic, and routed through
  `agent/test-map.json`.
- Generated score outputs under `.jankurai/` and `target/jankurai/` are local
  evidence, not accepted public baselines unless copied through the declared
  baseline command.

## Current Declared Surfaces

- Paper figures and PDFs are regenerated from their source tables or TeX source.
- Generated paper tables are regenerated from `agent/vibe-coverage.toml`,
  public repository score data, or conformance fixtures.
- `agent/baselines/main.repo-score.json` is accepted only from a clean standard
  audit and is otherwise read-only.
- `package-lock.json` is owned by the package manager and regenerated from npm
  manifests.

## Review Checklist

1. Confirm the changed output path appears in `agent/generated-zones.toml`.
2. Run the declared command or explain why the output was not regenerated.
3. Run the mapped proof command from `agent/test-map.json`.
4. Verify no generated output changed without a matching source, manifest, or
   generator change.
5. Keep volatile proof receipts under `target/jankurai/`.
