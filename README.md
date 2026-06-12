# Jankurai

Public hub for the Jankurai split family.

Jankurai is an agent-native repository conformance standard and audit CLI for
auditable AI-assisted merge. This hub intentionally stays thin: it carries the
installer, GitHub Action, release notes, family manifest, lockfile, and local
fusion script. The auditor source lives in `jankurai-core`; reusable tools live
in the `jankurai-tools-*` family repos.

## Install

Install a released binary from this hub release surface:

```bash
curl -fsSL https://github.com/neverhuman/jankurai/releases/download/v1.7.0-split.0/jankurai-installer.sh \
  | JANKURAI_RELEASE_TAG=v1.7.0-split.0 bash
```

The installer verifies the GitHub release, artifact attestation, checksum, and
Sigstore bundle before installing.

## Fused Development

Public development from GitHub tags:

```bash
git clone https://github.com/neverhuman/jankurai
cd jankurai
./scripts/fuse.sh --source github --all
.fusion/dev.sh build
```

Internal Jeryu development:

```bash
./scripts/fuse.sh --source jeryu --all
.fusion/dev.sh build
```

Local split-container development from sibling repos:

```bash
./scripts/fuse.sh --source local --all
.fusion/dev.sh build
```

The `.fusion/` directory is generated and ignored. It is the only place local
path patches are written.

## Repository Family

The split family is declared in `repos.manifest.toml`. A release is pinned by
`family.lock`, which records each member repo, tag, and commit SHA consumed by
the fused release.

| Repo | Role |
| --- | --- |
| `jankurai` | Hub, installer, GitHub Action, release notes, manifest, lock, fusion script. |
| `jankurai-core` | Rust package and binary source for the `jankurai` auditor. |
| `jankurai-contracts` | Schemas, artifact contracts, generated type source, compatibility tests. |
| `jankurai-standard` | Standard docs, mission, public conformance policy, agent-native text. |
| `jankurai-conformance` | Fixtures, expected reports, acceptance corpus. |
| `jankurai-paper` | TeX paper, data, generated table inputs, paper CI lane. |
| `jankurai-tools-tui` | Tuiwright libraries, CLI, examples, docs. |
| `jankurai-tools-ux` | `@jankurai/ux-qa`, UX policies, Playwright and axe wrapper. |
| `jankurai-tools-guard` | Guard/save-gate runtime and standalone guard crate. |
| `jankurai-tools-proof` | Proofbind, proofmark, receipts, proof schemas. |
| `jankurai-tools-dedup` | Copy-code and dedup source snapshots for extraction hardening. |
| `jankurai-tools-analyzers` | Analyzer source snapshots for extraction hardening. |
| `jankurai-tools-fleet` | Fleet, score history, trend, and repair-task source snapshots. |
| `jankurai-deploy` | Release builds, signing, installer publishing, mirroring, split tooling. |

## GitHub Action

```yaml
- uses: neverhuman/jankurai@v1.7.0-split.0
  with:
    mode: advisory
    release-tag: v1.7.0-split.0
```

The action installs the released `jankurai` binary from hub releases and then
runs the requested audit mode against the caller repository.

## Local Validation

```bash
bash scripts/validate-family.sh
```

The validator checks required split metadata, Jeryu mirror config, lockfile
pins, missing lockfiles, branch dependencies, committed cross-repo path
dependencies, and action pinning posture.
