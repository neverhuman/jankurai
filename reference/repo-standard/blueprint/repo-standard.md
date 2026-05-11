# Repository Standard

Last reviewed: 2026-03-31

## Reference Layout

```text
repo/
  Cargo.toml
  AGENTS.md
  CLAUDE.md
  .config/nextest.toml
  .github/
    copilot-instructions.md
    workflows/agent-proof.yml
    instructions/

  crates/
    cargo-vrc/
    cargo-aer/
    arc-bench/

  tools/
    paper-sync/

  challenges/
    cases/
    suites/

  os/
    null/
    v1/
    v2/
    v3/
    v90/

  docs/
    blueprint/
    research/
    aer-records/

  benchmark/
    runner/
    analysis/
    results/
    reports/
  labs/
  paper/
  tools/
    paper-sync/
```

## Standard Rules

1. The workspace root sets `default-members` so the inner loop stays cheap.
2. Each meaningful crate is an ARC unless an AER says otherwise.
3. Root guidance stays short and navigational.
4. `workspace.metadata.agent` carries workspace-wide validation and CI policy.
5. `package.metadata.agent` carries ARC-local purpose, ownership, invariants, commands, and exception refs.
6. `agent-map.json`, `test-map.json`, `vrc-plan.json`, and `aer-findings.json` are generated control-plane artifacts, not hand-maintained prose.
7. Benchmarks and exception fixtures are review inputs, not optional demos.

## Workspace Metadata Contract

`workspace.metadata.agent` should define:

- `validation_order`
- `slow_members`
- `shared_contracts`
- `ci_profiles`
- `instruction_roots`

## Package Metadata Contract

`package.metadata.agent` should define:

- `purpose`
- `owned_paths`
- `entrypoints`
- `invariants`
- `local_validate`
- `boundary_validate`
- `public_api`
- `risk`
- `consumers`
- `exceptions`

## Learned Bundle Contract

Learned controller bundles should follow the same host-side lifecycle everywhere:

1. `collect` a clean Docker runset into `benchmark/container-runs/`.
2. `promote` that runset into a versioned host bundle under `benchmark/datasets/<bundle>/`.
3. `train` the controller from the promoted bundle and write the model artifacts into the same bundle.
4. `test` the exported bundle with replay and holdout readers that never consume a mutable Docker workspace.
5. `seal` the bundle manifest once the artifacts are written, so stale or mixed bundles fail fast.

The canonical bundle manifest is `bundle_manifest.json` with `pipeline_manifest.json` as a compatibility alias.
Each bundle must record at minimum `source_runs_root`, `source_runset_id`, `trained_from_dataset_dir`,
`bundle_state`, `bundle_root`, and the exported artifact paths.

## Validation Order

1. `cargo check -p <arc>`
2. `cargo test -p <arc>`
3. `cargo test -p <arc> --doc`
4. reverse-dependency tests when a public or manifest boundary moved
5. contract tests when a shared protocol or schema moved
6. smoke tests when an end-user flow moved
7. full end-to-end only when the outer boundary changed

## Complexity Budgets

These are house defaults, not ecosystem law:

- function target: 10 to 40 logical lines
- function review trigger: 60+
- function AER trigger: 80+
- file target: 100 to 300 logical lines
- file review trigger: 500+
- file AER trigger: 800+
- crate split trigger: repeated multi-domain churn or repeated broad validation expansion

## Optimal Code Doctrine

The repo does not optimize for low line count alone.

It optimizes for:

- minimum semantic surface,
- minimum dynamic work,
- minimum ambient authority,
- memory-aware and thread-aware design,
- stable proof obligations,
- explicit extensibility points.

In practice, that means:

- fewer runtime layers when they do not buy clear flexibility,
- fewer shared mutable structures when message passing or ownership transfer is cheaper and safer,
- fewer “utility” crates and more domain-owned code,
- more compile-time proof and fewer implicit contracts.
