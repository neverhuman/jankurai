# jankurai family — release & provisioning runbook

Status as of the split-green-audit work: **15/15 repos pass their own jankurai
audit** (0 hard / 0 caps), core is de-monolithed into real crates, the binary
builds two ways with byte-identical audit output, and `validate-family.sh` is
green. This runbook is the remaining path to a public release + safely deleting
the legacy `~/jankurai` monorepo.

## The 15 repos
Hub `jankurai` (+ installer/action/manifest/lock/fuse) and 14 members:
`jankurai-core` (the binary) → depends on the engine crates
`jankurai-tools-kernel` (shared audit substrate, ~18k LoC) ← `-dedup`, `-fleet`,
`-analyzers` (real crates extracted from core; depend only on the kernel),
plus `-guard`, `-proof`, `-tui`, `-ux`, and the data/doc repos
`-contracts`, `-standard`, `-conformance`, `-paper`, and `jankurai-deploy`.

## How the binary builds (clear & reproducible)
- **Contributor / local:** `scripts/fuse.sh --source local --all && .fusion/dev.sh build`
  — clones every repo at the `family.lock`-pinned commit, fuses one cargo
  workspace with `[patch]` redirects, builds `-p jankurai`. Verified: produces a
  binary whose audit output matches the locked oracle 10/10.
- **From published tags (CI / release):** `jankurai-core`'s committed
  `crates/jankurai/Cargo.toml` git-depends on the tool crates at their tags; once
  the repos are on GitHub, `cargo build --release -p jankurai` in core resolves
  them with no patch. (Locally this is emulated by `git config --global
  url.file:///…/jankurai-tools-<r>.insteadof https://github.com/neverhuman/jankurai-tools-<r>.git`.)
- Committed manifests carry **only git-deps** (no path-deps) — enforced by
  `validate-family.sh`. Local path patches live only in the uncommitted
  `.fusion/` workspace or an uncommitted dev `[patch]`.

## BEFORE the first release — decide the version (BLOCKER)
`ops/ci/release-build.sh` asserts `RELEASE_TAG (minus v) == VERSION`. Today the
`VERSION` files disagree with the release tag:
- hub/core/deploy `VERSION` = `1.6.10`, kernel `VERSION` = `1.7.0`,
  core crate version = `1.6.11`, manifest `release` / tag = `1.7.0-split.0`.
**Action:** pick the canonical version (recommended `1.7.0-split.0` to match the
tag), set every repo's `VERSION` to it, commit, move tags, regenerate
`family.lock`. (The cargo crate `version =` is independent and may stay semver.)

## Release pipeline (consolidate onto the hub)
The installer + `action.yml` verify the cosign identity at
`github.com/neverhuman/jankurai/.github/workflows/release.yml@<tag>`, so the
release MUST run on the **hub**. The proven build/sign/publish scripts live in
`jankurai-deploy/ops/ci/release-{audit-gate,build,macos-sign,publish,sign-blob}.sh`.
Port them to the hub and have `release.yml`:
1. `bash scripts/fuse.sh --source github --all` (assemble the workspace from tags)
2. build+sign in the fused workspace (`CI_ROOT=.fusion`, Linux tar.gz + macOS pkg,
   cosign + attestation)
3. `gh release create` on `neverhuman/jankurai`, attaching `jankurai-installer.sh`.
NOTE: signing/notarization/publish is only testable on GitHub with the Apple +
Sigstore secrets configured.

## Provision GitHub + Jeryu (you run this)
With `gh` authenticated to `neverhuman` and the Jeryu remote reachable:
```
bash scripts/provision-family.sh --dry-run            # review every action
bash scripts/provision-family.sh --visibility public  # create+push 15 repos to
# GitHub AND Jeryu at parity (dependency-ordered: kernel/guard/proof → core →
# rest), verify parity, then push the release tag to trigger the hub release.
```
Then `validate-family.sh` should be run in CI with `ripgrep` installed (this
sandbox lacks it, so the workflow-action-SHA-pin check was skipped locally).

## Safe to delete ~/jankurai when
- All 15 pushed to GitHub + Jeryu at parity; tags + `family.lock` match.
- The first hub release is green and the installer verifies + installs the binary.
- `fuse --source github --all && .fusion/dev.sh build` reproduces the binary from
  the public tags. (Until then keep `~/jankurai` as the rollback source.)
