# Family manifest contract

This is the contract surface for the Jankurai split family. The hub publishes a
stable, machine-readable agreement that every family member and every consumer
of a fused release depends on. The contract has two protected, complementary
sources of truth:

- [`repos.manifest.toml`](../repos.manifest.toml) — declares the set of member
  repos, each member's role, and the public/local routing for the family.
- [`family.lock`](../family.lock) — pins every member to an immutable release
  tag and commit SHA consumed by a fused release.

The similarly named manifest at the split-container root is a generated runtime
projection, not a third authority. `scripts/project-family-manifest.sh --check`
requires it to match the tracked manifest byte-for-byte beneath a deterministic
source-hash header; only `--apply` may refresh it. The tracked manifest keeps
`authority_forge = "local_transition"` until the final protected hosted cutover.

## Contract guarantees

1. **Membership is closed and declared.** A release fuses exactly the members
   listed in `repos.manifest.toml`; nothing is pulled in implicitly.
2. **Every pin is immutable.** Each `family.lock` entry resolves to a tag and a
   commit SHA, never a branch and never a local checkout path. This is enforced
   by `scripts/validate-family.sh`.
3. **The manifest and lock stay coherent.** Every member in the manifest has a
   lock entry and vice versa; drift fails the fast lane.
4. **Operational identity is complete.** Every row binds one canonical physical
   path, `root/<repo>` slug, `main` default branch, `<repo>/required` check, and
   canonical hosted URL. Duplicate or missing mappings fail validation.
5. **GitHub is not a write route.** Historical GitHub URLs remain readable
   evidence, but every manifest row has `mirror_github = false`, and the legacy
   provisioning entrypoint is intentionally fail-closed.

## Compatibility and drift

The contract is verified, not hand-checked. `scripts/validate-family.sh` is the
drift gate: it checks required split metadata, Jeryu mirror config, lockfile
pins, missing lockfiles, branch dependencies, committed cross-repo path
dependencies, and action-pinning posture on every run. The attestation schemas
for the audit artifacts this contract is proven against live in
[`../schemas/`](../schemas/).

## Changing the contract

To add or repin a member, edit `repos.manifest.toml` and `family.lock` together,
refresh the root projection only from that tracked source, then run `just fast`.
A release promotes the validated manifest + lock pair as the fused-release bill
of materials (see [`../docs/release.md`](../docs/release.md)).
