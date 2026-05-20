# Installing jankurai

Jankurai adoption is no-write first, advisory by default, and ratcheted only
after a baseline exists. The preferred install path is the release-shipped
installer, which verifies the immutable tag, GitHub artifact attestation,
sha256 checksum, and Sigstore bundle before it installs the binary:

Prerequisites for the installer path: `curl`, `gh`, `cosign`, and `sudo`
permission for the macOS pkg installer if you are on macOS.

```bash
curl -fsSL https://github.com/neverhuman/jankurai/releases/download/v1.5.1/jankurai-installer.sh \
  | JANKURAI_RELEASE_TAG=v1.5.1 bash
```

If you need the fallback source install, clone the repo and build from the
workspace:

```bash
git clone https://github.com/neverhuman/jankurai.git
cd jankurai
cargo install --path crates/jankurai --locked
jankurai version
jankurai versions
```

Human terminal output uses color and progress bars when attached to a terminal.
For demos or logs, force rich output with `JANKURAI_COLOR=always` and
`JANKURAI_PROGRESS=always`.

`jankurai version` prints the installed CLI/version/source diagnostics and the
recommended upgrade command. The installer path prefers the notarized macOS
`pkg` or the Linux tarball produced by release CI; source install remains the
fallback for air-gapped or tool-minimal environments.

## Optional: pre-commit hooks

Install [pre-commit](https://pre-commit.com/) to mirror the CI gates locally:

```bash
pipx install pre-commit   # or: pip install --user pre-commit
pre-commit install
```

The repository's `.pre-commit-config.yaml` runs `cargo fmt --check`,
`cargo clippy -D warnings`, `gitleaks`, and verifies that the README test-surface
chart is in sync with the source tree. Skip a single commit with
`SKIP=cargo-clippy git commit ...` when you intentionally want to bypass a hook.

`jankurai versions` checks the source checkout against `VERSION`,
`crates/jankurai/Cargo.toml`, `packages/ux-qa/package.json`,
`agent/standard-version.toml`, `docs/agent-native-standard.md`,
`agent/JANKURAI_STANDARD.md`, and `paper/jankurai.md`.

For any external repo, start with artifacts under `target/jankurai/`:

```bash
jankurai audit /path/to/repo --mode advisory \
  --json /path/to/repo/target/jankurai/repo-score.json \
  --md /path/to/repo/target/jankurai/repo-score.md
jankurai adopt /path/to/repo --mode observe \
  --out /path/to/repo/target/jankurai/adoption-plan.json \
  --md /path/to/repo/target/jankurai/adoption-plan.md
```

Use `jankurai update --check` for a read-only upgrade plan. `jankurai upgrade`
is the write-capable refresh path, and `jankurai upgrade --score` runs the
follow-on scoring lane after the install refresh. `jankurai score` is the main
scoring command; with no subcommand it runs the audit lane, and `diff` and
`trend` remain available.

## Profiles

Bundled init profiles are defined as JSON validated against `schemas/init-profile.schema.json`.

- **`rust-ts-postgres`** (aliases: `rust-ts-vite-react-postgres`, `rust-ts-vite-react-postgres-bounded-python`) is the default full scaffold: agent constitution, IDE adapters, `contracts/` and `db/` README slots, `docs/architecture/` and `docs/decisions/` stubs, and `tools/security-lane.sh` stub. The historical bounded-Python alias does not authorize Python code; agents may add Python only for rare dated advanced-ML/data exceptions under `python/ai-service`.
- **`rust-api`**, **`react-web`**, **`b2b-saas`**, **`ai-product`**, **`regulated-saas`**, **`migration-target`** ship as bundled manifests under `crates/jankurai/templates/profiles/`.
- **`--profile-file path/to/profile.json`** loads a repo-local or shared manifest (same schema). Bundled `--profile` is not used to resolve the manifest when this flag is set. Plan JSON uses the manifest **`id`** as **`profile`**.
- Unknown bundled `--profile` values fail fast with an error listing supported IDs.

The canonical default manifest is `crates/jankurai/templates/profiles/rust-ts-postgres.json`. Planned file actions in dry-run / plan JSON are exactly the paths in `generatedPaths` (sorted); each path must have a matching entry in `crates/jankurai/src/init/templates.rs`.

Dry-run first:

```bash
jankurai init /path/to/repo --profile rust-ts-vite-react-postgres \
  --ide all --mode advisory --dry-run \
  --plan-json /path/to/repo/target/jankurai/init-plan.json
```

Apply when the plan looks right:

```bash
jankurai init /path/to/repo --profile rust-ts-vite-react-postgres \
  --ide all --mode advisory --yes
jankurai doctor /path/to/repo --fail-on high
jankurai audit /path/to/repo --mode advisory \
  --json /path/to/repo/target/jankurai/repo-score.json \
  --md /path/to/repo/target/jankurai/repo-score.md
jankurai ci install /path/to/repo --github --mode observe --dry-run
jankurai ci install /path/to/repo --github --mode observe
jankurai agent verify /path/to/repo
```

After `init --yes`, start Codex, Cursor, Claude, Copilot, or another agent in
the same repository root and say: `Read AGENTS.md, follow the jankurai standard,
then run the proof lane for my change.`

`init --yes` creates missing paths from the profile and uses the profile manifest's optional `mergePolicy` for existing `generatedPaths`. Bundled profiles explicitly declare their mergeable paths. A custom `--profile-file` without `mergePolicy` keeps the legacy suffix-based behavior for compatibility.

Allowed `mergePolicy` actions:

- **`merge-json`**: additive object/array merge.
- **`merge-toml`**: additive table/array merge.
- **`merge-lines`**: append template lines that are not already present.
- **`merge-marker`**: append an HTML merge marker for manual review.
- **`keep-existing`**: leave existing user-owned content unchanged.

Run **`jankurai init ... --dry-run`** (or **`--plan-json`**) first; the printed plan lists the action for each `generatedPaths` entry.

For agent repair work, use the narrow packet commands:

```bash
jankurai context-pack --task "repair agent context routing" --out target/jankurai/context-pack.json
jankurai repair-plan --from .jankurai/repo-score.json --out target/jankurai/repair-plan.json
```

## Greenfield Sequence

Use `rust-ts-postgres` unless the product is clearly narrower:

```bash
mkdir my-product
jankurai init my-product --profile rust-ts-postgres --dry-run \
  --plan-json my-product/target/jankurai/init-plan.json
jankurai init my-product --profile rust-ts-postgres --yes
jankurai audit my-product --mode advisory \
  --json my-product/target/jankurai/repo-score.json \
  --md my-product/target/jankurai/repo-score.md
```

## Brownfield Sequence

For an existing repo, do not install gates first:

```bash
jankurai adopt . --mode observe
jankurai migrate . --analyze --out target/jankurai/migration-report.json \
  --md target/jankurai/migration-report.md
jankurai init . --profile migration-target --dry-run \
  --plan-json target/jankurai/init-plan.json
jankurai ci install . --github --mode observe --dry-run
```

Use `--profile rust-api`, `--profile react-web`, or `--profile rust-ts-postgres`
only when the adoption plan recommends that profile. Use `migration-target`
when the repo is far from the standard; that route produces containment and
slice-planning docs without claiming full compliance.

## Ratchet Sequence

After the team accepts a baseline score, preserve it and then install ratchet
mode:

```bash
cp target/jankurai/repo-score.json target/jankurai/baseline-score.json
jankurai ci install . --github --mode ratchet \
  --baseline target/jankurai/baseline-score.json --min-score 85
```
