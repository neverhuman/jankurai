# Release process

The hub publishes releases from version tags. `VERSION`, the auditor version,
the Tuiwright version, and the UX npm package version must agree. The first post-migration release is
`v1.7.0`; publish its tag only after every component default branch, complete hub
integration, and both release-platform checks pass.

The release workflow independently checks the locked family, builds Linux x86-64
and Apple Silicon macOS products, runs their version commands, signs every asset
with Sigstore, generates GitHub attestations, verifies the complete inventory,
exports downloadable attestation bundles, installs and runs the staged native
products, and publishes a GitHub Release only after both platforms pass. No self-hosted runner or Apple signing account
is needed for the selected tarball distribution.

Public assets include:

- `jankurai-<version>-<target>.tar.gz`
- `tuiwright-<version>-<target>.tar.gz`
- `jankurai-ux-qa-<version>.tgz`, the built npm CLI package
- `family.lock`, `Cargo.lock`, per-platform provenance, and the installer
- SHA-256 checksums, Sigstore bundles, and GitHub artifact attestations

The governed launcher and `tuiwright-demo` remain internal build/test products.
Tarballs contain only their selected executable, license, locks, and provenance.

## Installation

Download `jankurai-installer.sh` from the desired hub release, then:

```sh
bash jankurai-installer.sh --tag v1.7.0
bash jankurai-installer.sh --tag v1.7.0 --product tuiwright
```

The installer requires the platform shell, curl, archive tools, and SHA-256 tooling.
It downloads temporary GitHub CLI, cosign, and jq binaries at exact versions and
checks their embedded SHA-256 pins. No GitHub login is required. It verifies the checksum, the Sigstore workflow identity and version
tag, the GitHub attestation identity and hosted runner, the release commit, the
archive inventory, and the embedded lock digests before installing. Both platforms
install to `~/.local/bin` by default. `--verify-only` performs all verification
and runs the staged binary without installing. The GitHub Action uses the same installer and defaults to
`v1.7.0`.

To install the UX CLI, verify its downloaded checksum, Sigstore bundle, and GitHub
attestation against the same release workflow identity before running
`npm install -g ./jankurai-ux-qa-1.7.0.tgz`. Install the declared Playwright peer
dependency and Chromium as required by the package.

Releases and dependency tags are immutable. Roll back by explicitly selecting an
earlier verified release or opening a protected PR that restores a previously
accepted family lock. Preserve the existing GitHub release history and legacy
refs during all migrations.

## Release gate and recovery evidence

Release readiness requires successful component aggregates, the hub integration
artifact, and both platform jobs for the candidate. The release workflow repeats
integration before packaging; failed or missing evidence blocks publication.
Its security lane records an SBOM and checks secrets, dependencies, and workflow
permissions. Installer tests exercise checksum and provenance tampering; the
release smoke test must additionally verify actual downloaded signed assets on
both platforms before declaring the release usable.

Backup custody consists of preserved Git refs, immutable dependency tags, prior
GitHub Releases, and a verified Git bundle before hub history migration. Record
the bundle digest and `git bundle verify` result in the migration evidence.
Rollback selects a prior verified release or restores accepted locks through a
protected PR; never move an existing release tag.

Monitoring uses required-check failures, uploaded audit findings, release job
status, and the hourly token-expiration job. Maintainers investigate failed
checks before another publication attempt. Abuse controls include read-only
build tokens, a hub-only publisher secret, bounded lock artifacts, exact-SHA
checks, protected PR merges, immutable tags, and fixed installer asset inventory.

The CI budget is bounded by workflow job timeouts and concurrency groups; the
updater runs once per hour and GitHub API rate limits bound its request quota.
Failure, token expiry, or exhausted quota is a stop condition. Disabling the
`family-update` workflow is the maintainer kill switch while investigating
unexpected workload. No workflow retries indefinitely or purchases extra quota.
