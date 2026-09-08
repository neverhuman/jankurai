# Install Jankurai 1.7.0

The [v1.7.0 release](https://github.com/neverhuman/jankurai/releases/tag/v1.7.0)
provides native Linux x86-64 and Apple Silicon macOS binaries.

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/neverhuman/jankurai/v1.7.0/jankurai-installer.sh | bash -s -- --tag v1.7.0
export PATH="$HOME/.local/bin:$PATH"
jankurai --version
```

The result is `jankurai 1.7.0`. The default directory is `~/.local/bin`; add the
PATH line to `~/.bashrc` or `~/.zshrc` if needed. Pass `--install-dir /your/bin`
to select a different writable directory. No sudo, Rust, Node.js, GitHub login,
or preinstalled verifier is needed for the auditor or Tuiwright. The platform
must provide Bash, curl, tar, a SHA-256 tool, and (on macOS) unzip.

Run the command again to upgrade or reinstall. It checks the staged binary and
replaces the installed file atomically. A download, verification, or version
failure preserves the existing binary. Remove it with `rm ~/.local/bin/jankurai`;
the installer keeps no permanent verification tools or background services.
Repository audit reports remain yours to retain or remove separately.

## Audit modes

Run this from the repository you want to inspect:

```sh
jankurai audit . --mode advisory --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md --repair-queue-jsonl target/jankurai/repair-queue.jsonl
```

Advisory mode emits findings for review. Ratchet mode additionally takes
`--baseline path/to/accepted-score.json` and rejects regressions. Release mode
also takes a baseline and applies release policy. Use `jankurai audit --help`
for the available policy and output options.

## Tuiwright

Add `--product tuiwright` to the version-pinned installer command, then run
`tuiwright --version` (expected: `tuiwright 1.7.0`). Remove it with
`rm ~/.local/bin/tuiwright`.

## UX package

The built `jankurai-ux-qa-1.7.0.tgz` is attached to the release. Browser auditing
requires Node.js 24, npm, Playwright 1.59.1, and Chromium. Before installing the
package, download its `.sha256`, `.sigstore.bundle`, and `.attestation.jsonl`
companions. Check the checksum and verify both signatures against
`neverhuman/jankurai/.github/workflows/release.yml@refs/tags/v1.7.0`, enforcing a
hosted runner and the release source commit. The exact commands used by release
validation are in [release-smoke.sh](../ops/ci/release-smoke.sh).

After verification:

```sh
npm install -g ./jankurai-ux-qa-1.7.0.tgz playwright@1.59.1
npx playwright@1.59.1 install chromium
jankurai-ux-qa --version
```

Expected: `jankurai-ux-qa 1.7.0`. Remove it with
`npm uninstall -g @jankurai/ux-qa`; remove Playwright separately if unused.

## Verification and source builds

The installer pins GitHub CLI 2.100.0, cosign 3.1.3, and jq 1.8.2 by version and
SHA-256 for each platform. It verifies the asset checksum, Sigstore identity,
archive inventory and file types, repository and lock provenance, and the local
GitHub attestation bundle. Attestation policy binds the source commit, tag,
release workflow, and GitHub-hosted runner. `--verify-only` also executes the
verified staged binary. Maintainers use `--assets-dir dist` to test signed staged
assets before publication; the same verification policy applies.

Contributor builds and family commands are documented in the
[README](../README.md#build-from-a-fresh-clone). Standard and schema versions
remain independent of the public CLI release number.
