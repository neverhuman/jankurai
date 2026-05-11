# Jankurai Task Tracker

Last updated: 2026-05-04
Owner: agent

This is the living tracker for the remaining program work. It is not a second roadmap. It keeps the long-horizon plan visible while we close the last hardening gaps and preserve receipts.

## Program Snapshot

The phase router is already populated through Phase 13. Most phases are complete or hardened. The active execution concern is Phase 02 hardening residue: the rule engine works, but the remaining semantic-oracle cleanup still needs deterministic, fixture-backed proof.

Working execution status:

- Phase 00: complete
- Phase 01: hardened
- Phase 02: complete with residual hardening work
- Phase 03: hardened
- Phase 04: complete
- Phase 05: complete
- Phase 06: complete
- Phase 07: hardened
- Phase 08: complete
- Phase 09: complete
- Phase 10: hardened
- Phase 11: hardened
- Phase 12: hardened
- Phase 13: hardened

## Active Queue

| ID | Area | Status | Next action | Proof |
| --- | --- | --- | --- | --- |
| P02-1 | Boundary manifest normalization | done | `agent/boundaries.toml` now parses Rust, TypeScript, and Python policy sections as first-class inputs. | `cargo test -p jankurai boundary::manifest::tests` |
| P02-2 | Deterministic fingerprints | done | Stable `sha256:` fingerprints now replace unstable default-hasher output. | `cargo test -p jankurai finding_builder::tests` |
| P02-3 | Deterministic owner routing | done | Overlapping owner-map prefixes now resolve in a stable longest-prefix order. | `cargo test -p jankurai finding_builder::tests` |
| P02-4 | Analyzer contract | open | Keep analyzer output rule-backed and preserve semantic evidence through the finding builder path. | `cargo test -p jankurai --test audit_smoke`, `cargo test -p jankurai --test proof_surface_smoke` |
| P02-5 | Fixture corpus | open | Add positive, negative, and edge fixtures for the semantic rule families that still rely on smoke-level coverage. | `cargo test -p jankurai --test rule_registry_smoke --test audit_smoke` |
| P02-6 | Rule registry parity | open | Keep `rules export`, `rules verify`, the standard docs, and the emitted registry aligned. | `cargo test -p jankurai --test rule_registry_smoke` |
| P02-7 | Receipt honesty | open | Reconcile the Phase 02 receipt and phase feedback status so the tracker reflects the real state after the hardening pass lands. | `git diff --check`, phase log append |

## Deferred But Tracked

- Live GitHub draft PR creation remains network/auth dependent.
- Optional hosted dashboards and external attestation overlays remain deferred overlays, not blockers.
- Roadmap-only vocabulary in docs must stay separated from current behavior.

## Completion Criteria

The larger plan is not done until all of these are true:

1. The tracker queue is empty or explicitly deferred.
2. Phase receipts and logs match current behavior.
3. The audit and proof lanes still pass.
4. The rule engine emits deterministic, registry-backed findings with stable proof routing.
5. Any future work is called out as a tracked follow-on instead of being buried in prose.

## Operating Rule

When a change touches audit behavior, proof routing, boundaries, generated zones, or repair flow, update this tracker in the same turn as the code or docs change. Do not rely on memory as the system of record.
