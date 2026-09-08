# Jankurai

[![CI](https://github.com/neverhuman/jankurai/actions/workflows/ci.yml/badge.svg)](https://github.com/neverhuman/jankurai/actions/workflows/ci.yml)

[Agent instructions](AGENTS.md) · [Architecture](docs/architecture.md)

GitHub is the primary home of Jankurai. This hub assembles 14 independently
editable component repositories under [neverhuman](https://github.com/neverhuman).
It owns the accepted family lock, combined checks, automatic updates, installer,
GitHub Action, and public releases.

## Quick start

Install Git, Rust (the toolchain is pinned in
`rust-toolchain.toml`), and Node.js 24. Then:

```sh
git clone https://github.com/neverhuman/jankurai.git workspace/jankurai
cd workspace/jankurai
bash scripts/family.sh setup
bash scripts/family.sh build
```

Setup clones missing components alongside the hub, verifies immutable tags against
`family.lock`, restores safe locked revisions, and installs Cargo/npm dependencies.
No Jeryu service, URL rewrite, pre-existing sibling repository, or dependency cache
is required. The generated `.fusion/components/` links point to the canonical
component checkouts; edit those component repositories directly.

| Shell command | Just recipe | Behavior |
| --- | --- | --- |
| `bash scripts/family.sh setup` | `just setup` | Bootstrap components and project dependencies at the accepted lock. |
| `bash scripts/family.sh pull` | `just pull` | Fetch successful component revisions, test a candidate in a disposable CI checkout, and update the two locks only after success. |
| `bash scripts/family.sh build` | `just build` | Build the auditor, Tuiwright CLI, and UX CLI; bootstrap missing components while preserving existing heads. |
| `bash scripts/family.sh check` | `just check` | Run component checks, combined Rust/UX tests, conformance, security, and audit evidence. |
| `bash scripts/family.sh status` | `just status` | Show branches, commits, dirty files, and differences from the accepted lock. |

Full checks also require Chromium, TeX/latexmk, nextest, and the security tools
installed explicitly in `.github/workflows/ci.yml` and `ops/ci/github-setup.sh`.
Normal Cargo builds use the committed aggregate `Cargo.lock` with `--locked`.
The legacy `scripts/fuse.sh --source github --all` and `.fusion/dev.sh` entrypoints
remain compatibility wrappers.

## Work preservation

Setup and pull reject dirty checkouts or changes that would discard ahead or
divergent commits. Obtain a stopped-head handoff before changing a busy checkout.
No command creates Git worktrees. Candidate integration uses an automatically
removed standalone CI checkout; failed candidates leave accepted locks unchanged.

## Releases and updates

[Release and installation details](docs/release.md) describe Linux x86-64 and
Apple Silicon macOS tarballs for `jankurai` and `tuiwright`, plus the built UX CLI
npm package. The first release after the GitHub migration is `v1.7.0` and is gated
on complete validation. The governed launcher and demo binary are not public assets.

[Automation](docs/github-automation.md) explains immutable CI tags, hourly lock
update PRs, exact-revision merge checks, and automation-token rotation.
All default branches require protected PR merges and `<repo>/required`.
Historical Jeryu branches, dependency tags, and the hub's legacy GitHub history
are preserved; public builds use GitHub exclusively.
