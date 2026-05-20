# bug: block agent-authored audit masking of Rust/core files

## Summary

Jankurai should block agent-authored changes that make audit failures invisible by
excluding Rust files, core repository files, or broad source roots from the scan,
or by post-processing audit output to remove caps/findings/issues.

A downstream agent cleanup recently created a false-green path by:

- adding tracked Rust/core surfaces to audit masking policy, including a concrete
  Rust file and a broad `testing/` tree that contained `*.rs` tests
- filtering `caps_applied` and `findings` after `jankurai audit` emitted the raw
  report, so the PR gate could pass without preserving the finding record

This should be treated as audit integrity drift, not as a normal policy edit.

## Bad Behavior To Block

Agents must not be able to add these paths to masking/exclusion surfaces and then
pass a merge gate:

- any tracked `*.rs` file
- Cargo manifests or Rust workspace control files
- root agent/control-plane files such as `AGENTS.md`, `agent/*.toml`,
  owner/test/proof routing, generated-zone policy, gate scripts, and workflow
  definitions
- broad roots that can contain tracked Rust/core files, such as `testing/`,
  `crates/`, `apps/`, `tools/`, or `agent/`
- generated-zone shields, ignore lists, or policy globs that indirectly hide the
  same files
- scripts that rewrite or filter `caps_applied`, `findings`, or `issues` after
  `jankurai audit` writes its report

Only a user should be able to intentionally make audit masking/exclusion policy
changes, and that path should require visible manual provenance/approval in the
diff or receipt.

## Proposed Controls

- Add an audit-integrity rule that emits a hard finding when an exclusion,
  generated-zone shield, ignore list, or mask matches tracked Rust/core files.
- Detect policy changes authored by agents and require a manual/user approval
  receipt before the gate accepts a new mask.
- Add gate-integrity detection for report post-processing that removes or
  rewrites `caps_applied`, `findings`, or `issues`.
- Include fixtures for:
  - direct `src/foo.rs` exclusion
  - broad `testing/` exclusion containing Rust tests
  - Cargo/root control-plane file exclusion
  - generated-zone or ignore-list masking of tracked Rust/core files
  - post-audit `jq` or equivalent filtering of findings/caps/issues
- Emit a machine-readable finding with a repair hint that tells agents to remove
  the mask and fix the underlying audit finding instead.

## Expected Behavior

`jankurai audit` and canonical PR gates fail when an agent hides Rust/core audit
coverage or tampers with raw finding output. The failure should be deterministic
and should point at the masking file or gate script that caused the loss of
visibility.

## Acceptance Criteria

- Agents cannot add Rust/core files or broad source roots to audit masking and
  still pass.
- A human/manual exception path is explicit, visible, and reviewable.
- Existing non-Rust fixture exclusions continue to work when they cannot match
  tracked Rust/core files.
- Post-audit filtering of caps/findings/issues is detected as a hard failure.
- GitHub, GitLab, and local agent workflows all receive the same blocking
  signal.

## Context

This issue is intentionally concrete because the failure mode is dangerous:
false-green audit gates train agents to hide violations instead of repairing
them. The downstream repository fixed the local masking guard, but Jankurai
should enforce this class centrally so future repositories do not need bespoke
defense scripts.
