#!/usr/bin/env bash
# Release artifact signing: produce a Sigstore bundle for a release asset.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

artifact="${1:?usage: $0 <artifact-path>}"
if [[ ! -f "${artifact}" ]]; then
  fail "expected release artifact: ${artifact}"
fi
if ! command -v cosign >/dev/null 2>&1; then
  fail "cosign is required to sign release artifacts"
fi

bundle="${artifact}.sigstore.bundle"

step "cosign sign-blob ${artifact}"
cosign sign-blob --yes --bundle "${bundle}" "${artifact}"
assert_nonempty "${bundle}"
