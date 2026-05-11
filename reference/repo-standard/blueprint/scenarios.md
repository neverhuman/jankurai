# Validation Scenarios

Last reviewed: 2026-03-31

These scenarios are the acceptance suite for the blueprint.

## 1. Leaf Bugfix

- Context scope: matched ARC only
- Validation ladder: local `cargo check`, local tests, local doctests
- Stop condition: local proof green and no public or manifest boundary touched

## 2. Public API Change

- Context scope: changed ARC plus reverse dependencies
- Validation ladder: local proof, reverse-dependency tests, contract tests, semver checks
- Stop condition: public surface hash changed and all mapped consumers stay green

## 3. Shared Crate Refactor

- Context scope: shared ARC plus known consumers
- Validation ladder: local proof, reverse dependencies, smoke flow if adapters moved
- Stop condition: changed shared surface and consumer graph validated

## 4. Feature Change

- Context scope: ARC plus feature graph metadata
- Validation ladder: local proof, grouped feature checks, mapped reverse dependencies
- Stop condition: changed feature combinations are explicitly covered or rejected

## 5. Compile-Fail or Type Contract Change

- Context scope: ARC plus compile-fail fixtures or trait consumers
- Validation ladder: local proof, compile-fail tests, reverse dependencies when trait or type shape moved
- Stop condition: expected compiler diagnostics remain stable

## 6. CI-Only Failure

- Context scope: changed ARC plus the CI profile that failed
- Validation ladder: reproduce the failing profile locally, then narrow to the smallest passing proof
- Stop condition: the failing CI profile is green and no broader ring was silently skipped

## 7. External Boundary Change

- Context scope: ARC, contract fixtures, smoke flow, and end-to-end edge
- Validation ladder: local proof, contract tests, smoke tests, full E2E when workflow behavior changed
- Stop condition: the external contract and user-visible flow both remain green

## Worked Example

The fixture in `labs/repo-shape-bench` demonstrates scenario 1.

- Monolith result: one package, 11 context files, 4,928 context bytes, 3 selected test commands.
- Arcified result: one ARC, 2 context files, 1,094 context bytes, 3 selected test commands.

The important point is not only runtime. The ARC layout makes the intended context surface obvious before the agent reads the code.
