# Jankurai Adoption

Adoption is staged to avoid breaking user projects:

```text
scan without writes -> adoption plan -> dry-run control plane -> observe CI -> accepted baseline -> ratchet
```

## Greenfield

Use this for a new repo that can start near the standard.

```bash
jankurai init . --profile rust-ts-postgres --dry-run --plan-json target/jankurai/init-plan.json
jankurai init . --profile rust-ts-postgres --yes
jankurai audit . --mode advisory --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
```

Completion: control-plane files exist, `doctor` passes for high severity, and
CI is observe-mode until the first baseline is accepted.

## Healthy Existing Repo

Use this when the repo already has tests, CI, and clear ownership.

```bash
jankurai adopt . --mode observe
jankurai init . --profile auto --dry-run --plan-json target/jankurai/init-plan.json
jankurai ci install . --github --mode observe --dry-run
```

Apply only the reviewed control-plane files. Existing `AGENTS.md`, `Justfile`,
docs, and workflows must be preserved or merge-marked.

## Legacy Or Far Repo

Use this when the detected stack is far from the target or proof lanes are weak.

```bash
jankurai adopt . --mode observe
jankurai migrate . --analyze --out target/jankurai/migration-report.json --md target/jankurai/migration-report.md
jankurai init . --profile migration-target --dry-run --plan-json target/jankurai/init-plan.json
```

Completion: inventory, boundary map, migration slices, equivalence proof, and
rollback notes exist. Do not enforce score 85 by default.

## Regulated Repo

Use `regulated-saas` only after the adoption plan confirms the repo can carry
the evidence burden.

```bash
jankurai adopt . --profile regulated-saas --mode advisory
jankurai init . --profile regulated-saas --dry-run --plan-json target/jankurai/init-plan.json
```

Completion: privacy, compliance, backup/restore, security evidence, and
exception-expiry paths are real evidence, not placeholders.

## Stop Conditions

Stop adoption if generated commands require this source workspace, a workflow
would overwrite existing user content, a proof lane is a noop, or CI enforces a
score gate before a baseline exists.
