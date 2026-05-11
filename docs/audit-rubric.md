# jankurai Audit Rubric

Version: `0.8.0`

Target stack: Rust core + TypeScript/React/Vite product surface + PostgreSQL truth + generated contracts + exception-only Python AI/data service.

The audit is strict on purpose. It is not a general-purpose repo quality score. It asks one question: can an agent safely reject, localize, prove, audit, and repair this codebase without turning human-friendly shortcuts into production behavior?

## Required Shape

| Surface | Allowed Role | Hard Boundary |
| --- | --- | --- |
| `apps/web/` | TypeScript, React, Vite, generated API clients, UI state, forms, local validation | no secrets, no direct DB, no handwritten DTO drift, no durable truth |
| `apps/api/` | Rust HTTP/RPC edge, request decoding, response encoding, auth/session bridge | no raw SQL in handlers, no domain rules hidden in framework code |
| `crates/domain/` | pure IDs, invariants, state machines, decisions, typed errors | no I/O, env, DB, HTTP, filesystem, queues, logging side effects |
| `crates/application/` | commands, authz, idempotency, transactions, workflow orchestration | no UI concerns, no scattered raw SQL, no provider-specific details |
| `crates/adapters/` | PostgreSQL, queues, external APIs, filesystem, env, providers | no domain rules |
| `crates/workers/` | async jobs, durable workflow glue, CPU workers | no UI truth, no bypass of application/domain invariants |
| `contracts/` | OpenAPI, protobuf, JSON Schema, generated contract outputs | generated outputs must be marked and repaired from source contracts |
| `db/` | migrations, constraints, seeds, indexes, RLS where useful | no ad hoc app-only durable invariants |
| `python/ai-service/` | rare approved advanced ML/data library work, embeddings, evals, typed model/data API | no product truth, no core authz, no repo tools, no proof lanes, no general backend glue, no direct production DB ownership |
| `ops/` | CI, observability, security, provenance, deployment | no hidden manual gates |

## Score Dimensions

| Dimension | Weight | What Good Looks Like |
| --- | ---: | --- |
| Ownership and navigation surface | 14 | root `AGENTS.md`, local routing docs, owner map, test map, short navigation |
| Contract and boundary integrity | 14 | generated clients, checked API drift, strict TypeScript, Rust typed boundaries |
| Proof lanes and test routing | 14 | one-command validation, deterministic fast lane, CI audit lane, rendered UX, e2e/property/integration tests |
| Security and supply-chain posture | 14 | lockfiles, secret scanning, dependency review, SBOM/provenance, workflow linting |
| Code shape and semantic surface | 12 | small files/functions, low duplication, no placeholder/fallback behavior, specific names |
| Data truth and workflow safety | 8 | migrations, constraints, DB isolated to adapters/db, no DB writes from wrong layers |
| Observability and repair evidence | 8 | tracing, request IDs, structured diagnostics, repair receipts, agent-friendly exceptions |
| Context economy and agent instructions | 8 | concise docs, generated zones, root router, evidence paths, no token-heavy maze |
| Python containment and polyglot hygiene | 4 | Python only in rare dated advanced-ML/data exceptions or explicit detector fixtures, no Python tooling or unnecessary runtime languages |
| Build speed signals | 4 | fast checks, caching, nextest/vitest, targeted commands, locked dependencies |

## Hard Rule Caps

| Rule | Max Score | Agent Repair |
| --- | ---: | --- |
| no root agent/developer instructions | 75 | add concise root `AGENTS.md` and route deeper docs locally |
| no one-command setup or validation | 70 | add canonical `setup`, `check`, `test`, or `verify` command |
| no deterministic fast lane | 65 | add the narrowest repeatable proof loop for changed files |
| high-risk repo with no security lane | 60 | add secret scan, dependency review, SBOM/provenance, workflow lint |
| generated contracts or public API drift untested | 80 | generate clients and gate drift in CI |
| Python owns product truth or DB ownership | 72 | move truth/authz/workflows into Rust and DB migrations |
| no secret or dependency scan in CI | 78 | add gitleaks/detect-secrets plus dependency review or equivalent |
| no jankurai audit lane in CI | 82 | run `jankurai` in every PR and publish JSON/Markdown |
| non-optimal product language found | 74 | migrate product runtime code to Rust, TypeScript, SQL, or generated contracts |
| too much Python in product surface | 72 | remove Python or box a rare approved advanced-ML/data exception under `python/ai-service` and move durable behavior to Rust |
| vibe placeholders in product code | 68 | replace TODO/stub/unimplemented/unreachable with real behavior or typed exceptions |
| fallback soup in product code | 70 | replace fallback chains with explicit states, bounded retries, telemetry, docs |
| future-hostile/dead-language in product runtime code | 64 | remove or rename dead/temporary/legacy wording, implement the state, or move copy/docs/generated/vendor text into an allowlisted context |
| severe duplication in product code | 70 | extract one named boundary and test it before editing behavior |
| generated zone mutation risk | 76 | add generated zone manifest and repair generated files from source contracts |
| direct DB access from wrong layer | 66 | move SQL and DB clients to `crates/adapters` or `db/` |
| missing web e2e lane | 82 | add Playwright or equivalent e2e tests for critical user flows |
| missing rendered UX QA lane | 84 | add Storybook states, Playwright screenshots, visual review or `@jankurai/ux-qa`, a11y, CLS, MSW, and design-token evidence |
| prompt injection risk in trusted agent/tool policy | 78 | isolate untrusted content, remove bypass wording, and validate tool calls |
| overbroad agent agency | 65 | replace broad permissions with least-privilege lane profiles and approvals |
| secret-like content detected | 60 | remove and rotate credential material, then add scanners and transcript/artifact review |
| false-green test risk | 76 | replace skipped/focused/tautological/snapshot-only proof with behavior assertions and red/green evidence |
| destructive migration risk | 70 | add rollback/backfill/lock-timeout/staged-deploy evidence and DB proof lane |
| authz or data-isolation gap | 78 | add owner/non-owner authorization tests, RLS evidence, or role-matrix proof |
| input-boundary gap | 78 | replace unsafe sinks with schemas, parameterization, allowlists, sandboxing, and negative tests |
| agent tool supply-chain gap | 78 | pin and review MCP/tool/hook/rule files and keep untrusted output out of trusted policy |
| release-readiness gap | 80 | attach security, backup, monitoring, rollback, and abuse-control launch evidence |
| cost-budget gap | 82 | add budgets, quotas, spend alerts, max tool-call limits, stop conditions, and kill switches |
| human-review evidence gap | 84 | attach raw CI logs, review receipts, and replayable commands for review/proof claims |
| missing Rust property/integration tests | 82 | add invariant/property tests plus integration tests through cargo test/nextest |
| no agent-friendly exception pattern | 76 | add typed errors with code, purpose, reason, common fixes, docs URL |
| missing agent-readable docs | 80 | add concise architecture, boundary, testing, and audit docs |
| rust-bad-behavior | 72 | detector-backed Rust bad-behavior findings only; keep proof-gated and high-confidence |
| sql-bad-behavior | detector-backed | SQL detector pack with hard and advisory signals |
| typescript-bad-behavior | detector-backed | TypeScript detector pack with hard and advisory signals |
| docker-bad-behavior | detector-backed | Docker detector pack with hard and advisory signals |
| release-bad-behavior | detector-backed | Release detector pack for mutable tags/assets, skipped proof, mutable latest-only artifacts, secret-bearing packages, and missing integrity evidence |
| web-security-bad-behavior | 68 | high-confidence web security findings for exposed Vite dev servers, client-exposed secrets, browser token storage, and credentialed wildcard CORS |
| repo-rot-bad-behavior | 88 | soft cap for active source that looks old, backed up, copied, parked, fake-versioned, or otherwise ambiguous without owner/proof/expiry |
| python-bad-behavior | detector-backed | Python detector pack with hard and advisory signals |
| ci-bad-behavior | detector-backed | CI detector pack with hard and advisory signals |
| git-bad-behavior | detector-backed | Git detector pack with hard and advisory signals |
| gittools-bad-behavior | detector-backed | Git tooling detector pack with hard and advisory signals |

## Known Vibe-Coding Insults

These are hard repair signals, not style nits.

## Top-Level Risk Mapping

`TLR` means Top-Level Risk. The audit prioritizes findings by TLR before count, because a single authz or secret failure matters more than several style findings.

| TLR | Hard findings | Soft findings |
| --- | --- | --- |
| Security, secrets, agency | generated code touches auth/input/crypto/filesystem without security proof; secret-like value; missing scan; overbroad terminal/browser/network permission | missing threat-model note, weak redaction evidence, broad env access, new dependency without rationale |
| Business truth | false-green domain behavior; authz/data isolation in UI/API/exception-only Python; app-only durable invariant | missing role matrix, missing negative test, unclear owner of invariant |
| Contracts and data truth | handwritten DTO/client; generated mutation; direct DB from wrong layer; missing generated-zone source | contract docs stale, generated-zone metadata incomplete |
| Verification and rendered UX | missing proof lane; disabled/no-assertion/snapshot-only test; no rendered UX proof for critical UI | weak visual baseline governance, missing edge fixtures, missing accessibility expert review |
| Context and setup | missing one-command setup; owner/test map gap; contradictory agent instructions | root docs too long, noisy command output, stale local guidance |
| Maintainability entropy | dead markers, fallback soup, mega functions/files, uncontrolled retries | weak names, performance/cost risk without budget |

Vibe coverage adds detector-backed stable rule IDs
`HLT-022-AUTHZ-ISOLATION-GAP`, `HLT-023-INPUT-BOUNDARY-GAP`,
`HLT-024-AGENT-TOOL-SUPPLY-GAP`, `HLT-025-RELEASE-READINESS-GAP`,
`HLT-026-COST-BUDGET-GAP`, and
`HLT-027-HUMAN-REVIEW-EVIDENCE-GAP`. These rules are coverage labels for
source-row reporting; rows are marked `detector-backed`, `partial`, or `none` based on
whether Jankurai has deterministic detector evidence, proof-lane evidence, and
CI/report artifacts.

| Insult | Why It Fails Agent-Native Engineering | Required Repair |
| --- | --- | --- |
| duplicated logic | agents patch one copy and miss another | extract one owned module and add tests |
| fallback soup | behavior becomes probabilistic and unreviewable | model explicit states and bounded retry policy |
| future-hostile/dead language | `legacy`, `deprecated`, `old`, `temporary`, `workaround`, `shim`, `fallback`, `TODO`, and similar markers train agents to preserve abandoned paths | delete, rename, implement, or move quoted product copy/docs/generated/vendor text into an allowlisted context |
| TODO/FIXME/HACK/XXX | placeholder intent becomes shipped behavior | implement or create typed unsupported-state exception |
| stub/placeholder/not implemented | fake completeness blocks proof | delete, implement, or gate behind explicit exception |
| `unreachable!`, `unimplemented!`, TODO panics | runtime surprise hidden from proof lanes | replace with typed errors and tests |
| handwritten DTOs | frontend/backend drift silently | generate from OpenAPI/protobuf/JSON Schema |
| handwritten fetch wrappers | every endpoint becomes a local contract fork | generate API client and keep one transport wrapper |
| direct DB from UI/API/domain/application | durable truth leaks into wrong layer | isolate DB in `crates/adapters` and `db/` |
| Python product truth | dynamic runtime owns durable business state | move truth/authz/workflows into Rust/PostgreSQL |
| unnecessary runtime languages | more syntax, tooling, locks, and failure modes | converge product runtime to the target stack |
| mega files | agents lose locality and reviewers lose ownership | split before 500 LOC, prefer under 300 LOC |
| mega functions | behavior cannot be named, tested, or localized | keep under 80 LOC by default |
| weak names | ownership and intent are hidden | use domain verbs/nouns, not `utils`, `helpers`, `manager`, `common` |
| missing docs | agents infer policy from code accidents | add short routed docs and local ownership files |
| missing audit CI lane | rules are advisory instead of enforced | run audit in every PR |
| mutated generated zones | generated code becomes forked source | edit source contract, regenerate, verify |
| no e2e web proof | UI regressions depend on human clicking | add Playwright critical-path tests |
| no rendered UX proof | cramped, clipped, overlapping, unstable, or inaccessible UI still depends on taste review | add Storybook, screenshots, visual review, accessibility, CLS, generated mocks, tokens, and geometry checks |
| no Rust property tests | invariants are example-only | add `proptest`/equivalent invariant tests |
| no Rust integration tests | cross-crate behavior is unproved | add tests under crate or workspace `tests/` |
| no security scan | AI-churned dependencies and secrets slip through | run secret/dependency/provenance gates |
| generated insecure code | plausible code hides injection, XSS, unsafe deserialization, weak crypto, or bad logging | run security lane, add negative tests, attach threat-model evidence |
| improper AI/tool output handling | generated text is parsed, rendered, or executed as trusted command/data | validate schemas, encode output, sandbox commands, and require tool-call receipts |
| prompt injection / hostile context | untrusted issue, doc, page, or tool output can override trusted policy | enforce source hierarchy, isolate untrusted context, validate tool calls |
| overbroad agent agency | broad terminal/browser/network/file permissions make unsafe actions easy | use least-privilege permission profiles and approval gates |
| customer-data or PII leakage | prompts, logs, vectors, screenshots, or transcripts retain user data beyond policy | classify data, redact artifacts, limit retention, and scan transcripts/vector stores |
| model/prompt/eval drift | model, prompt, provider, embedding, or eval data changes without replay evidence | version prompts/models and run golden evals before merge |
| destructive migration / data-loss hazard | generated SQL drops data or blocks production without safety proof | require rollback/down plan, backfill strategy, lock timeout, and DB rehearsal |
| idempotency or side-effect duplication | generated retries/jobs/handlers double-charge, replay, or duplicate external side effects | require idempotency keys, replay tests, and workflow receipts |
| setup hallucination | unclear setup leads agents to install random tools or skip service proof | add deterministic setup and setup proof lane |
| context retrieval failure | agents patch nearby stale patterns instead of owner code | keep root guidance short, route through owner/test maps, filter command output |
| orphan/dead reachable code | old paths remain executable and agents preserve them as product truth | prove replacement reachability, delete or isolate, and add owner-signed exception |
| opaque exceptions | failures tell humans too little and agents nothing | standardize agent-friendly exceptions |
| console/println debugging | production evidence is unstructured | use tracing, request IDs, and structured logs |
| junk drawer folders | every patch becomes global search | replace with owned domain/adapters modules |

## Future-Hostile Language Rule

Product/runtime code must not contain future-hostile or dead-language markers such as `legacy`, `deprecated`, `depricated`, `obsolete`, `old`, `temporary`, `temp`, `workaround`, `shim`, `compat`, `backcompat`, `fallback`, `best effort`, `cleanup later`, `remove later`, `dead code`, `unused`, `stale`, `hack`, `todo`, `fixme`, `placeholder`, `stub`, or `dummy`.

The rule is strict because these words encode uncertainty as production behavior. An agent should not infer whether a `legacy` branch is still required, whether a `temporary` path can be removed, or whether a `fallback` is intentional policy.

Allowlisted contexts are path-based and must be obvious: documentation, reference material, generated files, vendor code, or explicitly named product-copy surfaces such as `product-copy`, `copydeck`, `i18n`, `locales`, or `translations`. The allowlist is not a comment escape hatch. If the file owns runtime behavior, repair the term by naming the real state, implementing the behavior, or raising a typed unsupported-state exception.

Every finding for this rule must include `path`, `line`, `matched_term`, `reason`, and `agent_fix` so agents can patch exact evidence without broad searching.

Non-policy prose files such as `*.md`, `*.tex`, and `*.txt` are word-neutral for lexical cleanup and prose-only audit signals. Trusted policy and control-plane surfaces such as `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `agent/`, `.agents/`, `.github/`, `.cursor/`, and `.claude/` remain scanned.

## Agent-Friendly Exceptions

Every controlled error that can reach logs, API responses, background jobs, or tests should expose:

| Field | Required Meaning |
| --- | --- |
| `name` | stable exception or error name |
| `code` | stable machine-readable code |
| `purpose` | what invariant or boundary this error protects |
| `reason` | why this instance failed |
| `common_fixes` | concrete repair candidates for agents and humans |
| `docs_url` | direct local or public documentation link |
| `source` | underlying provider/system error, when safe |
| `correlation_id` | trace/request/job id for production repair |

Rust should prefer enum error types with `thiserror` or equivalent plus structured diagnostic fields. TypeScript should mirror boundary errors with `Error` subclasses or discriminated result unions. Approved Python AI/data exceptions should return typed API errors, not raw provider exceptions.

## Test Standard

| Layer | Minimum Proof |
| --- | --- |
| Rust domain | unit tests plus property tests for invariants/state machines |
| Rust application | integration tests for authz, idempotency, transactions, workflows |
| Rust adapters | DB integration tests, migration tests, external API contract tests or fakes |
| TypeScript web | unit/component tests for pure UI logic plus rendered UX QA and Playwright e2e critical paths |
| Contracts | generation test, drift check, schema compatibility check |
| PostgreSQL | migration apply/rollback where possible, constraint tests, seed validation |
| Exception-only Python AI/data | eval tests, contract tests, no product-truth tests that imply ownership |
| Ops/security | secret scan, dependency/SBOM scan, workflow lint, audit scorer in CI |

## CI Contract

Every repository adopting this standard should run:

```bash
cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md
```

The JSON is the machine contract. The Markdown is the review surface. CI should upload both artifacts and fail when score or hard-cap policy crosses the team threshold.

The output must include:

| Field | Purpose |
| --- | --- |
| `standard_version` | lets repos track standard upgrades |
| `auditor_version` | identifies scanner implementation release |
| `schema_version` | protects JSON/Markdown output compatibility |
| `paper_edition` | binds findings to the paper edition that described the policy |
| `target_stack_id` | stable machine ID for the target stack |
| `target_stack` | prevents generic scoring drift |
| `score` and `raw_score` | final capped score plus weighted score |
| `caps_applied` | hard rule failures |
| `dimensions` | weighted breakdown |
| `profile_structure` | detected reference-profile cells, canonical paths, and migration steering |
| `findings` | actionable evidence with path, line, matched term, reason, problem, and repair |
| `agent_fix_queue` | ordered repair work for coding agents |
| `ux_qa` | rendered UX QA evidence, missing categories, and geometry-runtime readiness |

The Markdown report includes a `## Reference Profile Structure` section that summarizes detected cells, canonical folders, local guidance status, and migration steering.

## Versioning

The audit script, paper, and agent-facing artifacts must version together. Repos should record the jankurai standard version they target and schedule regular checks for newer releases. Breaking audit changes should include migration notes and example repairs.
