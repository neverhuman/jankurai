# Branch protection

Every default branch requires a pull request and the aggregate check named
`<repo>/required`. Checks are strict, administrators cannot bypass them, force
pushes and branch deletion are forbidden, and conversation resolution is required.
Linear history is maintained through squash or rebase merges.

The hub aggregate includes fast validation, complete locked-family integration,
and both Linux x86-64 and Apple Silicon macOS release-build jobs. A skipped,
cancelled, or failed dependency makes the aggregate fail.

The required approval count is zero so the narrowly scoped lock updater can
complete its authorized automatic merge cycle. Protected checks and exact-head
validation still apply; the updater may change only `family.lock` and `Cargo.lock`.
Other changes follow the normal maintainer PR review process.

All existing and new tags are protected against update and deletion. Successful
component default-branch CI creates a new immutable `ci-<SHA>` tag. Public release
tags trigger the full hub validation, signing, attestation, and publication workflow.

`scripts/provision-family.sh` implements the manifest-selected protection settings.
