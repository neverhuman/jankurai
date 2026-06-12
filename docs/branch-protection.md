# Branch protection

GitHub branch protection is the enforcement layer for the Jankurai standard.
The rules below describe the policy a project should apply on `main`. Settings
must be configured by a repository administrator in
**Settings → Branches → Branch protection rules**; this document is the
authoritative reference.

## Required status checks

`main` should require all of the following checks to pass before merge:

- `jankurai / audit` — full repo audit, ratchet enforcement, security lane.
- `jankurai / test (ubuntu-latest)` — fmt, clippy, workspace tests on Linux.
- `jankurai / test (macos-latest)` — fmt, clippy, workspace tests on macOS.
- `jankurai / coverage (llvm-cov)` — Rust coverage artifact generation.

Mark the checks **Strict**: PR branches must be up-to-date with `main` before
merge.

## Review requirements

- Require at least **one** approving review.
- Require review from **Code Owners** (see `/CODEOWNERS`).
- Dismiss stale approvals when new commits are pushed.

## Commit and history hygiene

- **Require signed commits.** Every commit on `main` must carry a verified
  signature so the merge witness records provable authorship.
- **Require linear history.** Disallow merge commits; use squash- or
  rebase-merge only.
- **Restrict who can push directly.** No one should push to `main`; merge only
  through reviewed pull requests.

## Bypasses

- Do **not** allow administrators to bypass these rules. The merge witness
  contract treats `main` as gated even for repository admins.
- The `release` workflow runs on tag push; tag protection rules should require
  the `release / publish release` job to succeed before creating new tag
  artifacts. See `.github/workflows/release.yml`.

## Tag protection

Add a tag protection rule for `v*.*.*` requiring the audit-gate job from
`.github/workflows/release.yml` to pass. This guarantees that any release tag
ships with a clean audit, signed commits, and full test coverage.

## Rolling baselines

When the team accepts a new baseline score, the corresponding
`agent/baselines/main.repo-score.json` change must be reviewed and approved
under the same code-owner rules. Ratchet enforcement reads this file on every
audit run.
