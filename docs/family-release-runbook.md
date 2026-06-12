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

## Version — DONE
Every repo's `VERSION` is reconciled to `1.7.0-split.0` (matches the release tag +
manifest `release`); `ops/ci/release-build.sh`'s `RELEASE_TAG == VERSION` gate
passes. Tags moved + `family.lock` re-pinned; 15/15 still audit-green, oracle 10/10.
(The cargo crate `version =` fields are independent semver and were left as-is.)

## Release pipeline — DONE (lives on the hub)
`jankurai/.github/workflows/release.yml` (+ `ops/ci/release-*.sh`) now owns the
signed release, on the hub so the installer's cosign identity
(`github.com/neverhuman/jankurai/.github/workflows/release.yml@<tag>`) verifies.
On a `v*.*.*` tag it runs: family-gate (`validate-family.sh`) -> build
(`scripts/fuse.sh --source github --all` then `BUILD_DIR=.fusion bash
ops/ci/release-build.sh`, Linux tar.gz + macOS pkg, cosign + attestation) ->
publish (`gh release create` on `neverhuman/jankurai` with `jankurai-installer.sh`).
Verified locally: yaml/bash valid, VERSION==tag gate passes, hub stays audit-green,
the fuse build produces the binary. NOT testable offline: signing/notarization/
publish need the Apple + Sigstore secrets on GitHub.
(`jankurai-deploy/.github/workflows/release.yml` is superseded/inert — it triggers
on `v*.*.*`, which only the hub ever receives; safe to delete.)

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
