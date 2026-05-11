## Winner-Only Architecture

The winner architecture is intentionally narrow: Rust core, TypeScript/React/Vite product surface, PostgreSQL truth, generated contracts, and exception-only Python for rare advanced ML/data work. The goal is not elegance in a diagram. The goal is a repository that tells an agent where a change belongs, what it is allowed to touch, which proof lane must run, and what kind of repair is acceptable.

An agent-native repo should make confusion expensive and correct routing cheap. The filesystem, dependency graph, generated zones, test map, docs, and CI gates should all say the same thing.

### Default Repository Shape

```text
repo/
  AGENTS.md
  agent/
    standard-version.toml
    owner-map.json
    test-map.json
    proof-lanes.toml
    generated-zones.toml
    agent/repo-score.json
  apps/
    web/                    # TypeScript, React, Vite, generated clients only
    api/                    # Rust Axum/Tower or ConnectRPC edge
  crates/
    domain/                 # pure invariants, IDs, state machines, no I/O
    application/            # commands, authz, idempotency, transactions
    adapters/               # DB, queues, external APIs, filesystem, env
    workers/                # async jobs, workflow glue, retries, replay
  contracts/
    openapi/
    protobuf/
    json-schema/
    generated/
  db/
    migrations/
    constraints/
    seeds/
    snapshots/
  python/
    ai-service/             # exception-only advanced ML/data, typed API, no product truth
  ops/
    ci/
    observability/
    security/
    release/
  docs/
    decisions/
    exceptions/
    repair/
    runbooks/
```

This shape is part of the standard. It is not decoration. A repo that puts handlers, SQL, UI types, prompt code, and workflows wherever they first became convenient is asking the agent to infer architecture from accident. That is exactly what must end.

### Root Instruction Surfaces

Agent tools increasingly depend on repo-local instruction files. Codex uses `AGENTS.md` conventions. Claude Code uses project memory such as `CLAUDE.md`. Cursor uses rule files under `.cursor/rules`. Other tools have their own steering surfaces. The standard should support them without letting instructions fork into contradictory manuals.

The rule:

| Surface | Purpose | Limit |
| --- | --- | --- |
| `AGENTS.md` | universal routing: owners, proof lanes, generated zones, forbidden edits | short, root-level, no long tutorials |
| `.cursor/rules/*` | Cursor-specific editing and review rules | mirrors the standard, no extra architecture |
| `CLAUDE.md` or equivalent | Claude-specific memory and commands | mirrors the standard, no conflicting permissions |
| local `AGENTS.md` files | crate/app-specific rules | only local concerns, must not override root boundaries silently |
| `docs/repair/*` | repair recipes, common failures, known fixes | evidence-backed and versioned |

The root file should tell the agent where to go, not teach the whole system. Deep context belongs near the code it governs.

### Ownership Cells

An ownership cell is a bounded region with one reason to change, one default proof lane, one owner path, and one set of allowed dependencies.

| Cell | Owns | Must not own | Default proof |
| --- | --- | --- | --- |
| `apps/web` | UI, routes, forms, local interaction state, client-side validation, generated API clients | durable truth, secrets, core authz, direct DB writes, handwritten API mirrors | typecheck, component tests, Playwright critical paths, contract-client diff |
| `apps/api` | HTTP/RPC edge, request normalization, auth middleware, rate limits, idempotency keys, response shaping | UI state, domain invariants hidden in handlers, raw SQL scattered through routes | integration tests, contract tests, authz matrix |
| `crates/domain` | IDs, value objects, invariants, state machines, pure decisions, stable error types | I/O, env reads, network, filesystem, database clients, tracing side effects, framework code | unit tests, property tests, mutation-sensitive tests |
| `crates/application` | commands, queries, authz policy, transaction orchestration, idempotency, use-case flow | transport details, UI concerns, SQL driver code, prompt/model logic | use-case tests, transaction tests, role/scope tests |
| `crates/adapters` | PostgreSQL access, queue clients, external API clients, filesystem, env, secret providers | business rules that require domain language to explain | integration tests, adapter contract tests, failure injection |
| `crates/workers` | async jobs, retries, backoff, dead-letter handling, workflow replay, scheduled tasks | UI behavior, duplicated invariants, ad hoc product truth | workflow tests, replay tests, idempotency tests |
| `contracts/*` | OpenAPI, Protobuf, JSON Schema, generated stubs and clients | business logic, manual edits to generated files | schema lint, generation check, backward-compatibility diff |
| `db/*` | migrations, constraints, indexes, RLS policies, seed rules, schema snapshots | app orchestration, controller logic, hidden business process | migration tests, schema drift checks, constraint tests |
| `python/ai-service` | rare approved advanced ML/data library work, embeddings, evals, typed service boundary | product truth, authz, billing, repo tools, proof lanes, general backend glue, direct production DB ownership, user-facing workflow state | contract tests, eval suites, no-direct-db scan |
| `ops/*` | CI, release, telemetry, secret scanning, SBOM, SCA, provenance, policy gates | feature behavior, hidden manual approvals as product logic | policy checks, security lane, release dry run |

The shortest rule: if a cell needs another cell's private knowledge to be safe, the boundary is wrong.

### Exact Language Boundaries

The winning stack is polyglot by design, but each language gets a narrow job.

| Language/layer | Should own | Should not own | Practical cap |
| --- | --- | --- | --- |
| Rust | domain invariants, application use cases, authz, idempotency, workflows, adapters, API edge, workers, parsers, compute-heavy logic | browser UI, notebooks, prompt experiments, handwritten generated clients | Most durable backend code belongs here |
| TypeScript | React UI, Vite build, route state, form state, local validation, generated clients, Storybook/component tests, Playwright tests | durable truth, billing truth, direct SQL, core authz, workflow state, duplicate backend DTOs | Product surface and tests, not backend truth |
| PostgreSQL/SQL | durable records, constraints, indexes, migrations, RLS, uniqueness, referential integrity, transactional truth | application orchestration, UI logic, model prompts | Truth that must survive app bugs belongs here |
| Python | rare advanced ML/data library work with a dated exception | product API ownership, authz, billing, repo tools, proof lanes, direct prod DB writes, general backend glue | Boxed to `python/ai-service`; not a default implementation language |
| Shell | thin wrappers around standard commands | business logic, multi-page deployment logic, hidden data mutation | Keep scripts short and route to typed tools |

The Python boundary is strict because Python is both useful and dangerous. It is useful when a Python-only advanced ML/data library is genuinely required. It is also the easiest place for "just glue this" to become unowned production truth. The audit should treat any new Python without a dated advanced-ML/data exception as suspicious until proven otherwise.

### Dependency Direction

Dependencies should point inward toward domain truth or outward through declared ports, never sideways by convenience.

```text
apps/web -> contracts/generated
apps/api -> crates/application -> crates/domain
crates/application -> port traits -> crates/adapters
crates/workers -> crates/application + crates/adapters
crates/adapters -> db/contracts/external systems
python/ai-service -> contracts only
ops -> build/test/security/release tools
```

Forbidden edges:

- `apps/web` to database, secrets, Rust internals, or handwritten backend DTOs.
- `apps/api` to UI code or duplicated product rules.
- `crates/domain` to I/O, env, logging side effects, framework code, or database code.
- `crates/adapters` to own domain rules.
- `python/ai-service` to production database ownership.
- `ops` to hidden feature logic.
- any generated file to become the source of truth.

### Contracts and Generated Zones

Every boundary needs one source of truth and one generated or enforced mirror.

| Boundary | Source of truth | Generated/enforced mirror | Failure caught |
| --- | --- | --- | --- |
| Browser to API | OpenAPI, Protobuf, or JSON Schema | generated TypeScript client, runtime validation where useful | stale frontend assumptions |
| API to application | Rust command/query types and auth policy | handler tests and application tests | transport logic becoming business logic |
| Application to domain | validated Rust constructors and enums | unit/property tests | invalid state |
| Application to adapters | port traits and transaction abstractions | adapter contract tests | side effects leaking into use cases |
| App to PostgreSQL | migrations, constraints, indexes, RLS | migration/schema drift checks | app-only data truth |
| Rust to Python | typed RPC/queue/schema contract | contract tests and eval gates | Python taking product ownership or becoming backend glue |
| Repo to agents | owner map, test map, proof lanes, generated-zone manifest | audit report and CI gate | directionless edits |

Generated zones must be declared in `agent/generated-zones.toml`. Files in generated zones are read-only to agents unless the patch also changes the generator or source contract. Hand-edited generated code is a hard failure.

### File and Module Size Rules

Agent-native code should fit into context without hiding responsibilities.

| Item | Soft limit | Hard limit | Required action |
| --- | ---: | ---: | --- |
| Rust source file | 300 LOC | 500 LOC | split by domain concept, port, adapter, or use case |
| TypeScript/TSX file | 250 LOC | 450 LOC | split component, hook, generated client, or test fixture |
| Exception-only Python file | 250 LOC | 400 LOC | split eval, model client, transform, or service boundary |
| Markdown instruction file | 150 LOC | 250 LOC | move details into local docs or repair recipes |
| Function/method | 40 LOC | 80 LOC | extract named decision, parser, adapter call, or test helper |
| Directory | 20 peer files | 35 peer files | introduce ownership subfolders |

The numbers are not style trivia. Large files make agents over-edit, miss invariants, and patch the wrong layer. Refactoring should preserve behavior first, then narrow ownership. A size violation is not proof of bad code, but it is proof the repo owes the agent a clearer map.

### Testing Structure

Tests should be organized by proof question, not by the convenience of the first author.

| Proof question | Test type | Location |
| --- | --- | --- |
| Does the domain reject invalid state? | Rust unit/property tests | `crates/domain` |
| Does the use case enforce authz and transactions? | Rust application tests | `crates/application` |
| Does the adapter match reality? | Integration/contract tests | `crates/adapters` |
| Does the API match the contract? | Contract and handler tests | `apps/api`, `contracts` |
| Does the UI behave on critical paths? | Component tests and Playwright tests | `apps/web` |
| Does the database preserve truth? | Migration, constraint, schema drift tests | `db` |
| Does the Python exception stay bounded? | Contract tests and eval suites | `python/ai-service` |
| Does the whole repo remain agent-safe? | Audit script and proof-lane checks | `agent`, `ops/ci` |

Playwright is the default browser automation choice for this stack because it is widely adopted, cross-browser, CI-friendly, and fits TypeScript product surfaces. It should prove critical user paths, auth/session behavior, permissions boundaries, and regressions that unit tests cannot see. It should not become a dumping ground for every UI assertion. Most behavior should still be proven closer to the owning cell.

### Agent-Friendly Exceptions

Exceptions are not loopholes. They are versioned, searchable repair knowledge.

Every exception should live under `docs/exceptions/` and include:

| Field | Required content |
| --- | --- |
| Exception name | stable ID, for example `PY-DB-001` |
| Purpose | what this exception permits |
| Reason | why the standard cannot be followed now |
| Scope | exact files, crates, services, or contracts affected |
| Common fixes | repair patterns agents should try first |
| Proof lane | tests/security checks required for edits |
| Expiration or exit criteria | date, milestone, or measurable removal condition |
| Documentation link | canonical local doc or external reference |

The same idea applies to runtime exceptions/errors in code. Rust errors, TypeScript error objects, and approved Python exception-service errors should carry stable names, purpose, reason, and repair hints where appropriate. The goal is not verbose failure text. The goal is for an agent to see a failure and know the next bounded move.

### Observability and Repair Evidence

OpenTelemetry-style traces, metrics, and logs should be treated as part of the architecture. Every externally visible request should carry enough structured context to reconstruct ownership:

- request ID or trace ID
- authenticated principal shape, without secrets
- route or RPC method
- contract version
- domain command/use case
- database transaction boundary where relevant
- external dependency calls
- error code with documentation link
- release/build/provenance identifier

The repair loop should produce receipts: what changed, which proof lanes ran, which generated zones changed, which contracts changed, and which audit findings were closed or introduced. Without receipts, the next agent starts over.

### Boundary Rules

- TypeScript owns product interaction, not durable truth.
- Rust owns backend truth, use cases, authorization, workflows, and side effects through explicit layers.
- PostgreSQL owns durable facts, constraints, indexes, migrations, and transaction semantics.
- Python owns only approved advanced ML/data capability through typed service boundaries.
- Contracts own cross-language shape. Handwritten mirrors are defects.
- Generated files are outputs. Editing them by hand is a defect.
- Root instructions route. Local docs explain. Neither should contradict the standard.
- Security scanning, contract checks, generated-zone checks, and fast proof lanes belong in CI.
- Exceptions must be named, documented, scoped, and given exit criteria.
- Any "temporary" fallback path must be treated as production architecture unless it is deleted before merge.

The winning architecture is narrow because narrowness is what makes it repairable. If the UI is wrong, inspect the UI cell. If the contract is wrong, inspect the contract source. If the invariant is wrong, inspect Rust domain. If the persisted fact is wrong, inspect PostgreSQL. If approved model behavior is wrong, inspect the exception-bounded Python service. The repo should answer "where does this belong?" before the agent has time to guess.
