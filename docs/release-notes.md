Jankurai 1.7.0 makes GitHub the primary home for the independently editable component family.

The hub provides setup, pull, build, check, and status commands, locked combined builds,
protected component updates, and verified release downloads.

Public products are the Jankurai auditor, Tuiwright CLI, and the UX QA npm CLI package.
Linux x86-64 and Apple Silicon macOS tarballs include the family lock, Cargo lock,
and build provenance. Each release asset has a SHA-256 checksum and Sigstore bundle;
Downloadable GitHub attestation bundles bind the downloads to the release workflow,
tag, hosted runner, and source commit. The installer bootstraps temporary verifiers
with embedded version and SHA-256 pins, requires no GitHub login or compiler, and
runs the staged binary before atomically replacing an installed version.

All public CLI products report 1.7.0. Standard and schema versioning is independent.
