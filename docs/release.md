# Release process

This document is the release control surface for the jankurai hub. It covers the
version source, the changelog, the release automation, integrity and SBOM
evidence, and rollback. Launch gates require every section below to be backed by
a real artifact or command.

## Version source

The single source of truth for the hub version is the [`VERSION`](../VERSION)
file at the repository root. The release tag and the version recorded in
[`agent/split-member.toml`](../agent/split-member.toml) and
[`agent/standard-version.toml`](../agent/standard-version.toml) MUST match
`VERSION`. Tags follow the family pattern `jankurai-v<MAJOR.MINOR.PATCH>-split.<N>`
as described in [`SPLIT.md`](../SPLIT.md).

## Changelog

Every release records its user-visible changes in
[`CHANGELOG.md`](../CHANGELOG.md) under a heading that matches the new `VERSION`.
The `Unreleased` section is promoted to a dated version heading at tag time.

## Release automation

Releases are cut by CI and the family deploy repo, not by hand:

1. Bump [`VERSION`](../VERSION) and promote the `Unreleased` section of
   [`CHANGELOG.md`](../CHANGELOG.md).
2. Repin the family by updating [`family.lock`](../family.lock) so every member
   resolves to an immutable tag and commit SHA, never a branch.
3. Run the full local gate: `just check` (fast family validation, security
   scan, and self-audit). The same lanes run in CI via
   [`.github/workflows/ci.yml`](../.github/workflows/ci.yml), which delegates to
   `ops/ci/<lane>.sh` and uploads the `repo-score` artifacts.
4. Push the version commit and tag the release commit with
   `jankurai-v<version>-split.<N>`. The tag mirror in
   [`.jeryu/repo.toml`](../.jeryu/repo.toml) publishes the immutable tag to the
   public GitHub mirror.

Release builds depend on immutable tags, never branches.

## Integrity, provenance, and SBOM

- **Dependency integrity**: the hub ships no Cargo.toml or package.json. Its
  dependency surface is the family lock; `bash scripts/validate-family.sh`
  verifies every `family.lock` pin resolves to an immutable tag and commit and
  fails on branch dependencies, committed cross-repo path dependencies, or
  missing lockfiles.
- **SBOM**: the `repos.manifest.toml` + `family.lock` pair is the hub's bill of
  materials — it enumerates every member repo, its role, and the exact tag and
  commit consumed by a fused release. The deploy repo attaches this resolved set
  as the release SBOM.
- **Provenance**: the security job runs `gitleaks detect` for secret scanning and
  the family-lock review for supply-chain drift; the audit job publishes the
  `repo-score` artifacts that prove the release passed the jankurai gate. The
  released binary installer (`jankurai-installer.sh`) additionally verifies the
  GitHub release, artifact attestation, checksum, and Sigstore bundle before
  installing.
- **Action pinning**: every third-party GitHub Action in
  [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) is pinned to a
  40-character commit SHA, and `scripts/validate-family.sh` reports the action
  pinning posture so the supply chain of the pipeline itself is fixed.

## Launch gate

A release does not ship until the launch gate passes. The launch gate is the
artifact-backed checklist below; every item must be green before a tag is cut:

- **Security**: `gitleaks detect` finds no committed secrets and the family-lock
  review reports no supply-chain drift (`bash ops/ci/security.sh`).
- **Backups**: the distribution surface is recoverable — every released artifact
  is a backup-by-design immutable GitHub release asset, and the resolved
  `repos.manifest.toml` + `family.lock` set is retained so any release can be
  re-fused from its backup of pinned tags and commit SHAs.
- **Monitoring**: the jankurai audit job uploads `repo-score.{json,md}` on every
  push, providing continuous monitoring of the hub's conformance score; a
  regression below the minimum score fails CI and alerts maintainers.
- **Rollback**: the rollback procedure below is documented, tested against the
  last known-good tag, and required to be exercised before a risky release.
- **Abuse and rate limits**: the GitHub Action and installer are advisory by
  default and apply GitHub API rate limits; abuse of the release surface (forged
  artifacts) is blocked by the installer's attestation, checksum, and Sigstore
  verification before anything is installed.

## Rollback

If a release regresses:

1. Identify the last known-good tag (`jankurai-v<version>-split.<N>`).
2. Re-point consumers at that immutable tag; tags are never moved or deleted.
3. Open a revert commit that restores the previous `VERSION`, `CHANGELOG.md`,
   and `family.lock` pins, and add a `### Fixed` entry describing the rollback.
4. Re-run `just check` to confirm the rolled-back tree is green before
   re-publishing.

Because tags are immutable and `family.lock` pins every member to a commit SHA,
any prior release can be re-fused and rebuilt bit-for-bit from its tag.
