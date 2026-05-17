# Coverage Evidence Tools

Jankurai parses deterministic local artifacts. These tools are recommended producers, not mandatory dependencies.

| Surface | Recommended producers | Notes |
| --- | --- | --- |
| Rust execution | `cargo-nextest`, `cargo-llvm-cov` | Prefer changed-line LCOV over total project percentage. |
| Rust behavior | `cargo-mutants`, `proptest`, `cargo-fuzz`, `miri`, `kani` | Mutation survivors and generated counterexamples are stronger than line hits. |
| TypeScript/Vite/React | Vitest coverage with `@vitest/coverage-v8`, StrykerJS, `fast-check`, Playwright, Storybook test runner | Configure coverage include rules so untouched files are visible. |
| API contracts | Schemathesis, Specmatic, Pact, Spectral, `oasdiff` | RESTler is useful for deep or nightly stateful API fuzzing. |
| PostgreSQL | Testcontainers, `pgTAP`, Atlas migrate lint/test, Squawk, SQLFluff, SQLx compile-time checks | Migration and isolation proof routes through DB/auth/input HLT rules. |
| Docker and supply chain | Trivy, Hadolint, Syft, Grype, Docker Scout, Dockle, container-structure-test, Goss | Local JSON outputs are preferred. |
| Dashboards | Codecov, Qlty, SonarQube | Optional dashboard inputs only; never required for local audit. |
| AI accelerators | Keploy, Meticulous, Cover Agent, Qodo, Copilot | Useful for generating candidate tests, but deterministic artifacts decide pass/fail. |

Implemented v1 parsers:

- `lcov`
- `cargo-mutants-json`
- `stryker-json`
- `trivy-json`
- `hadolint-json`
- `jankurai-json`
- `generic-json-summary`

The Jankurai repo's local CI requires `cargo-mutants` because
`agent/coverage-sources.toml` expects changed-code mutation evidence at
`target/mutants/mutants.out/outcomes.json`. Run `just ci-coverage` to generate
that file; do not hand-write it.

Future parser candidates include Cobertura, JaCoCo, Istanbul JSON, Playwright JSON, Storybook test runner JSON, Schemathesis reports, Pact verification output, SQLFluff JSON, Squawk JSON, Syft CycloneDX/SPDX, and Grype JSON.

Avoid archived or stale tools as primary proof sources. Dredd, for example, should be treated as legacy unless a project already has reviewed local evidence and an owner-backed migration plan; its GitHub repository was archived on November 8, 2024: <https://github.com/apiaryio/dredd>.
