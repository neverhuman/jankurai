## Agent-Friendly Exceptions And Automated QA

Agent-friendly exceptions are one of the most important ideas in the standard. Most teams already have a hidden exception catalog in human memory: "when this error happens, check the tenant mapping," "this provider returns 409 when it means stale token," "this migration fails when the old enum exists," "this model error usually means the embedding dimension changed." jankurai turns that memory into structured, versioned repair knowledge.

### Runtime Exceptions As Repair Packets

A controlled boundary error should carry:

| Field | Meaning |
| --- | --- |
| `name` | stable searchable exception name |
| `code` | stable machine-readable error code |
| `purpose` | invariant or boundary protected |
| `reason` | why this instance failed |
| `common_fixes` | likely repairs agents should try first |
| `docs_url` | direct local or public documentation link |
| `owner` | owning cell |
| `severity` | repair urgency |
| `retryable` | whether automation may retry |
| `correlation_id` | request, trace, job, or repair id |
| `source` | safe underlying cause |

RFC 9457 problem details, OpenTelemetry exception semantic conventions, JavaScript `Error.cause`, Rust contextual error patterns, and typed language-native errors all point toward the same shape: failures should be structured enough for tools to route.

### Language Patterns

| Layer | Pattern | Forbidden |
| --- | --- | --- |
| Rust domain | enum errors with stable codes and invariant language | `panic!`, `unwrap`, `expect` in production paths without documented invariant |
| Rust application | typed use-case errors mapped to API problem details | stringly authz or transaction failures |
| TypeScript UI | discriminated API error unions and safe user messages | `throw "message"`, broad `any`, hidden catch-all fallbacks |
| PostgreSQL | named constraints mapped to stable application errors | anonymous constraints and app-only durable truth |
| Exception-only Python AI/data service | typed service-boundary exceptions mapped to schema-defined errors | raw provider exceptions crossing the product boundary |
| Workers | retryable/permanent error split with idempotency key and trace id | infinite retry loops and log-only failure handling |

The point is not to make every function verbose. The point is that every boundary failure should teach the next agent what happened and where repair belongs.

### Exception Catalog

Each repo should maintain:

```text
docs/exceptions/
  HB_CONTRACT_DRIFT.md
  HB_PYTHON_DB_BOUNDARY.md
  HB_PROVIDER_RETRYABLE.md
  HB_UNSUPPORTED_STATE.md
```

Each entry needs owner, purpose, trigger, common fixes, proof lane, examples, and expiration if it is a policy exception. This is how teams pool vibe-coding lessons without letting them remain vibes.

### Test Coverage In The Agent Era

Agent-native testing is an evidence routing system:

| Layer | Minimum proof | Preferred tools |
| --- | --- | --- |
| Rust domain | unit tests and property tests for invariants/state machines | `cargo test`, `proptest`, table tests |
| Rust application | authz, idempotency, transaction, workflow tests | integration tests, test containers where useful |
| Rust adapters | DB/external contract tests and failure injection | `sqlx` checks, local services, fakes with contract tests |
| TypeScript UI | component behavior and accessibility checks | Vitest, Testing Library, axe where useful |
| Browser E2E | critical product flows, auth/session, permissions | Playwright by default |
| PostgreSQL | migration apply, constraint checks, rollback policy | migration test harness, schema drift checks |
| Contracts | generation and backward compatibility | OpenAPI/Protobuf/JSON Schema checks |
| Exception-only Python AI/data service | eval fixtures, model IO contracts, reproducibility | typed contract tests, golden evals |
| Ops/security | secrets, dependencies, SBOM, provenance, workflow lint | gitleaks, dependency review, Syft/Grype, SLSA, Zizmor |
| Audit | standard compliance and agent repair queue | `cargo run -p jankurai --` |

Playwright is the default browser tool for this stack because its official guidance aligns with agent-friendly QA: isolate tests, use user-visible locators, prefer role/text/test-id locators, use web-first assertions, avoid brittle implementation selectors, and capture traces/screenshots/videos on failure.

### Preventing Test Explosion

AI will make tests cheap to generate. That is dangerous if the repo treats quantity as quality. The test strategy must prevent coverage from becoming another junk drawer.

Rules:

- New behavior gets tests in the owning cell first.
- A bug fix gets a regression test where the bug should have been caught.
- Browser tests cover critical flows, not every button state.
- Property tests cover invariants and state spaces, not static snapshots.
- Snapshots need semantic assertions or they become approval spam.
- Skipped tests require owner, reason, issue, and expiration.
- Retries are allowed only for known flaky infrastructure, not unknown behavior.
- Mocks cannot replace generated contracts when a contract test is practical.
- Test data builders live in named support modules, not random helper files.

The purpose of automated QA is not to eliminate judgment. It is to make human judgment operate over evidence instead of hope.
