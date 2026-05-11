# Concepts

Last reviewed: 2026-03-31

## ARC

`ARC` stands for **Agent Responsibility Cell**.

An ARC is the smallest self-sufficient change cell that an agent should touch by default. In this repo, the ARC is usually a crate, not an arbitrary folder.

An ARC owns:

- a coherent responsibility,
- a narrow public surface,
- explicit invariants,
- local validation commands,
- graph-visible dependencies,
- explicit exception references when strict defaults are broken.

## VRC

`VRC` stands for **Validation Radius Contract**.

A VRC describes how validation expands from the local proof ring outward:

1. local compile proof,
2. local tests,
3. local doctests,
4. mapped reverse dependencies,
5. contract tests,
6. smoke tests,
7. full end-to-end only when the outer boundary actually moved.

The key rule is simple: validation expands by dependency graph and test graph, not by directory ancestry.

## AER

`AER` stands for **Agent Exception Record**.

An AER is required when the repo intentionally breaks a strict default such as:

- a mega-file,
- a mega-function,
- a monolith crate,
- feature explosion,
- hidden side effects,
- macro opacity,
- orphan cross-crate invariants.

Each AER captures:

- the broken rule,
- the reason the exception improves reality,
- the risk introduced,
- the fix tip or migration path,
- docs links,
- a sunset condition.

## Minimum Semantic Surface

This repo uses **minimum semantic surface** as the working definition of optimal code.

The phrase is more precise than “least code.”

It means code with:

- the minimum interface surface needed to express the job,
- the minimum dynamic work needed to satisfy the contract,
- the minimum ambient authority needed to run,
- the minimum synchronization and allocation pressure needed for the workload,
- the maximum proof density practical for the team and system,
- extensibility through explicit interfaces instead of speculative abstraction.

This is proposed doctrine, not a universal law. The doctrine exists because autonomous coding magnifies the cost of hidden behavior, oversized surfaces, and needless runtime work.
