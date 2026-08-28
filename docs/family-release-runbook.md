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

## Historical release pipeline — transition state

The checked-in GitHub release workflow records the prior signed-release design,
but GitHub is no longer a repository provisioning, tag, or publication route.
The fail-closed `scripts/provision-family.sh` cannot trigger that workflow. A
replacement artifact-distribution design is outside this Git-authority change;
do not infer or simulate it from the retained historical workflow.

## Forge provisioning

The historical GitHub-plus-Jeryu provisioning script is deliberately disabled;
it cannot create repositories, move tags, or push to GitHub. The tracked family
manifest is the sole inventory, and its root projection is generated with
`scripts/project-family-manifest.sh`. While `authority_forge` remains
`local_transition`, releases continue through the protected local lifecycle.
The final hosted flip is a separate protected manifest change after ref/LFS
parity, private visibility, branch protection, accounts, and onboarding pass.
No release step may infer that cutover from the presence of hosted URLs alone.

## Legacy source retirement gate

Do not remove any legacy source merely because repository provisioning changed.
Retirement remains blocked until all of the following are proven:

- All 15 authoritative refs and objects exist at the manifest-selected forge;
  tags + `family.lock` match.
- A replacement artifact-distribution path is reviewed and its installer
  verifies and installs the binary.
- After the protected dependency-URL migration, a fresh cache reproduces the
  family from the private authority without GitHub or source-tree fallback.
