# JEP-0001: Governance and RFC Process

**Status**: Accepted
**Author**: Jankurai Core Team
**Created**: 2026-05-05

## Motivation

Jankurai is moving from a powerful reference implementation to a ubiquitous standard. To ensure the standard remains rigorous but allows for community contribution without diluting the anti-vibe mission, we need a formal governance and Request for Comments (RFC) process. 

The Jankurai Enhancement Proposal (JEP) process provides a structured, high-quality path for introducing new certified cells, changes to the core standard, schema bumps, or major architectural shifts.

## Design

### The JEP Lifecycle

1. **Draft**: The proposal is being authored and shaped.
2. **Proposed**: The proposal is submitted as a PR and is actively under review by the governance board.
3. **Accepted**: The proposal has been approved and is ready for implementation or already implemented.
4. **Rejected**: The proposal was considered but did not align with Jankurai's mission or constraints.
5. **Deprecated**: The proposal was once accepted but is now superseded by another JEP or standard version.

### Mandatory JEP Sections

Every JEP must include the following sections to ensure high-quality discussion:

*   **Motivation**: Why is this change necessary? What problem does it solve?
*   **Design**: The technical architecture, API changes, or schema modifications.
*   **Migration Impact**: How does this affect existing conformant repositories? Are there automated tools to migrate?
*   **Evidence**: Proof that this proposal adheres to Jankurai's "No proof, no merge; no receipt, no trust" thesis.
*   **Agent Impact**: How will this change affect autonomous agents interacting with Jankurai? (e.g., token usage, rule parsing).

### Governance Board Roles

The board ensures that any change to the standard has a clear owner. As defined in the Release Plan:

*   **Standard Editor**: Owns rule text and release notes.
*   **Auditor Maintainer**: Owns scanner behavior and output schema.
*   **Evidence Maintainer**: Refreshes sources and research claims.
*   **Template Maintainer**: Keeps starter repos current.
*   **Benchmark Maintainer**: Owns tasks and raw results.
*   **Security Reviewer**: Reviews rules that affect CI gates and secrets.

## Migration Impact

This process establishes a path forward for new features. It does not break any existing repositories. Future JEPs will be required for breaking changes to the `jankurai` CLI or standard rules.

## Evidence

This JEP defines the structure for all future evidence. No code changes are associated with this JEP itself.

## Agent Impact

Agents must refer to the JEPs to understand the historical context and architectural decisions of Jankurai. JEPs are stored as readable markdown files within `docs/jeps/`.
