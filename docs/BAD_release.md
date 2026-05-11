# Release Bad Behavior

Release work is where repository evidence becomes user-visible. A professional
release is an immutable, traceable, tested, documented, verifiable, and
recoverable snapshot of source, artifacts, approvals, and operating context.

This document distills the tracked bad-release corpus into the release policy
Jankurai audits. It is intentionally stack-neutral: a repository can use
SemVer, CalVer, package versions, container tags, or deployment IDs, but it
must make the release identity and evidence machine-reviewable.

## Minimum Release Structure

Coding projects that publish packages, binaries, containers, hosted apps, or
other shipped artifacts need a release control surface:

| Surface | Required content |
| --- | --- |
| Version source | `VERSION`, package manifest version, `Cargo.toml` version, or equivalent canonical version file. |
| Changelog | Human-readable change history with release dates, breaking changes, fixes, and migration notes. |
| Release process doc | How to cut, verify, publish, monitor, and roll back a release. |
| Release automation or command policy | CI workflow, script, `just` target, or documented command set that prevents ad hoc laptop publishing. |
| Integrity policy | Checksums, signatures, SBOM, provenance, attestation, or equivalent artifact integrity evidence. |
| Rollback guidance | Previous known-good version, roll-forward/rollback path, monitoring trigger, and owner. |

Missing structure routes to `HLT-025-RELEASE-READINESS-GAP`. Actively unsafe
release commands route to `HLT-037-RELEASE-BAD-BEHAVIOR`.

## Gold Standard

A release should answer these questions without detective work:

| Question | Evidence |
| --- | --- |
| What code shipped? | Exact commit SHA and immutable tag or release ID. |
| What changed? | Changelog, release notes, linked PRs/issues, and breaking-change notes. |
| Who approved it? | Required reviews, CODEOWNERS where applicable, release approver, and CI identity. |
| What ran? | CI run, required checks, security lane, tests, and post-release verification. |
| What artifact shipped? | Package/image/binary identifier plus digest, checksum, signature, SBOM, provenance, or attestation. |
| How is it recovered? | Rollback or roll-forward plan, previous known-good version, monitoring threshold, and incident owner. |

## Inexcusable Behavior

Treat these as release-blocking until corrected or explicitly waived with
owner, expiry, residual risk, and compensating proof:

| Behavior | Why it blocks |
| --- | --- |
| Retagging, force-pushing, deleting, or moving release tags | Destroys traceability from version to commit. |
| Overwriting or deleting release assets | Makes checksums, approvals, and user downloads untrustworthy. |
| Publishing from a dirty worktree or developer laptop | Severs the link between reviewed source, CI, and artifact. |
| Skipping tests, package verification, security scans, or lifecycle checks | Converts release proof into ceremony. |
| Publishing only mutable `latest` tags | Prevents users and auditors from knowing what shipped. |
| Missing changelog, migration note, or breaking-change rationale | Hides user impact and compatibility risk. |
| Shipping without checksums, SBOM, provenance, signature, or attestation where artifacts are distributed | Leaves consumers unable to verify artifact identity. |
| Broad tokens, personal access tokens, or secrets in release workflows | Expands blast radius and weakens audit identity. |
| Running untrusted PR code in privileged release jobs | Turns validation into a privilege bridge. |
| Packaging `.env`, `.npmrc`, `.pypirc`, `.ssh`, private keys, logs, dumps, or local config | Leaks credentials or private operating context. |
| No rollback or roll-forward path | Makes a known-bad release an incident amplifier. |

## Jankurai Enforcement

`HLT-025-RELEASE-READINESS-GAP` checks that release-capable projects have the
expected release structure and launch-gate evidence. The current minimum
structure is a version source, changelog, release process doc, automation or
command policy, integrity/provenance policy, and rollback guidance.

`HLT-037-RELEASE-BAD-BEHAVIOR` is detector-backed and looks for high-confidence
release hazards in release scripts, publish scripts, package scripts, and CI
release workflows:

| Detector family | Examples |
| --- | --- |
| Mutable tags | `git tag -f`, forced tag pushes, tag deletion. |
| Mutable assets | `gh release upload --clobber`, release deletion. |
| Skipped verification | `SKIP_TESTS=1`, `cargo publish --no-verify`, `npm publish --ignore-scripts`. |
| Mutable latest-only artifacts | `docker push ...:latest`, `git tag latest`. |
| Secret-bearing artifacts | Archives or release uploads that include `.env`, `.npmrc`, `.pypirc`, `.ssh`, private keys, or secret-named paths. |
| Unverified release creation | `gh release create` without tag verification evidence. |
| Missing integrity evidence | Publish commands without checksum, SBOM, provenance, signature, attestation, or Jankurai witness evidence. |
| Untrusted privileged publishing | Release/publish commands in `pull_request_target` or privilege-bridged workflows with write tokens or secrets. |

The detector skips docs, paper sources, tips, reference material, generated
outputs, tests, fixtures, and examples. The goal is not to punish research
material for describing bad behavior; it is to stop dangerous release behavior
from becoming executable policy.

## Repair Playbook

1. Stop mutating published tags or assets. Publish a new version.
2. Add or update `CHANGELOG.md` and release notes with breaking changes,
   migration steps, security notes, and known issues.
3. Build artifacts in CI from a clean, reviewed commit and immutable tag.
4. Attach checksums and supply-chain evidence such as SBOM, provenance,
   signature, or attestation.
5. Restrict release workflows to protected branches or protected tags.
6. Use short-lived, least-privilege publishing credentials or trusted
   publishing/OIDC where available.
7. Scan artifacts for secrets before publication.
8. Document rollback or roll-forward commands and the monitoring signal that
   triggers them.
9. Run `cargo test -p jankurai --test language_bad_behavior` and then `just score`.
