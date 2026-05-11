# Agent-First Code Design Public Reference Study

Date: 2026-05-01
Scope: public references for agent-first code design, repo setup, validation, auditability, and tool-specific instructions.
Primary rule: prefer official docs, project repositories, standards, and papers. Treat Reddit/X/community posts as low-weight sentiment only.

Current workspace policy is Rust-first: agents must not add Python for repo
tools, proof lanes, product services, authorization, direct PostgreSQL writes, or
backend glue. Python appears only as a rare dated advanced-ML/data exception
under `python/ai-service`, or as research/background material in this file.

## Executive Findings

1. `AGENTS.md` is the best neutral repo instruction file. Official and project sources now describe it as a plain Markdown, agent-focused companion to README, with nested files for monorepos and closest-file precedence.
2. Tool-specific rule files still matter. Cursor, Claude Code, Gemini CLI, GitHub Copilot, and Jules all have their own memory/rule/instruction surfaces. The jankurai standard should generate these from one canonical `AGENTS.md`/`agent/` source, not hand-maintain divergent copies.
3. Agent-first repos need deterministic control surfaces more than prose. Strong sources converge on setup scripts, test commands, scoped instructions, generated contracts, security checks, and reproducible environments.
4. Benchmarks show two core bottlenecks: setup reliability and context retrieval. SetupBench reports agents still struggle to bootstrap real environments. ContextBench reports large gaps between explored and useful context. This supports one-command setup, fast lanes, repo maps, owner maps, and token-filtered command output.
5. The optimal repo shape for the paper's winning stack is a Rust core, TypeScript/React/Vite product surface, PostgreSQL truth, generated contracts, and exception-only Python AI/data service. Agent-first rules should enforce ownership, generated zones, import boundaries, and a rare advanced-ML/data exception process as CI policy.
6. Security gates must run before trust. Official GitHub secret scanning, OpenSSF Scorecard, OSV-Scanner, SLSA provenance, and OpenTelemetry all support the audit thesis: generated code needs secrets, dependency, provenance, and production traceability checks.
7. Playwright is the default browser QA source for this stack. Its official best practices align with agent-friendly testing: user-visible behavior, isolation, locators, web-first assertions, cross-browser projects, and no hard sleeps.
8. Agent-friendly exceptions should be a standard. Combine RFC 9457 problem details, OpenTelemetry exception attributes, language-native error causes/context, and doc-linked error codes so agents can localize, classify, and repair failures.
9. Token minimization is a repo design problem. Public support comes from scoped rules, hierarchical memories, instruction size limits, context benchmarks, and codebase maps. RTK-like filtered command wrappers are a local implementation of this broader practice.

## Recommended Agent-First Repo Contract

Canonical source:

```text
AGENTS.md
agent/
  standard-version.toml
  owner-map.json
  test-map.json
  proof-lanes.toml
  generated-zones.toml
  audit-policy.toml
docs/
  decisions/
  exceptions/
  research/
```

Tool adapters generated or mirrored from canonical source:

```text
.cursor/rules/*.mdc
.github/copilot-instructions.md
.github/instructions/*.instructions.md
CLAUDE.md
GEMINI.md
```

Core doctrine:

- Root `AGENTS.md`: short, canonical, stable. Include stack, ownership, hard rules, one-command setup, fast validation, security validation, and repair protocol.
- Nested `AGENTS.md`: only where local rules change. Use for `apps/web`, `apps/api`, `crates/domain`, `crates/application`, `crates/adapters`, `db`, and `python/ai-service`.
- Generated zones: mark generated files and forbid manual edits. Every generated client/schema must point back to its source contract and regeneration command.
- Test map: map path patterns to required checks. Agents should not guess test scope.
- Owner map: map path patterns to domain owner, allowed dependencies, and forbidden dependencies.
- Proof lanes: `fast`, `contract`, `security`, `e2e`, `db`, and `full`.
- Audit policy: encode file-size caps, duplication caps, fallback bans, direct DB rules, Python containment, docs requirements, naming rules, and exception schema.

## Source Matrix

| Area | Best Sources | Finding | jankurai Doctrine |
| --- | --- | --- | --- |
| Neutral agent instructions | https://agents.md/, https://github.com/openai/codex/blob/main/docs/agents_md.md, https://github.com/openai/codex | `AGENTS.md` is plain Markdown, supports nested guidance, and is now widely recognized by coding-agent tools. | Use `AGENTS.md` as canonical repo instruction file. Generate tool-specific adapters from it. |
| Codex | https://github.com/openai/codex, https://github.com/openai/codex/blob/main/docs/agents_md.md, https://openai.com/index/introducing-codex/ | Codex CLI is local and repo-oriented. Codex docs route AGENTS.md behavior through official developer documentation. | Root and nested `AGENTS.md` must list setup, validation, style, PR, and safety rules. |
| Cursor | https://docs.cursor.com/en/context | Cursor Project Rules live in `.cursor/rules`, use metadata such as `description`, `globs`, and `alwaysApply`, and support scoped persistent context. | Generate `.cursor/rules/*.mdc` from canonical owner/test maps. Use `alwaysApply` sparingly for non-negotiable rules. |
| Claude Code | https://code.claude.com/docs/en/memory | Claude loads project instructions from `CLAUDE.md` or `.claude/CLAUDE.md`, supports hierarchy, and recommends project architecture, commands, standards, and workflows. | Generate `CLAUDE.md` as a concise adapter. Keep canonical details in linked docs. |
| Gemini CLI | https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/cli-reference.md, https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/commands.md | Gemini CLI uses `GEMINI.md` hierarchical memory and exposes `/memory` commands to list, refresh, and show loaded context. | Generate `GEMINI.md` from canonical rules. Include memory-refresh/check commands in validation docs. |
| Jules / Google agents | https://jules.google/docs/, https://jules.google/docs/changelog/2025-06-20/ | Jules integrates with GitHub, accepts setup scripts, reads `AGENTS.md`, and emphasizes plans and test habits. | Root `AGENTS.md` must be enough for cloud async agents to plan, set up, test, and open PRs. |
| GitHub Copilot | https://docs.github.com/en/copilot/concepts/prompting/response-customization | Copilot supports repo-wide `.github/copilot-instructions.md`, path-specific `.github/instructions/*.instructions.md`, and agent instructions such as `AGENTS.md`, `CLAUDE.md`, and `GEMINI.md`. Copilot review reads only the first 4,000 chars of custom instruction files. | Keep repo-wide Copilot file short. Put path rules in path-specific files. Do not rely on long instructions for review. |
| SWE-agent | https://arxiv.org/abs/2405.15793, https://github.com/SWE-agent/mini-swe-agent, https://mini-swe-agent.com/latest/ | SWE-agent argues that agents need purpose-built computer interfaces, not just human shells. mini-SWE-agent shows small, inspectable agents can perform well. | Design repo interface for agents: direct commands, concise maps, deterministic tests, and repair packets. |
| SWE-bench family | https://arxiv.org/abs/2310.06770, https://github.com/SWE-bench/SWE-bench, https://swebench.com/ | Real GitHub issue resolution remains hard and environment-dependent. | Every repo needs reproducible setup, tests, and issue-to-validation routing. |
| SetupBench | https://arxiv.org/abs/2507.09063 | Agents struggle with environment bootstrap, dependency conflicts, DB config, and background services. | One-command setup and one-command validation are hard audit requirements. |
| ContextBench | https://arxiv.org/abs/2602.05892, https://github.com/EuniAI/ContextBench | Agents over-retrieve context and fail to convert exploration into useful context. | Use path-scoped docs, owner maps, generated indexes, filtered command output, and small instruction files. |
| SWE-Effi | https://arxiv.org/abs/2509.09853, https://openreview.net/forum?id=x7C9A4Y9cF | Agent quality must be measured under resource constraints, not only pass/fail. | jankurai score should track token, time, and validation-radius economy. |
| OpenHands | https://arxiv.org/abs/2407.16741, https://docs.openhands.dev/, https://github.com/All-Hands-AI/OpenHands | OpenHands models agents as developers that write code, use shells, and browse. | Repo must expose safe tools, setup, tests, browser QA, and command boundaries. |
| Aider | https://github.com/Aider-AI/aider | Aider uses a repo map to help LLMs work in larger projects and integrates with Git. | jankurai should require repo maps and changed-file scoped repair queues. |
| Cline / Roo Code / Goose | https://github.com/cline/cline, https://github.com/RooCodeInc/Roo-Code, https://github.com/block/goose | Open-source agents emphasize file search, AST/context inspection, terminal commands, MCP/tool use, modes, and human review. | Standardize rules and validation so any agent can operate safely without bespoke prompting. |
| OpenTelemetry | https://opentelemetry.io/docs/, https://opentelemetry.io/docs/specs/otel/semantic-conventions/ | OTel provides vendor-neutral traces, metrics, logs, and semantic exception attributes. | Every service must emit request IDs, trace IDs, error type, and repair-relevant context. |
| Secret scanning | https://docs.github.com/en/code-security/secret-scanning/working-with-secret-scanning-and-push-protection | GitHub secret scanning/push protection blocks exposed credentials and has MCP-specific push-protection docs. | CI and pre-push should scan secrets. Agents must never add example real keys or bypass secret gates. |
| Supply chain | https://github.com/ossf/scorecard, https://github.com/ossf/scorecard-action, https://google.github.io/osv-scanner/, https://slsa.dev/ | OpenSSF, OSV, and SLSA provide automated security health, vulnerability scanning, and provenance language. | jankurai audit must require SCA, lockfiles, provenance, and dependency rationale for high-risk repos. |
| Playwright QA | https://playwright.dev/docs/best-practices, https://playwright.dev/docs/writing-tests, https://playwright.dev/docs/codegen | Official guidance: test user-visible behavior, isolate tests, use locators, web-first assertions, and cross-browser projects. | Use Playwright for product-surface QA. Forbid hard sleeps and brittle CSS/XPath selectors unless justified. |
| TypeScript/React/Vite | https://www.typescriptlang.org/tsconfig/strict.html, https://react.dev/learn/typescript, https://vite.dev/guide/build | Strict TypeScript improves correctness guarantees; React docs guide TS usage; Vite gives deterministic build command. | `apps/web` must be strict TypeScript, generated clients only, Vite build/test scripts, no hand-rolled API types. |
| Rust core | https://doc.rust-lang.org/cargo/reference/workspaces.html, https://rust-lang.github.io/api-guidelines/, https://doc.rust-lang.org/stable/rust-by-example/error.html | Cargo workspaces and Rust API guidelines provide reviewable structure; Rust error handling is typed and explicit. | `crates/domain` stays pure; `application` orchestrates; `adapters` own I/O; `api` is transport edge. |
| PostgreSQL truth | https://www.postgresql.org/docs/current/ddl-constraints.html, https://www.postgresql.org/docs/current/ddl-rowsecurity.html | PostgreSQL constraints and RLS enforce durable truth and row access policy at the database layer. | App-only invariants are insufficient for durable truth. Migrations must encode constraints and policy. |
| Generated contracts | https://openapi-generator.tech/docs/usage, https://openapi-generator.tech/docs/generators/typescript-fetch, https://buf.build/docs/ | OpenAPI Generator and Buf support generated clients/code from contract sources. | Contract sources live in `contracts/`; generated code lives under declared generated zones and is diff-checked. |
| Agent-friendly errors | https://www.rfc-editor.org/rfc/rfc9457, https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Error/cause, https://opentelemetry.io/docs/specs/semconv/registry/attributes/exception/, https://docs.rs/anyhow/latest/anyhow/ | Standardized problem details, structured error causes, OTel exception attributes, and Rust context support machine-readable repair. | Errors must include stable code, purpose, reason, common fixes, doc URL, trace ID, and safe user message. |

## Agent Tooling Notes

### Codex and AGENTS.md

Useful sources:

- https://agents.md/
- https://github.com/openai/codex
- https://github.com/openai/codex/blob/main/docs/agents_md.md
- https://openai.com/index/introducing-codex/

Findings:

- `AGENTS.md` should complement README, not duplicate it.
- Nested `AGENTS.md` files are the right pattern for monorepos.
- Tests listed in `AGENTS.md` are more likely to be run and repaired.
- Closest-file precedence means local crate/app rules can override root defaults.

jankurai rule:

- Root `AGENTS.md` must fit in a small prompt budget and route to maps/docs.
- Local `AGENTS.md` files must be short and must not conflict with root hard rules.
- Every rule must be auditable: either enforced by CI, linked to a doc, or labeled as guidance.

### Cursor

Useful sources:

- https://docs.cursor.com/en/context
- Low-weight sentiment: https://www.reddit.com/r/cursor/

Findings:

- Project rules live in `.cursor/rules` and are version-controlled.
- Rule application can be `Always`, `Auto Attached`, `Agent Requested`, or `Manual`.
- Community sentiment repeatedly reports drift when rules and CI disagree.

jankurai rule:

- Generate Cursor rules from `agent/audit-policy.toml`, `owner-map.json`, and `test-map.json`.
- Keep always-on Cursor rules short: stack, no-bypass rules, validation commands, generated zones.
- Scope detailed rules by glob so agents do not carry all context all the time.

### Claude Code

Useful sources:

- https://code.claude.com/docs/en/memory

Findings:

- Project instructions can live at `CLAUDE.md` or `.claude/CLAUDE.md`.
- Claude loads hierarchy above cwd and local subdirectory instructions on demand.
- Official docs recommend architecture, commands, standards, naming, and workflows.

jankurai rule:

- Generate `CLAUDE.md` from canonical agent rules.
- Keep project instructions durable and team-owned; keep personal/local details out of repo.

### Gemini CLI and Jules

Useful sources:

- https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/cli-reference.md
- https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/commands.md
- https://jules.google/docs/
- https://jules.google/docs/changelog/2025-06-20/

Findings:

- Gemini CLI supports hierarchical `GEMINI.md` memory and `/memory` inspection commands.
- Jules reads `AGENTS.md`, can use setup scripts, and works from GitHub.

jankurai rule:

- Generate `GEMINI.md` from canonical rules.
- Root setup scripts must be cloud-agent safe: no local secrets, no machine-specific paths, deterministic service startup.

### GitHub Copilot

Useful sources:

- https://docs.github.com/en/copilot/concepts/prompting/response-customization

Findings:

- Copilot supports repo-wide, path-specific, organization, personal, and agent instructions.
- Copilot code review reads only the first 4,000 characters of custom instruction files.

jankurai rule:

- `.github/copilot-instructions.md` must be a compact summary.
- Detailed rules belong in `.github/instructions/*.instructions.md` with path scopes.
- CI must enforce all hard rules; Copilot instructions alone are not policy.

### Antigravity

Useful sources:

- https://antigravity.im/documentation
- Low-weight sentiment: https://www.reddit.com/r/google_antigravity/

Findings:

- Public official documentation is available, but detailed rule-file behavior is less mature/less centralized than Codex/Cursor/Claude/Gemini/Copilot docs.
- Community sources mention workspace/global rules and `.agent/rules`, but this should remain low-weight until official docs stabilize.

jankurai rule:

- Treat Antigravity as an adapter target, not the canonical rule source.
- Keep canonical rules in `AGENTS.md` plus `agent/` maps.

## Optimal Agent-First Layout For Winner Stack

```text
repo/
  AGENTS.md
  agent/
    standard-version.toml
    owner-map.json
    test-map.json
    proof-lanes.toml
    generated-zones.toml
    audit-policy.toml
    agent/repo-score.json
  apps/
    web/
      AGENTS.md
      src/
      tests/
      package.json
      tsconfig.json
      vite.config.ts
      playwright.config.ts
    api/
      AGENTS.md
      src/
  crates/
    domain/
      AGENTS.md
    application/
      AGENTS.md
    adapters/
      AGENTS.md
    workers/
      AGENTS.md
  contracts/
    openapi/
    protobuf/
    json-schema/
    generated/
  db/
    migrations/
    constraints/
    seeds/
    AGENTS.md
  python/
    ai-service/
      AGENTS.md
  ops/
    ci/
    observability/
    security/
  docs/
    decisions/
    exceptions/
    research/
```

Ownership rules:

| Layer | Owns | Must Not Own | Agent Checks |
| --- | --- | --- | --- |
| `apps/web` | UI, routing, forms, local validation, generated API clients | secrets, direct DB writes, durable truth, hand-written API types | strict TS, generated-client imports, Playwright tests |
| `apps/api` | HTTP/RPC edge, request extraction, auth identity extraction, response mapping | domain rules, raw SQL business decisions | route tests, RFC 9457 errors, tracing |
| `crates/domain` | IDs, invariants, state machines, pure decisions | I/O, env reads, framework types, DB clients | no forbidden imports, property/unit tests |
| `crates/application` | commands, authz, idempotency, transactions | UI, transport details, scattered SQL | integration tests, transaction tests |
| `crates/adapters` | DB, queues, external APIs, filesystem, env | domain rules | SQLx checks, contract tests, fault tests |
| `crates/workers` | jobs, retries, queues, durable workflow glue | product truth not backed by DB | retry/idempotency tests, trace IDs |
| `contracts` | OpenAPI/protobuf/schema source | product logic | generated diff checks |
| `db` | migrations, constraints, indexes, RLS, seeds | app-only hidden invariants | migration tests, constraint tests |
| `python/ai-service` | rare approved advanced ML/data library work, embeddings, evals | product truth, authz, repo tools, proof lanes, general backend glue, direct prod DB writes | import/driver audit, API contract tests |
| `ops` | CI, OTel, SBOM, SCA, secret scanning, provenance | manual hidden release gates | CI audit, evidence artifacts |

## Testing And QA Best Practices

Primary sources:

- https://playwright.dev/docs/best-practices
- https://playwright.dev/docs/writing-tests
- https://playwright.dev/docs/codegen
- https://docs.cypress.io/app/core-concepts/best-practices

Rules:

- Use Playwright for browser QA on this stack unless a repo has a documented exception.
- Test user-visible behavior, not component implementation details.
- Use locators by role/text/test id, not brittle CSS/XPath.
- Use web-first assertions and auto-waiting. Ban `waitForTimeout` except with an approved test exception.
- Keep each test isolated. No test should depend on previous test state.
- Run browser projects for Chromium, Firefox, and WebKit when product risk warrants it.
- Store test data builders/fixtures in a dedicated test support package.
- Capture traces/screenshots/videos on failure in CI.
- Add contract tests for every generated API boundary.
- Add database constraint tests for every durable invariant.
- Add Rust domain property/unit tests for state machines and invariants.
- Add Python eval tests only for approved advanced-ML/data exceptions, and keep product truth tests outside Python.

Agent QA doctrine:

- Agents should add tests near the changed behavior.
- Agents should update test maps when introducing new path categories.
- Agents should not hide failures behind retries, sleeps, fallback branches, or broad mocks.
- CI should produce a machine-readable repair packet: failed command, owning path, likely layer, log excerpt, and suggested next check.

## Token Minimization And Context Economy

Public support:

- Copilot review reads only a bounded instruction prefix: https://docs.github.com/en/copilot/concepts/prompting/response-customization
- Claude recommends project instructions plus scoped rules/memory: https://code.claude.com/docs/en/memory
- Cursor supports scoped rules with globs: https://docs.cursor.com/en/context
- Gemini supports hierarchical memory inspection: https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/commands.md
- ContextBench studies context retrieval failures: https://arxiv.org/abs/2602.05892
- SWE-Effi studies resource-effective agent evaluation: https://arxiv.org/abs/2509.09853
- Aider uses repo maps: https://github.com/Aider-AI/aider

Rules:

- Keep root agent instructions short. Push detail into scoped docs.
- Prefer maps over prose: owner map, test map, generated-zone map, proof lanes.
- Prefer filtered command wrappers for large outputs. RTK-like tools are valid when they preserve security-relevant signal and expose raw-output escape hatches.
- Prefer file/path-targeted prompts over broad "inspect repo" prompts.
- Require audit output to be JSON plus Markdown: JSON for agents, Markdown for humans.
- Log raw evidence paths instead of pasting long output into instructions.
- Keep generated docs indexed by stable IDs.
- Ban token-heavy generated artifacts from default agent context unless requested.

## Agent-Friendly Exceptions

Primary sources:

- RFC 9457 problem details: https://www.rfc-editor.org/rfc/rfc9457
- OpenTelemetry exception attributes: https://opentelemetry.io/docs/specs/semconv/registry/attributes/exception/
- JavaScript `Error.cause`: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Error/cause
- Rust `anyhow::Context`: https://docs.rs/anyhow/latest/anyhow/
- Rust error overview: https://doc.rust-lang.org/stable/rust-by-example/error.html

Minimum schema:

```json
{
  "code": "HB_CONTRACT_DRIFT",
  "name": "ContractDrift",
  "purpose": "Reject manual API type drift before it reaches runtime",
  "reason": "Generated client is older than contracts/openapi/public.yaml",
  "common_fixes": [
    "run just generate-contracts",
    "commit generated diff",
    "update contract test snapshot"
  ],
  "doc_url": "docs/exceptions/HB_CONTRACT_DRIFT.md",
  "trace_id": "otel-trace-id",
  "safe_message": "API contract is out of date."
}
```

Language rules:

- Rust: domain/application errors use typed enums; boundary/application context can attach `anyhow::Context`; API maps to RFC 9457 JSON.
- TypeScript: custom errors must set stable `name`, `code`, and `cause`; UI displays safe message only.
- PostgreSQL: constraint names must be stable and documented so agents can map DB failures to fixes.
- Approved Python AI/data exception: custom exceptions must be typed, carry `code`, and map to RFC 9457 at service boundary.
- OpenTelemetry: log `exception.type`, `exception.message`, `exception.stacktrace`, plus stable `error.type` and trace ID.

## Audit Rubric Inputs From Research

Hard audit rules:

- Missing root `AGENTS.md` or equivalent canonical agent instructions.
- Divergent tool-specific instructions not generated from canonical rules.
- No one-command setup.
- No deterministic fast validation lane.
- No changed-path to test-map routing.
- No secret scanning.
- No dependency vulnerability scanning.
- No lockfiles for package managers in use.
- No generated contract drift check.
- Hand-written frontend API types when contracts exist.
- Direct DB access from web/UI.
- Product truth in Python.
- Python outside a dated advanced-ML/data exception under `python/ai-service`.
- Raw SQL in handlers instead of adapters/application-approved modules.
- Rust domain crate imports I/O/framework/env/DB crates.
- Missing DB migrations for schema changes.
- App-only invariants that should be DB constraints.
- Missing request/trace IDs in service boundaries.
- Errors without stable codes/docs/common fixes.
- `waitForTimeout` or hard sleeps in browser tests.
- Brittle selectors in browser tests.
- Broad catch/swallow/log-and-continue fallbacks.
- Duplicate implementations of same behavior across layers.
- Mega files above configured LOC caps.
- Mega functions above configured LOC/cyclomatic caps.
- Junk drawers: `utils`, `helpers`, `common`, `misc` without ownership docs.
- Generated files edited manually.
- Unbounded README/agent docs that flood context.
- TODO/FIXME without owner/date/issue.
- Test snapshots updated without source behavior explanation.
- CI job allowed to pass with ignored audit failures.

Known "vibe coding" problem classes to include:

- Prose-only architecture rules that CI does not enforce.
- "Just in case" fallback code that hides invalid states.
- Copy-pasted parallel implementations.
- Dead feature flags and unused compatibility layers.
- Silent exception swallowing.
- Hand-rolled protocol/client code instead of generated contracts.
- Broad `any`, `unknown as`, `unwrap`, `expect`, `panic`, or `except Exception` without documented boundary reason.
- Local-only scripts that fail in clean environments.
- Test suites requiring manual order or hidden credentials.
- State shared across tests.
- Mocking the system under test instead of testing the contract.
- Unclear ownership of files and directories.
- Mixed product truth between DB, frontend state, background jobs, and Python notebooks.
- Tool-specific prompt files that contradict each other.
- Docs that describe old commands or deleted paths.
- Agent instructions that tell agents to "be careful" instead of giving executable commands.

CI rule:

- jankurai audit should run on every PR.
- PRs can only merge with `score >= policy.minimum_score`, no hard-cap violations, and no high findings unless explicitly waived in `docs/exceptions/`.
- Audit output must include `agent_fix_queue` with path, owner, command, evidence, and smallest repair.

## Community Sentiment, Low Weight

Use only as directional signal:

- Reddit Cursor threads often report rules being ignored or drifting from CI: https://www.reddit.com/r/cursor/
- Reddit Claude Code threads focus on memory placement, instruction size, and auto-memory confusion: https://www.reddit.com/r/ClaudeCode/
- Reddit Codex threads focus on whether `AGENTS.md` was loaded and how to force reliable project guidance: https://www.reddit.com/r/codex/
- Reddit Playwright threads reinforce official guidance against hard waits and brittle assertions: https://www.reddit.com/r/Playwright/
- X/Twitter posts are useful for launch announcements and sentiment, but unstable for citation. Do not use X as evidence for the paper unless mirrored by official docs/blogs.

## Suggested BibTeX Keys

The sidecar file is `paper/references-agent-first.bib`.

Key groups:

- `agentsMdSpec2026`, `openaiCodexGithub2026`, `openaiCodexAgentsMdDocs2026`
- `cursorRulesDocs2026`, `claudeCodeMemoryDocs2026`, `geminiCliReference2026`, `geminiCliCommands2026`, `julesDocs2026`, `githubCopilotInstructions2026`, `antigravityDocs2026`
- `sweAgentAci2024`, `sweBench2024`, `setupBench2025`, `contextBench2026`, `sweEffi2025`
- `openhands2024`, `openhandsSdk2025`, `aiderGithub2026`, `clineGithub2026`, `rooCodeGithub2026`, `gooseGithub2026`
- `playwrightBestPractices2026`, `githubSecretScanning2026`, `openssfScorecard2026`, `osvScanner2026`, `slsaSpec2026`
- `rfc9457ProblemDetails2023`, `otelSemanticConventions2026`, `typescriptStrictDocs2026`, `cargoWorkspaces2026`, `postgresConstraints2026`
