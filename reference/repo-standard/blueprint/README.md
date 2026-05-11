# Blueprint

Last reviewed: 2026-03-31

This folder is the normative repo standard behind the paper and toolchain.

## What Lives Here

- `concepts.md`: ARC, VRC, AER, and the doctrine of minimum semantic surface.
- `repo-standard.md`: the recommended workspace layout and operating rules.
- `scenarios.md`: deterministic validation plans for seven common change classes.
- `exception-catalog.yaml`: strict inherited code classes with allowed exceptions.
- `schemas/`: machine-readable contracts for generated artifacts and records.
- `examples/`: example outputs and manifests that tools can round-trip.

## Design Intent

The blueprint assumes:

1. the default unit of work is a small crate-sized ARC,
2. validation expands by dependency and test graph,
3. exceptions are explicit records rather than tribal memory,
4. benchmark artifacts are first-class review inputs,
5. the paper, the Markdown edition, and the code all point at the same artifacts.
