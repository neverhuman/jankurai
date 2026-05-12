# jankurai Agent-Native Repository Standard

Standard version: `0.8.0`
Published: `2026-05-05`
Paper: `Jankurai: A Versioned Repository Conformance Standard for Trustworthy AI-Assisted Merge`
Public thesis line: `No proof, no merge; no receipt, no trust.`
Target stack: Rust core, TypeScript/React/Vite product surface, PostgreSQL truth, generated contracts, exception-only Python AI/data service.
Implementation default: use Rust for repository tools, core behavior, proof lanes, and automation whenever practical. Agents must not create or expand Python for repo tooling, proof lanes, product services, general backend glue, product truth, authorization, or direct production DB access. The only allowed Python exception is rare: advanced ML/data work that depends on a Python-only library, is boxed under `python/ai-service`, and has a dated exception with owner, expiry, proof lane, and migration/containment plan.

This is an operational standard for coding agents and maintainers. Repositories do not need jankurai merely because they use AI. Repositories claiming jankurai conformance should point root agent instructions to this file and to `agent/JANKURAI_STANDARD.md`.

## 1. Mission

The repo is no longer optimized for a human to remember intent. It is optimized for agents to reject wrong code quickly, localize failures, repair narrow scopes, and leave auditable evidence.

The winning repository makes these facts machine-obvious:

- what owns each behavior
- what must not own that behavior
- which files are generated
- which commands prove a change
- which failures are actionable
- when a file is too large to edit safely
- when a dependency, fallback, or duplicate is forbidden

Agent-native engineering treats "vibe coding" as a defect class: ambiguous ownership, broad rewrites, hand-maintained contracts, silent fallbacks, scattered truth, large files, skipped tests, and undocumented exceptions.

## 2. Adoption Contract

Every repository claiming `HL3` or higher MUST include:

- `AGENTS.md` at repo root with a short pointer to this standard.
- `agent/JANKURAI_STANDARD.md` copied or vendored from this standard.
- `agent/owner-map.json` mapping paths to owners and allowed dependencies.
- `agent/test-map.json` mapping paths to validation lanes.
- `agent/zyal/**/*.zyal` as the canonical ZYAL runbook root. `agent/zyal/README.md`
  may exist as the only non-runbook file, and `.zyal.yml` / `.zyal.yaml` are
  legacy forms that should be renamed.
- `agent/generated-zones.toml` or equivalent generated-file manifest.
- `agent/standard-version.toml` binding paper, standard, audit, schema, and artifact versions.
- `agent/repo-score.json` produced by CI.
- one command for fast validation.
- one command for full validation.
- CI job that runs the jankurai audit on every pull request.

Recommended pointer:

```md
# AGENTS.md

Read `agent/JANKURAI_STANDARD.md` first.
For full policy, read `docs/agent-native-standard.md`.
Do not edit outside requested ownership. Run the mapped test lane before final response.
```

Conformance levels:

| Level | Meaning |
|---|---|
| `HL0` | unscored or unrouted repository |
| `HL1` | advisory audit emits JSON/Markdown |
| `HL2` | guarded critical caps block merge |
| `HL3` | standard score floor and high/critical blocking |
| `HL4` | ratchet mode prevents score regression without exception |
| `HL5` | release contract across audit, tests, security, contracts, DB, e2e, and versions |

Stable rule IDs:

| Rule | Meaning |
|---|---|
| `HLT-001-DEAD-MARKER` | future-hostile product/runtime marker |
| `HLT-002-GENERATED-MUTATION` | generated output changed outside source regeneration |
| `HLT-003-OWNERLESS-PATH` | path has no owner-map route |
| `HLT-004-UNMAPPED-PROOF` | path has no test-map proof lane |
| `HLT-005-PYTHON-PRODUCT-TRUTH` | Python owns durable product behavior |
| `HLT-006-DIRECT-DB-WRONG-LAYER` | DB access appears outside adapters/db |
| `HLT-007-HANDWRITTEN-CONTRACT` | public API/client contract is mirrored by hand |
| `HLT-008-FALSE-GREEN-RISK` | passing lane does not prove changed behavior |
| `HLT-009-GENERATED-SECURITY` | generated security-sensitive code lacks security proof |
| `HLT-010-SECRET-SPRAWL` | secret-like value, env dump, fixture, or transcript leak |
| `HLT-011-PROMPT-INJECTION` | untrusted context changes trusted policy/tool behavior |
| `HLT-012-OVERBROAD-AGENCY` | agent/tool permissions exceed lane scope |
| `HLT-013-RENDERED-UX-GAP` | user-facing UI lacks rendered proof |
| `HLT-014-A11Y-GAP` | UI lacks accessibility proof for changed surface |
| `HLT-015-CONTEXT-SETUP-GAP` | setup/context routing is not deterministic |
| `HLT-016-SUPPLY-CHAIN-DRIFT` | dependency/provenance change lacks review evidence |
| `HLT-017-OPAQUE-OBSERVABILITY` | boundary failure lacks repairable telemetry |
| `HLT-018-PERF-CONCURRENCY-DRIFT` | performance/concurrency risk lacks proof |
| `HLT-019-STREAMING-RUNTIME-DRIFT` | broker client or Kafka stack identity escapes adapter boundaries |
| `HLT-020-CI-HARDENING-GAP` | CI workflow permissions, unpinned actions, or proof posture gaps |
| `HLT-021-DESTRUCTIVE-MIGRATION` | destructive SQL under migration paths without documented safety evidence |
| `HLT-022-AUTHZ-ISOLATION-GAP` | authorization or tenant/data isolation lacks negative proof |
| `HLT-023-INPUT-BOUNDARY-GAP` | unsafe input boundary or sink lacks deterministic negative proof |
| `HLT-024-AGENT-TOOL-SUPPLY-GAP` | agent tool, MCP, hook, or rule supply chain lacks trust evidence |
| `HLT-025-RELEASE-READINESS-GAP` | release or launch gate lacks artifact-backed readiness evidence |
| `HLT-026-COST-BUDGET-GAP` | unbounded paid work lacks budget, quota, or stop-condition evidence |
| `HLT-027-HUMAN-REVIEW-EVIDENCE-GAP` | review or proof claim lacks reproducible receipts |
| `HLT-028-BOUNDARY-EVIDENCE-GAP` | audited runtime boundary reclassification lacks deterministic evidence |
| `HLT-029-RUST-BAD-BEHAVIOR` | Rust code uses unsafe, unchecked, or dishonest APIs without local proof |
| `HLT-030-SQL-BAD-BEHAVIOR` | SQL code or migrations use unsafe string assembly or unchecked execution without proof |
| `HLT-031-TYPESCRIPT-BAD-BEHAVIOR` | TypeScript code uses unchecked boundary or runtime shortcuts without proof |
| `HLT-032-DOCKER-BAD-BEHAVIOR` | Docker or container build behavior hides unsafe or unreviewed execution steps |
| `HLT-033-PYTHON-BAD-BEHAVIOR` | Python code owns runtime behavior or unchecked data paths without an approved exception |
| `HLT-034-CI-BAD-BEHAVIOR` | CI workflows hide unsafe, unpinned, or nonblocking security and proof behavior |
| `HLT-035-GIT-BAD-BEHAVIOR` | Git automation or hooks use destructive, hidden-state, or unreviewed mutation behavior |
| `HLT-036-GITTOOLS-BAD-BEHAVIOR` | Git hook managers or policy tooling normalize bypass, destructive mutation, or broad staging |
| `HLT-037-RELEASE-BAD-BEHAVIOR` | Release automation mutates tags/artifacts, skips proof, ships mutable latest-only outputs, or publishes without integrity evidence |
| `HLT-038-REFERENCE-PROFILE-STRUCTURE-GAP` | Reference-profile cells drift from canonical folder names or miss local AGENTS guidance |
| `HLT-039-WEB-SECURITY-BAD-BEHAVIOR` | Web apps expose high-confidence security hazards such as public Vite dev servers, client secrets, browser token storage, or credentialed wildcard CORS |
| `HLT-040-REPO-ROT-BAD-BEHAVIOR` | Active source contains ambiguous old, backup, copied, parked, or hard-disabled code without owner, proof lane, expiry, and cleanup plan |
| `HLT-041-COMMENT-HYGIENE` | Source code contains dangerous comments admitting unsafe behavior, temporary hacks, or AI scaffolding |
| `HLT-042-CI-LOCAL-PARITY` | CI workflows do not delegate to versioned `ops/ci/*.sh` scripts, so failures cannot be reproduced locally before push |

`HLT-029-RUST-BAD-BEHAVIOR` is detector-backed in this release. `HLT-030` through `HLT-042` are detector-backed catalog IDs in the bad-behavior family.

Centerline drift is the delta between claimed conformance and observed repository behavior. Hard caps are versioned policy, not final empirical truth.

Coverage evidence is proof support, not a score category. Line coverage is reachability; mutation, property, integration, API, database, UX, accessibility, and container evidence are stronger behavior signals. Missing optional coverage tools cannot block merge. Required proof gaps on changed critical surfaces route to the existing HLT rule for that surface.

## 3. Hard Gates

These are blocking violations unless an approved, dated exception exists in `docs/exceptions/`.

| Gate | Rule | Agent action |
|---|---|---|
| Root instructions | Repo has no root agent instructions | Add concise `AGENTS.md` pointer and stop broad edits until reviewed |
| One-command validation | No deterministic fast validation command | Add `just fast`, `make fast`, or equivalent before feature work |
| Ownership | File path has no owner or allowed dependency cell | Add owner-map entry before editing |
| Contract drift | Public API/schema changed without generated clients/tests | Regenerate contracts and run contract lane |
| Generated files | Hand edit inside generated zone | Revert hand edit, change source, regenerate |
| Too-large file | Non-generated file exceeds hard LOC limit | Refactor before adding behavior |
| Too-large function | Function exceeds hard LOC limit | Extract pure units before adding behavior |
| Silent fallback | Code hides failure with default, retry, catch-all, or stale data | Replace with explicit policy and agent-friendly error |
| Duplicate behavior | Same decision or transformation exists in multiple owner cells | Consolidate into owning layer |
| Paper filename ban | Paper sources mention `tips/*.txt` file names | Rewrite the paper to use corpus, row-family, or source-group labels instead |
| Direct DB misuse | UI, domain, or exception-only Python writes product truth directly | Move write into Rust application/adapters |
| Python sprawl | Python appears outside a dated advanced-ML/data exception or owns product behavior | Remove it or migrate the behavior to Rust/TypeScript/PostgreSQL |
| Security lane | High-risk change skips secret/dependency/static scanning | Add lane and block merge |
| Release structure | Release-capable project lacks version, changelog, process, automation, integrity, or rollback surface | Add the release control surface before trusting release scores |
| Release mutation | Release automation mutates tags/assets, skips proof, packages secrets, or omits integrity evidence | Publish a new immutable version from a green commit with artifact evidence |
| Disabled tests | New skipped/flaky/no-assertion test lands | Fix test or record reviewed quarantine with expiration |

## 4. Hard LOC Limits

Generated files are exempt only when listed in `agent/generated-zones.toml` and stamped with generator metadata.

| Artifact | Target | Hard max | Required refactor trigger |
|---|---:|---:|---|
| Rust domain file | 220 LOC | 350 LOC | Split by invariant, state machine, or value object |
| Rust application file | 250 LOC | 400 LOC | Split by command/use case |
| Rust adapter file | 250 LOC | 450 LOC | Split by external system or table |
| Rust function | 40 LOC | 70 LOC | Extract pure decision or port call |
| Rust test file | 350 LOC | 700 LOC | Split by behavior lane |
| TypeScript React component | 180 LOC | 280 LOC | Split view, state hook, generated client use |
| TypeScript hook/helper | 120 LOC | 220 LOC | Split by one responsibility |
| TypeScript route/page | 220 LOC | 350 LOC | Split loader, action, view, test fixture |
| TypeScript function | 35 LOC | 60 LOC | Extract named decision |
| Exception-only Python AI/data file | 180 LOC | 300 LOC | Split model IO, eval, transform, service boundary |
| Exception-only Python function | 35 LOC | 60 LOC | Extract typed pure step |
| SQL migration | 180 LOC | 350 LOC | Split into semantic migration steps |
| Markdown agent instruction | 100 lines | 180 lines | Move detail into linked topic docs |
| Markdown design doc | 300 lines | 600 lines | Split into decision, protocol, and reference docs |

Agents MUST check file length before editing. If a target file is already above hard max, the first change must be a refactor or an exception file.

## 5. Refactoring Rules

Refactor by ownership, not by convenience.

- Split by domain concept, use case, external adapter, UI surface, or contract.
- Do not create `common`, `misc`, `helpers`, `utils`, `shared`, or `legacy` junk drawers.
- Name extracted code after the behavior it owns, not the syntax it uses.
- Keep Rust domain pure: no IO, env, system time, random, network, database, filesystem, logging side effects, or framework types.
- Keep TypeScript product surface UI-focused: generated clients only, no durable truth.
- Keep Python exception-only: use it only for approved advanced ML/data library work, behind a typed API, with no product authorization, product truth, proof lane, repo tooling, general backend glue, or direct production DB writes.
- Replace inheritance ladders with enums, traits, composition, or explicit interfaces unless a framework requires inheritance.
- When duplicate logic appears a second time, create an owning module or generated contract.
- When a third call site appears, add table-driven tests or property tests for the owner.
- Every refactor must preserve or improve validation routing in `agent/test-map.json`.

## 6. Ideal Repo Layout

```text
repo/
  AGENTS.md
  README.md
  Justfile | Makefile | package.json scripts
  agent/
    JANKURAI_STANDARD.md
    owner-map.json
    test-map.json
    generated-zones.toml
    agent/repo-score.json
  apps/
    web/                 # TypeScript, React, Vite, generated clients only
    api/                 # Rust Axum/Tower HTTP or ConnectRPC edge
  crates/
    domain/              # pure invariants, IDs, state machines
    application/         # commands, authz, idempotency, transactions
    adapters/            # DB, queues, external APIs, filesystem, env
    workers/             # jobs, CPU work, durable workflow glue
  contracts/
    openapi/
    protobuf/
    json-schema/
    generated/
  db/
    migrations/
    constraints/
    seeds/
  python/
    ai-service/          # exception-only advanced ML/data; typed API; no product truth
  ops/
    ci/
    observability/
    security/
  docs/
    decisions/
    exceptions/
    runbooks/
```

Allowed variants require `docs/exceptions/<id>.md` with owner, reason, expiration, and migration plan.

## 7. Ownership Boundaries

| Layer | Owns | MUST NOT own |
|---|---|---|
| `apps/web` | UI, forms, local validation, rendering state, generated API clients, Playwright selectors | secrets, durable truth, core authz, direct DB writes, hand-authored API types |
| Optional BFF | session bridge, UI aggregation, feature flags, request shaping | business invariants, product truth, durable workflows |
| `apps/api` | HTTP/RPC edge, request extraction, response mapping, tracing boundary | domain rules, raw SQL business decisions, UI concerns |
| `crates/domain` | IDs, invariants, value objects, state machines, pure decisions | IO, env reads, time/random, framework types, DB types |
| `crates/application` | commands, authz, idempotency, transaction orchestration, port interfaces | UI, external protocol details, scattered SQL |
| `crates/adapters` | PostgreSQL, queues, external APIs, filesystem, environment, secret loading | domain rules, authorization policy |
| `crates/workers` | async jobs, backpressure, retries, durable workflow glue | product truth outside application layer |
| `contracts` | OpenAPI/protobuf/JSON Schema source and generated artifacts | handwritten drift from server/client |
| `db` | migrations, constraints, indexes, RLS, seeds, extension policy | ad hoc app-only invariants |
| `python/ai-service` | approved advanced ML/data library work, embeddings, eval pipelines, feature extraction, offline analysis | product truth, authz, direct prod DB writes, UI API ownership, repo tools, proof lanes, general backend glue |
| `ops` | CI, OTel, SBOM, SCA, secrets, deploy, provenance | hidden manual gates |
| `docs` | decisions, exceptions, runbooks, public standard | stale generated truth |
| `agent` | owner map, test map, generated zones, audit score, agent standard | prose-only policy with no machine check |

## 8. Dependency Direction

Allowed direction:

```text
apps/web -> contracts/generated
apps/api -> crates/application -> crates/domain
crates/application -> port traits
crates/adapters -> crates/application ports + crates/domain
crates/workers -> crates/application + crates/adapters
python/ai-service -> contracts/generated only
db -> migrations/constraints only
```

Forbidden direction:

- `crates/domain` importing adapters, HTTP, SQL, env, time, logging, metrics, or filesystem.
- `apps/web` importing SQL clients, secrets, database URLs, or product authorization internals.
- Python importing application database clients for production truth.
- Python added without a dated advanced-ML/data exception.
- Kafka, Tansu, Iggy, Fluvio, NATS, Redis Streams, or similar clients outside declared queue adapters.
- Adapters calling UI or product surface code.
- Generated code importing handwritten implementation code.

## 9. Generated Zones

Generated zones MUST be declared, stamped, and reproducible.

Required generated-file header:

```text
Generated by: <tool> <version>
Source: <contract path>
Command: <regen command>
DO NOT EDIT BY HAND.
```

Structured generated artifacts that cannot legally carry comment headers MUST
carry equivalent generated identity in their native format. For example,
`agent/repo-score.json` is valid when its schema URL, generated timestamp, and
standard/auditor/schema version fields are present. Native lockfiles such as
`package-lock.json` are valid when the package-manager lockfile shape validates.
Arbitrary JSON without recognized generated identity is still treated as an
unprotected generated-zone mutation.

Required zones:

- `contracts/generated/**`
- `apps/web/src/generated/**`
- `apps/api/src/generated/**` when used
- `crates/*/src/generated/**` when used
- `python/ai-service/generated/**` when used

Rules:

- Change source contracts, not generated outputs.
- CI must fail when generated output is stale.
- Public API changes must run contract tests and update clients.
- Agents must not infer types from runtime JSON when generated types exist.

## 10. Test Lanes

Every repo MUST expose these lanes, even if some are initially empty with explicit rationale.

| Lane | Purpose | Typical commands |
|---|---|---|
| `fast` | deterministic local proof under 2 minutes | `cargo test -p domain`, `pnpm test --run`, focused lint |
| `contract` | prove public API/schema compatibility | OpenAPI/protobuf checks, generated client diff, consumer tests |
| `db` | prove durable truth changes | migration apply/revert, constraint tests, query compile checks |
| `web` | prove UI behavior and rendered UX where possible | Vitest, Testing Library, Storybook, jankurai UX QA |
| `e2e` | prove critical user journeys | Playwright preferred for browser workflows |
| `security` | secrets, dependency, SAST, unsafe, licenses | secret scan, SCA, cargo audit, npm audit policy, SBOM |
| `observability` | request IDs, traces, structured error payloads | OTel smoke tests, log schema checks |
| `audit` | enforce this standard | jankurai audit JSON/MD/SARIF |
| `release` | full merge gate | all above, provenance, artifact build |

Path changes MUST route to lanes through `agent/test-map.json`. Agents MUST run the smallest mapped lane locally and report any skipped lane with reason.

## 11. Test Coverage Rules

Coverage means behavior proof, not line count.

- Domain invariants need unit tests and property/table tests.
- Application commands need authorization, idempotency, transaction, and failure tests.
- Adapters need contract/integration tests against real or faithful services.
- Database migrations need forward apply, rollback policy, constraint checks, and tenant isolation checks when multi-tenant.
- TypeScript UI needs component tests, rendered UX geometry checks, visual/a11y evidence, and Playwright for critical browser journeys.
- Rust TUI surfaces MAY use Tuiwright as positive rendered UX evidence when tests combine `Page::spawn` or `SpawnConfig` with at least one wait/assertion and, ideally, interaction or artifact signals. Audit consumes that evidence but does not run Tuiwright itself. Missing Tuiwright proof stays advisory unless a repository declares required TUI flows in an explicit manifest.
- Approved Python AI/data exceptions need golden evals, model IO contract tests, data-shape tests, reproducibility seeds, and a containment/migration plan.
- Bugs require regression tests in the owner cell that failed.
- Every external boundary needs success, validation failure, retryable failure, and permanent failure coverage.
- Snapshot tests are allowed only when paired with semantic assertions.
- Mocking own contracts is forbidden when generated contracts or test containers are available.
- Skipped tests require owner, reason, issue link, and expiration date.

Recommended browser lane: Playwright for end-to-end flows because it exercises real browser behavior, selectors, network boundaries, traces, screenshots, and videos. Use Testing Library for component behavior below the browser boundary.

## 11.1 Rendered UX And Browser-Step QA

Browser QA is first-class but risk-routed. Critical flows, auth, payments, admin actions, onboarding, canvas/3D surfaces, and layout-sensitive components SHOULD have Playwright traces or screenshots in the PR lane. Rendered UX proof SHOULD combine Storybook states, screenshots, ARIA snapshots, accessibility scans, CLS checks, generated mocks, design-token evidence, and DOM geometry rules for edge clearance, target size, overlap, clipping, wrapping, overflow, sticky obstruction, focus visibility, form labels, and nested scrollbars. Full viewport/device matrices MAY run nightly or at release unless the changed path directly touches those surfaces.
Rust TUI proof MAY use Tuiwright evidence instead of browser proof when the surface is terminal-native: a flow counts only when a real Rust test uses `Page::spawn` or `SpawnConfig` plus an assertion or wait, and screenshots, GIFs, and traces are supporting artifacts rather than proof by themselves. The audit reads that evidence but does not execute Tuiwright during repo scoring.

Required evidence for high-risk UI repairs:

- route or story ID
- viewport and browser
- action sequence
- assertion
- screenshot, crop, ARIA snapshot, trace, or video artifact
- rule ID, selector, owner, and merge decision
- changed files and proof lane

Agents MUST NOT treat a passing typecheck as proof of visual correctness for changed critical UI flows.
Deterministic rendered-UX violations block. Pixel diffs route to baseline review. AI/CV opinions route to humans unless backed by deterministic evidence.

## 12. Agent-Friendly Exception Contract

Exceptions are a knowledge-pooling surface. They must teach agents how to repair failures.

Every controlled exception/error crossing a boundary MUST carry:

- `name`: stable machine-readable exception name.
- `purpose`: what invariant or boundary it protects.
- `reason_code`: stable enum/string for routing.
- `message`: concise human-readable message with no secrets.
- `common_fixes`: ordered fixes an agent can attempt.
- `docs_url`: direct link to repo documentation.
- `owner`: owning path/team.
- `retryable`: true/false.
- `severity`: `debug`, `info`, `warn`, `error`, `fatal`.
- `correlation_id`: trace/request ID.
- `source`: component and operation.
- `contract_version`: relevant API/schema version.

Rust pattern:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("invalid order transition: {from:?} -> {to:?}")]
    InvalidOrderTransition { from: OrderState, to: OrderState },
}

impl DomainError {
    pub fn reason_code(&self) -> &'static str { "ORDER_INVALID_TRANSITION" }
    pub fn docs_url(&self) -> &'static str { "docs/runbooks/orders.md#invalid-transition" }
    pub fn common_fixes(&self) -> &'static [&'static str] {
        &["load current order state", "call allowed transition command", "add missing state migration"]
    }
}
```

TypeScript pattern:

```ts
export class AgentFriendlyError extends Error {
  constructor(readonly info: {
    name: string;
    purpose: string;
    reasonCode: string;
    commonFixes: string[];
    docsUrl: string;
    retryable: boolean;
    correlationId?: string;
  }) {
    super(info.reasonCode);
  }
}
```

Python exception boundary:

Do not add Python as a pattern for repo tooling, proof lanes, product services,
or backend glue. If a dated advanced-ML/data exception already exists, its
service boundary must expose the same structured fields through generated
contracts and must not leak raw provider exceptions across the product boundary.

Forbidden:

- string-only errors at service boundaries
- catch-all that drops reason
- `panic`, `unwrap`, or `expect` in production paths without documented invariant
- Python bare `except`
- TypeScript `throw "message"`
- HTTP 500 without stable error payload
- errors that recommend "try again" when not retryable

## 13. CI Audit Requirements

CI MUST run jankurai audit on every pull request and default branch push for repositories claiming `HL3` or higher. Lower levels may run in advisory mode while they build the required controls.

Minimum outputs:

- `agent/repo-score.json`
- markdown summary attached to CI job
- PR comment or check annotation for high findings
- optional SARIF for code scanning; SARIF is a planned output format for the v0.x auditor line

CI modes:

| Mode | Merge behavior |
|---|---|
| advisory | emit JSON/Markdown, never block |
| guarded | block critical caps and malformed output |
| standard | block high/critical findings plus score-floor failure |
| ratchet | prevent score regression without dated exception |
| release | gate shipped artifacts on audit, tests, security, contracts, DB, e2e, and versions |

Minimum `standard` gates:

| Condition | Action |
|---|---|
| score below `85` | fail PR |
| any hard cap below `80` | fail PR |
| high severity finding | fail PR unless exception exists |
| generated drift | fail PR |
| missing fast lane | fail PR |
| missing security lane on high-risk repo | fail PR |
| standard version behind latest minor by more than 30 days | warn |
| standard version behind latest major | fail until migration plan exists |

Audit findings MUST include:

- rule_id when stable
- severity
- category
- path
- line when available
- problem
- evidence
- agent_fix
- owner
- validation lane
- docs link

Audit JSON MUST include `standard_version`, `auditor_version`, `schema_version`, `paper_edition`, `target_stack_id`, raw score, final score, hard caps, dimension breakdown, `profile_structure`, findings, and ordered `agent_fix_queue`.

The Markdown report MUST include a `## Reference Profile Structure` section that summarizes detected cells, canonical folders, local guidance status, and migration steering.

Plotting integrations that want rolling score plots MUST use the bounded history export command; `jankurai score trend` remains the summary command. Do not scrape full audit JSON for trend plots.

Operational receipts from `doctor`, `init`, and future phase closeouts should live under `target/jankurai/receipts/<action>-<unix-seconds>.json`. Keep them volatile and cite them in release notes or phase receipts instead of promoting them into tracked source.

## 14. Vibe-Coding Failure Catalog

The audit MUST detect or require explicit exceptions for these problems.

`TLR` means Top-Level Risk. Repair priority MUST account for TLR, not only finding count. Security, business truth, and contract/data-truth findings outrank easier style repairs.

| TLR | Hard examples | Required evidence |
|---|---|---|
| Security, secrets, agency | generated insecure code, committed secret, prompt injection, overbroad tool permission, missing scan | security lane output, secret scan, permission receipt, threat-model note |
| Business truth | false-green business rule, authorization drift, data-isolation drift | domain/application tests, role matrix, negative cases, DB constraint/RLS where useful |
| Contracts and data truth | handwritten DTO, generated mutation, direct DB wrong layer, app-only invariant | generated contract diff, owner map, migration/constraint proof |
| Verification and rendered UX | missing proof lane, shallow tests, pixel/accessibility gap | test-map lane, semantic assertions, screenshot/trace/geometry/a11y evidence |
| Context and setup | setup hallucination, context retrieval failure, instruction drift | one-command setup, short root router, local rules, filtered command evidence |
| Maintainability entropy | dead markers, fallback soup, mega functions/files, perf/concurrency drift | exception record, bounded retry policy, LOC split, benchmark/trace proof |

| Failure | Hard rule |
|---|---|
| God file | Non-generated file exceeds LOC hard max |
| Mega function | Function exceeds LOC hard max |
| Junk drawer | `utils`, `helpers`, `common`, `misc`, `legacy`, `stuff`, `shared` without owner README |
| Duplicate behavior | same validation, mapping, authz, query, or transform in multiple cells |
| Handwritten API type | frontend or exception-only Python declares types that should come from contract generation |
| Handwritten client drift | fetch/axios client duplicates generated client behavior |
| Silent fallback | fallback value hides unavailable dependency, bad schema, auth failure, or model failure |
| Broad catch | catch-all without reason_code, retry policy, and docs link |
| Uncontrolled retry | retry without budget, backoff, cancellation, and idempotency |
| Dead TODO | TODO/FIXME/HACK without owner and expiration |
| Disabled test | skip/only/quarantine without issue and expiration |
| No assertion test | test executes code but proves no behavior |
| Snapshot abuse | snapshot without semantic assertion |
| False-green business logic | code and tests pass while the product invariant is wrong |
| Security flaw in generated code | generated auth, input handling, crypto, deserialization, filesystem, or logging change lacks security proof |
| Prompt injection | untrusted context changes trusted instructions, tool calls, or policy |
| Overbroad agent agency | terminal/browser/network/filesystem permission exceeds lane scope |
| Any sprawl | TypeScript `any`, `@ts-ignore`, unchecked JSON, or loose mode without exception |
| Unsafe sprawl | Rust `unsafe`, `unwrap`, `expect`, or `panic` in production path without ledger |
| Python creep | Python outside a dated advanced-ML/data exception under `python/ai-service`, or Python owning product APIs/truth |
| Notebook in prod | notebook checked into production path or CI path without export policy |
| Direct DB in UI | browser code, BFF, or exception-only Python owns direct durable writes |
| Domain IO | Rust domain reads env, clock, random, DB, filesystem, network, logger, metrics |
| App-only invariant | database lacks constraint for durable invariant |
| Migration hazard | destructive migration lacks rollback, lock, data backfill, and review note |
| Secret risk | committed secret, broad env dump, missing secret scan |
| Dependency spray | new dependency without rationale, owner, license, and security review |
| Supply-chain drift | dependency, action, image, package, or provenance change lacks scan evidence |
| Multiple package managers | lockfiles conflict without documented reason |
| Unpinned action/image | CI action, Docker base, or install script unpinned where policy requires pinning |
| Observability gap | no request ID, trace ID, structured error, or operation name across boundary |
| Rendered UX gap | critical UI change lacks screenshot, trace, geometry, visual baseline, or accessibility proof |
| Pixel baseline drift | screenshot changed without owner-approved baseline decision and artifact receipt |
| AI visual false authority | model/VLM says a UI is acceptable without deterministic geometry, a11y, or baseline evidence |
| Setup gap | repository lacks deterministic setup or local service proof |
| Context bloat | root agent docs too long, duplicated policy, pasted logs, or generated docs in prompt path |
| Orphan code | file not reachable from owner-map, build, tests, docs, or import graph |
| Naming fog | vague names like `manager`, `processor`, `handler`, `data`, `new2`, `final`, `temp` |
| Stale docs | public behavior changed without docs/runbook/decision update |

## 15. Naming And Documentation Rules

Names MUST communicate ownership and behavior.

- Use domain nouns and verbs: `OrderTransition`, `AuthorizeRefund`, `InvoicePosted`.
- Avoid vague suffixes unless framework-required: `Manager`, `Processor`, `Handler`, `Util`, `Helper`, `Common`.
- API names MUST match contract names.
- Database constraints MUST be named after invariant and table.
- Error names MUST match reason codes.
- Tests MUST name behavior and expected result.
- Documentation files MUST include `Status`, `Owner`, `Last reviewed`, and `Applies to`.
- Decision docs live in `docs/decisions/NNNN-title.md`.
- Exceptions live in `docs/exceptions/NNNN-title.md` and must expire.
- Runbooks live in `docs/runbooks/<system>.md`.

## 16. Token Minimization Rules

Agent context is a budget.

- Root `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, Cursor rules, and Copilot instructions should be short pointers, not full manuals.
- Put full policy in `docs/agent-native-standard.md`.
- Put quick boot policy in `agent/JANKURAI_STANDARD.md`.
- Use `agent/owner-map.json` and `agent/test-map.json` instead of prose path descriptions.
- Prefer `rg`, symbol search, generated maps, and targeted reads.
- Do not paste full logs into prompts. Save logs under ignored artifacts and quote relevant lines.
- Use filtered command wrappers such as `rtk` where available and safe.
- Exclude build outputs, vendored dependencies, generated outputs, and large artifacts from agent context with tool-specific ignore files.
- Keep generated docs out of always-loaded instruction files.
- Use path-scoped rules for specialized areas.
- Optional compressed speech modes are allowed for human-agent chat only when the team agrees; do not encode project policy in novelty dialects.

## 17. Tool-Specific Instructions

Tool behavior changes. Each repo MUST keep a short adapter for each tool it uses and point back to this standard.

| Tool | Required repo artifact | Rules |
|---|---|---|
| Codex | `AGENTS.md` | Codex reads `AGENTS.md` files by scope. Keep root short, place local overrides near specialized code, and ask Codex to report loaded instructions when debugging. |
| Cursor | `.cursor/rules/*.mdc` or `.cursor/rules/*` plus optional `AGENTS.md` | Use project rules for versioned repo policy. Prefer small scoped rules. Do not rely on deprecated `.cursorrules` except as migration shim. |
| Claude Code | `CLAUDE.md` or `.claude/CLAUDE.md` | Import `AGENTS.md` or `agent/JANKURAI_STANDARD.md`; keep under 200 lines; use `.claude/rules/` for path-specific rules; use `/memory` to inspect loaded context. |
| Gemini CLI | `GEMINI.md` | Use `@agent/JANKURAI_STANDARD.md` import where supported; use `/memory show`, `/memory list`, and `/memory refresh` to verify loaded context; configure context filenames if the team standardizes on `AGENTS.md`. |
| Antigravity | verified current rule file for installed version | Treat loading rules as version-sensitive. Prefer shared `AGENTS.md`/`GEMINI.md` pointer when supported. Disable unattended terminal/browser actions for untrusted repos. Require checkpoints before writes. |
| GitHub Copilot | `.github/copilot-instructions.md` and optional `.github/instructions/*.instructions.md` | Put the critical rules in the first 4,000 characters for code review compatibility. Use path-specific instruction files for detailed rules. Keep statements short and self-contained. |

Universal tool boot prompt:

```text
Read agent/JANKURAI_STANDARD.md and docs/agent-native-standard.md.
Identify owner-map and test-map entries for the requested paths.
Do not edit generated files by hand.
Before adding behavior, check file/function LOC limits.
After edits, run the mapped fast lane and report skipped lanes.
```

## 18. Version Update Policy

This standard changes quickly. Repositories MUST track standard version explicitly.

Required:

- `agent/standard-version.toml` is the canonical manifest.
- `agent/JANKURAI_STANDARD.md` includes `Standard version`.
- `jankurai version` is the direct local check for installed CLI diagnostics
  after install or version bumps.
- `jankurai versions` is the direct local check for manifest bindings after install or version bumps.
- CI audit reads the version.
- Repo pins the standard source URL or vendored commit when available.
- `docs/decisions/` records adoption decision.
- Standard upgrades are reviewed like dependency upgrades.
- `jankurai upgrade` is write-capable; use `jankurai upgrade --score` to run the post-upgrade scoring lane.

Required artifact bindings for this workspace:

| Artifact | Binding |
|---|---|
| `paper/jankurai.tex` | `paper-source`, version `paper_edition` |
| `paper/jankurai.pdf` | `paper-render`, generated by `just paper` |
| `paper/jankurai.md` | `paper-agent-md`, companion only |
| `docs/agent-native-standard.md` | `coding-standard`, version `standard_version` |
| `agent/JANKURAI_STANDARD.md` | `agent-standard-brief`, source standard doc |

Version rules:

| Version change | Meaning | Repo action |
|---|---|---|
| patch | wording, clearer examples, non-breaking checks | adopt within 30 days |
| minor | new recommended checks or stricter warnings | adopt within 60 days or file exception |
| major | new hard gates or layout contract | create migration plan before adoption |

CI update check:

- warn when patch/minor update exists.
- fail when major update exists and no migration issue is linked after 30 days.
- never auto-apply standard updates without review.

## 19. Agent Start Checklist

Before editing:

- Read `agent/JANKURAI_STANDARD.md`.
- Identify changed paths.
- Look up owner-map and test-map.
- Check file and function LOC.
- Check generated-zone status.
- Search for duplicate existing behavior.
- Confirm boundary owner.

During editing:

- Keep change in one owner cell where possible.
- Prefer contract/source edits over generated edits.
- Add or update tests in mapped lane.
- Emit agent-friendly exceptions at boundaries.
- Update docs only where behavior changes.

Before final response:

- Run mapped validation.
- Run audit when standard-sensitive files changed.
- Report score/failures/lanes.
- List exceptions created or used.

## 20. Sources For Tool Loading Rules

- Codex AGENTS.md: `https://developers.openai.com/codex/guides/agents-md`
- Cursor rules: `https://docs.cursor.com/en/context/rules`
- Claude Code memory and CLAUDE.md: `https://code.claude.com/docs/en/memory`
- Gemini CLI GEMINI.md: `https://google-gemini.github.io/gemini-cli/docs/cli/gemini-md.html`
- GitHub Copilot custom instructions: `https://docs.github.com/en/copilot/concepts/prompting/response-customization`
