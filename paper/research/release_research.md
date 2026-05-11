# Release Research Notes

Source group: tracked bad-release corpus and existing release-plan material.
Purpose: distill release best practices and inexcusable release behavior into
Jankurai audit, scoring, steering, and paper language.

## Research Weighting

The source group repeatedly converges on a standards-grade release definition:
a release is an immutable, auditable, reproducible or at least verifiable
snapshot of source, build inputs, tests, approvals, artifacts, version,
changelog, deployment record, and rollback path.

The strongest source themes align with Git release tagging, SemVer-style
immutability, SLSA-style provenance, SBOM/checksum/signature evidence,
protected branches/tags, CI/CD trust separation, human-readable changelogs,
and DORA/SRE-style recoverability.

## Control Themes

| Theme | Jankurai consequence |
| --- | --- |
| Release identity | Require exact version, tag/release ID, commit SHA, and artifact digest. |
| Immutability | Reject retagging, forced tag pushes, deleted release refs, and overwritten release assets. |
| Verification | Require tests, security checks, package verification, and release receipts before publish. |
| Integrity | Require checksum, SBOM, provenance, signature, attestation, or equivalent evidence for distributed artifacts. |
| Governance | Protect release branches/tags, use least-privilege release credentials, and separate untrusted validation from privileged publishing. |
| User communication | Require changelog, release notes, breaking-change notes, migration/deprecation notes, and known issues. |
| Recovery | Require rollback/roll-forward path, previous known-good version, monitoring trigger, and owner. |

## Detector Implications

`HLT-025-RELEASE-READINESS-GAP` should fail release-capable code projects when
the expected release structure is absent. Minimum structure: version source,
changelog, release process doc, release automation or command policy,
checksum/provenance/SBOM policy, and rollback guidance.

`HLT-037-RELEASE-BAD-BEHAVIOR` should fail executable release policy that:

- mutates tags or release refs;
- overwrites or deletes published release assets;
- skips tests, package verification, lifecycle checks, or security proof;
- publishes mutable latest-only artifacts as the primary release identity;
- packages local secrets or private config;
- creates GitHub releases without tag verification evidence;
- publishes without checksum/SBOM/provenance/signature/attestation evidence;
- runs privileged release publishing from untrusted PR or privilege-bridged workflows.

The detector must skip docs, paper sources, tips, reference material,
generated outputs, tests, fixtures, and examples so research material can keep
bad examples without failing the repository.

## Paper Language

The paper should frame release behavior as merge evidence, not ceremony. The
claim is narrow: Jankurai does not prove a release is semantically correct, but
it can require the repository to expose the source, version, artifact identity,
integrity evidence, release automation, approval, monitoring, and recovery path
needed for a defensible release decision.
