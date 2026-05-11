# Release Plan: jankurai Standard

jankurai must ship as a paper, a standard, an auditor, and a set of agent-ready operating artifacts. The goal is not to win an argument on style. The goal is to make agent-native engineering easier to adopt than vibe-coded sprawl.

Current workspace policy is stricter than older bounded-Python planning notes:
agents must not add Python for repo tools, proof lanes, product services,
authorization, direct PostgreSQL writes, or backend glue. Python is allowed only
for rare dated advanced-ML/data exceptions under `python/ai-service`.

## Release Lines

| Release line | Artifact | Audience | Compatibility promise |
| --- | --- | --- | --- |
| Paper edition | `paper/jankurai.tex`, `paper/tex/`, PDF, Markdown companion | engineering leaders, researchers, senior developers | citations and argument may change by edition |
| Standard spec | repo layout, ownership rules, audit rubric | teams adopting jankurai | semantic versioning |
| Audit tool | `crates/jankurai/` | CI, agents, maintainers | semantic versioning plus output schema version |
| Agent artifacts | `AGENTS.md`, `CLAUDE.md`, Cursor rules, Copilot instructions, generated-zone manifests | coding agents and IDEs | versioned rule packs |
| CI integrations | GitHub Action, reusable workflow, pre-commit hook, local `just`/`make` targets | platform teams | backwards-compatible minor releases |
| Templates | greenfield repo template, migration template, exception catalog template | teams starting or converting repos | versioned with the standard |
| Benchmark suite | repair tasks, drift tasks, token-use tasks | researchers and skeptics | dataset versioning |

## Version Model

Use separate versions because the paper, rules, and tooling will move at different speeds.

| Version | Format | Example | Rule |
| --- | --- | --- | --- |
| Paper edition | date plus edition | `2026.05-ed8` | changes when the argument or evidence changes |
| Standard version | SemVer | `0.8.0` | breaking compliance rule means major bump |
| Audit version | SemVer | `0.8.0` | implementation release of the scanner |
| Output schema | SemVer | `1.5.0` | breaking JSON/Markdown contract means major bump |
| Rule pack version | SemVer plus tool | `codex-0.8.0` | tracks standard version with tool-specific packaging |

Every audit output should include:

```json
{
  "standard_version": "0.8.0",
  "auditor_version": "0.8.0",
  "schema_version": "1.5.0",
  "paper_edition": "2026.05-ed8",
  "target_stack_id": "rust-ts-vite-react-postgres-bounded-python",
  "target_stack": "rust-ts-vite-react-postgres-bounded-python"
}
```

Every adopted repo should pin:

```json
{
  "jankurai_standard": "0.8.0",
  "audit_min_version": "0.8.0",
  "audit_update_channel": "stable",
  "fail_on": ["critical", "high"],
  "advisory_on": ["medium", "low"]
}
```

Canonical local manifest: `agent/standard-version.toml`.

Required artifact bindings:

| Artifact | Manifest ID | Source / command |
| --- | --- | --- |
| `paper/jankurai.tex` | `paper-source` | `paper/tex/`, `just paper` |
| `paper/jankurai.pdf` | `paper-render` | source `paper/jankurai.tex`, command `just paper` |
| `paper/jankurai.md` | `paper-agent-md` | companion to TeX, not canonical |
| `docs/agent-native-standard.md` | `coding-standard` | version `standard_version` |
| `agent/JANKURAI_STANDARD.md` | `agent-standard-brief` | source `docs/agent-native-standard.md` |

Paper artifacts MUST use the `jankurai.*` prefix. `main.md`, `main.tex`, and `main.pdf` are forbidden anywhere in this repository.

## Receipt Convention

Operational receipts are volatile evidence, not source material.

- `jankurai doctor` and `jankurai init` write receipts under `target/jankurai/receipts/<action>-<unix-seconds>.json`
- release closeouts should cite the command, changed paths, and the receipt path
- the canonical score artifacts remain `agent/repo-score.json` and `agent/repo-score.md`
- `target/jankurai/` is the shared scratch root for audit, doctor, init, UX, and future proof outputs

Use those receipts to make phase handoffs and release evidence reproducible without promoting them into tracked source files.

## Standard Channels

| Channel | Purpose | Update policy |
| --- | --- | --- |
| `draft` | active experiments, new checks, research probes | may change weekly |
| `beta` | candidate rules for adopter feedback | no breaking changes without notice |
| `stable` | CI-safe compliance target | breaking changes only in major releases |
| `lts` | slow enterprise adoption | security fixes and clarifications only |

The audit should check for newer versions but should not surprise-break pinned CI. Version drift should produce an advisory finding first. Security-critical updates can raise severity, but still need explicit release notes and migration guidance.

## CI Adoption Strategy

jankurai should enter CI in phases.

| Phase | CI behavior | Goal |
| --- | --- | --- |
| 0. Observe | run audit, upload JSON/Markdown, never fail | collect baseline without blocking teams |
| 1. Advisory | fail only malformed auditor output or missing pin file | prove CI integration |
| 2. Guardrails | fail `critical` findings and hard caps below org threshold | stop dangerous drift |
| 3. Standard gate | fail `high` and `critical`; require score floor | make compliance real |
| 4. Repair loop | agent opens or updates repair PRs from `agent_fix_queue` | turn findings into action |
| 5. Release contract | block release unless audit, tests, security, contracts pass | make standard production policy |

Minimum CI lanes:

| Lane | Required command shape |
| --- | --- |
| `audit` | `cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md` |
| `fast` | one deterministic command for local agent edits |
| `contracts` | generated API/schema drift check |
| `security` | secret scan, dependency scan, SBOM/SCA where available |
| `db` | migration and schema drift checks |
| `ui` | component tests plus Playwright smoke for critical flows |
| `full` | integration and E2E validation |

The audit must always produce actionable repair output. A failed CI job without `agent_fix_queue` is a bad product.

## Audit Roadmap

### v0.1.0: Current Baseline

Scope:

- score dimensions and hard caps
- JSON and Markdown output
- root instructions detection
- proof lane detection
- security lane detection
- contract and generated-zone signals
- Python containment signals
- basic file/function size signals

Exit criteria:

- scores jankurai and `how_to_code_rust`
- emits stable top-level output
- runs without third-party dependencies

### v0.3.0: Vibe Coding Rules

Add hard-rule checks:

| Rule | Detection target |
| --- | --- |
| duplication | repeated functions, copied blocks, mirrored DTOs |
| fallbacks | `catch`, `except`, `unwrap_or`, `orElse`, default cascades without named reason |
| mega files | language-specific LOC limits |
| mega functions | rough function-body length and nesting depth |
| junk drawers | `utils`, `helpers`, `common`, `misc`, `shared` without ownership manifest |
| hidden I/O | env/network/filesystem/DB access in domain/core paths |
| direct DB access | UI/Python/worker bypasses application boundary |
| generated drift | generated files changed without source contract change |
| naming drift | inconsistent domain vocabulary and banned vague names |
| doc drift | missing or stale local `*.md` for ownership cells |

Exit criteria:

- each finding has path, evidence, and agent repair text
- CI mode supports score floor and severity floor
- changed-path mode checks only relevant rules where safe

### v0.3.0: Agent-Friendly Exceptions

Add checks and templates for:

- Rust error enum metadata
- TypeScript discriminated API errors
- PostgreSQL named constraint mapping
- Python boundary exception shape
- docs link validation
- exception catalog completeness

Exit criteria:

- scanner detects undocumented or string-only errors in owned layers
- generated exception catalog can route repairs to owners
- examples exist for Rust, TypeScript, SQL, and Python service boundary

### v0.7.0: Vibe Coverage Hardening

Patch hardening for the v0.6 coverage release:

- reviewed canonical groups and detector/evidence status for all 260 vibe-coding source rows
- `0` unmapped, duplicate, unreviewed, or unjustified `none` rows
- `detector-backed` coverage only when deterministic audit/report evidence exists
- semantic mapping fixtures and HLT-022 through HLT-027 detector fixtures
- generated paper table uses short rule labels in cells and a separate rule legend

Exit criteria:

- version bindings align at standard/auditor `0.7.0`, schema `1.5.0`, and paper `2026.05-ed5`
- `jankurai vibe validate` rejects missing rows, duplicate rows, title drift, unknown rule/tool/lane references, unreviewed rows, unjustified `none`, and unsupported coverage states

### v0.6.0: Trustworthy Merge Release

Ship the release surface as one product:

| Surface | Artifact |
| --- | --- |
| Audit exports | JSON, Markdown, SARIF, JUnit, GitHub summary, repair queue JSONL, issue export |
| Install | idempotent `init --profile --ide --mode --dry-run --yes --diff` |
| Doctor | stale score, root artifact, path leak, echo-only proof, UX artifact, boundary, and paper-source checks |
| CI | `jankurai ci install --github --mode ratchet --baseline agent/repo-score.json --min-score 85` |
| Merge witness | `jankurai witness . --changed-from origin/main --baseline agent/repo-score.json` |
| Boundaries | authoritative streaming and queue manifest with Kafka brownfield exception shape |
| UX QA | route-matrix and Storybook audit commands with artifact-backed proof |

Exit criteria:

- every below-floor audit output includes routed repair work
- canonical score artifacts remain under `agent/`
- `paper/tex/` is the canonical paper source and Markdown sections are marked legacy-only
- version bindings align at standard/auditor `0.6.0`, schema `1.4.0`, and paper `2026.05-ed5`

### v0.6.0: GitHub Action And Badges

Ship:

- `jankurai` GitHub Action
- reusable workflow
- PR comment summary
- score badge
- SARIF or code-scanning export if practical
- update-check advisory

Exit criteria:

- public repos can adopt with fewer than 10 lines of YAML
- failed PR shows exact findings and repair queue
- badge reflects pinned standard version

### v0.6.0: Streaming Evaluation Pack

Ship:

- `agent/boundaries.toml` schema and examples
- Kafka brownfield exception template
- Tansu Kafka-compatible evaluation harness
- Apache Iggy and Fluvio greenfield evaluation notes
- replay, consumer-group, retention, compaction, ACL, quota, observability, and migration proof checklist

Exit criteria:

- streaming clients outside adapters produce `HLT-019-STREAMING-RUNTIME-DRIFT`
- Kafka exceptions require owner, expiry, brownfield reason, and migration path
- benchmark fixtures distinguish Kafka-compatible replacement readiness from greenfield Rust-native alternatives

### v0.7.0: Benchmark Pack

Ship a public task suite:

- boundary drift tasks
- duplicate logic tasks
- Python containment tasks
- exception repair tasks
- generated contract drift tasks
- token-economy tasks
- test-routing tasks

Measure:

- solve rate
- wrong-owner edit count
- token use
- commands run
- time to first correct patch
- regression rate
- human review burden

Exit criteria:

- compare baseline repo vs jankurai-compliant repo
- publish methodology and raw evidence
- include multiple coding agents

### v1.2.0: Stable Compliance Standard

Requirements:

- stable score schema
- stable hard caps
- documented exception process
- CI integration
- migration playbook
- public templates
- initial benchmark evidence
- governance policy for rule changes

## Adoption Plan

The standard needs distribution, proof, and low-friction adoption.

### 1. Make The Argument Unavoidable

Publish the paper with a clear thesis:

> Codebases optimized for human comfort are misaligned with agent maintenance. The winning stack is the one that rejects wrong generated code fastest.

The title should be used consistently:

> Jankurai: A Versioned Repository Conformance Standard for Trustworthy AI-Assisted Merge

The public thesis line remains:

> No proof, no merge; no receipt, no trust.

The paper should include a ranking graph, a concrete winner architecture, and the audit contract. No vague manifesto without an enforcement path.

### 2. Make The Audit Useful In One Command

The first public experience should be:

```bash
cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md
```

No bootstrap. No service. No API key. No dependency install. Immediate findings.

### 3. Make Public Examples

Publish sample reports for:

| Repo type | Why |
| --- | --- |
| clean greenfield jankurai repo | shows target shape |
| typical React/Node app | shows contract and DB drift |
| Python-heavy product repo | shows containment risk |
| Rust service repo | shows domain/application/adapters split |
| monorepo with weak instructions | shows token and ownership failures |
| mature open-source repo | shows how established projects score differently |

Community reports should be framed as examples, not attacks. The value is repair, not public shaming.

### 4. Make Repair Agent-Ready

Every audit finding should become a task an agent can execute:

```text
category: boundary
path: apps/web/src/api.ts
problem: frontend hand-maintains API types
agent_fix: replace handwritten types with generated client from contracts/openapi
evidence: fetch wrapper and duplicated UserDto found
```

This is the core product. Scoring without repair is theater.

### 5. Ship Templates

Templates should include:

- root `AGENTS.md`
- `agent/owner-map.json`
- `agent/test-map.json`
- `agent/generated-zones.toml`
- `schemas/cell-manifest.schema.json`
- `schemas/cell-registry.schema.json`
- `agent/jankurai-standard.json`
- Rust workspace with `domain`, `application`, `adapters`, `workers`
- Vite/React app with generated client path
- PostgreSQL migrations and constraints folders
- exception-only `python/ai-service` for rare advanced-ML/data work
- CI workflows for audit, fast, contracts, security, db, ui, full
- docs skeleton for decisions and exceptions

The template should make the right path the easy path.

### 6. Create A Compliance Badge

Badge fields:

| Field | Example |
| --- | --- |
| score | `jankurai 86` |
| standard | `standard 0.3` |
| channel | `stable` |
| caps | `0 caps` |
| generated | `contracts checked` |
| python | `contained` |

The badge should link to the latest audit Markdown and JSON artifact.

### 7. Build Tool-Specific Rule Packs

Agent tools disagree on instruction loading. jankurai should not depend on one vendor.

Rule packs should translate one standard into each tool's native shape:

- Codex: `AGENTS.md` and scoped overrides.
- Claude: `CLAUDE.md`, `.claude/rules`, memory hygiene.
- Cursor: `.cursor/rules`.
- Copilot: `.github/copilot-instructions.md` and path-specific instructions.
- Antigravity-style tools: artifact and approval checklists.
- Aider: repo-map and token-budget guidance.

All packs must route to the same owner map, test map, generated zones, and audit command.

### 8. Publish Benchmarks

Claims need numbers. Use benchmark suites that test:

- repair speed
- token use
- test selection accuracy
- boundary drift prevention
- generated contract repair
- exception-guided repair
- duplicate logic elimination
- Python containment

Publish raw artifacts. Do not trust summary charts alone.

### 9. Partner With Open Source Maintainers

Offer low-friction PRs:

- add audit in advisory mode
- add root instructions
- add test map
- add generated-zone manifest
- add exception catalog template
- add CI score artifact

Do not force stack migration into unrelated projects. Use non-winner stacks as research controls, not compliance targets.

### 10. Establish Governance

jankurai needs a standards board before v1.0:

| Role | Responsibility |
| --- | --- |
| standard editor | owns rule text and release notes |
| auditor maintainer | owns scanner behavior and output schema |
| evidence maintainer | refreshes sources and research claims |
| template maintainer | keeps starter repos current |
| benchmark maintainer | owns tasks and raw results |
| security reviewer | reviews rules that affect CI gates and secrets |

Rule changes require:

- rationale
- evidence tier
- migration impact
- examples
- auditor behavior
- release note
- deprecation path if breaking

## CI Policy Defaults

Recommended defaults by repo maturity:

| Repo maturity | Score floor | Fail on | Notes |
| --- | ---: | --- | --- |
| new template | 85 | high, critical | strict from day one |
| migrating product repo | 70 | critical | raise floor by 5 points per milestone |
| legacy monolith | 60 | critical security and product-truth violations | start with advisory repair queue |
| regulated system | 85 | medium, high, critical | require exception approvals |
| open-source library | 75 | high, critical | focus on instructions, tests, supply chain |

Hard caps should remain hard. Teams may add documented exceptions, but an exception should not erase the cap. It should explain why the team accepts the ceiling and what must happen to remove it.

## File And Refactor Limits

Default limits for the optimal stack:

| Target | Soft limit | Hard limit | Required action |
| --- | ---: | ---: | --- |
| Rust domain file | 300 LOC | 500 LOC | split by invariant, state machine, or value object |
| Rust application file | 350 LOC | 600 LOC | split by command/query/use case |
| Rust adapter file | 400 LOC | 700 LOC | split by external system or table family |
| TypeScript component | 220 LOC | 350 LOC | split presentation, state, and generated client use |
| TypeScript module | 300 LOC | 500 LOC | split by route, feature, or contract |
| Exception-only Python AI/data file | 250 LOC | 400 LOC | split model call, transform, eval, and boundary schema |
| SQL migration | 250 LOC | 500 LOC | split into ordered migrations unless atomicity requires one |
| Markdown instruction file | 120 lines | 200 lines | move detail to path-scoped docs |
| Function/method | 60 LOC | 100 LOC | extract named policy, validator, command, or adapter call |

Refactor rule: split by ownership, not by arbitrary size. A refactor is good only if the next agent can route faster and prove behavior with a smaller test lane.

## Naming And Documentation Rules

Strict defaults:

- Names must use domain vocabulary from docs or contracts.
- Ban vague permanent names: `utils`, `helpers`, `misc`, `stuff`, `common`, `manager`, `processor`, `handler` without a domain noun.
- Generated files must contain generated markers and source references.
- Public APIs must have contract docs.
- Every ownership cell must have local README or equivalent instructions when behavior is not obvious from code.
- Exceptions must link to docs.
- TODOs must include owner, date, issue, and exit condition.
- CI workflows must name proof lanes consistently. The live `agent/proof-lanes.toml` currently defines the minimal set `fast`, `audit`, `paper`, `security`, and `full`; broader target lanes such as `contracts`, `db`, `ui`, and `observability` are roadmap vocabulary, not live config.

## Future Research Releases

Planned research outputs:

| Release | Study |
| --- | --- |
| `research-0.1` | baseline audit scores across public repos |
| `research-0.2` | before/after agent repair speed on template vs conventional repo |
| `research-0.3` | instruction-file size vs solve rate and token cost |
| `research-0.4` | agent-friendly exceptions vs plain exceptions |
| `research-0.5` | generated contracts and drift repair |
| `research-0.6` | Python containment and production-risk proxies |
| `research-1.0` | full jankurai compliance vs incident/rollback correlation |

## Known Release Risks

| Risk | Mitigation |
| --- | --- |
| standard becomes dogma without evidence | keep research gaps visible and publish raw benchmarks |
| audit produces noisy findings | require evidence paths and repair text for every rule |
| tool-specific rules drift | generate rule packs from one standard source |
| CI adoption feels punitive | start advisory, then raise gates |
| public repo audits look hostile | frame reports around repair and opt-in examples |
| file-size limits become cargo cult | allow documented exceptions with proof evidence |
| Python rule feels ideological | define clear rare advanced-ML/data exception process and keep default work in Rust/TypeScript/PostgreSQL |
| agent tooling changes quickly | release rule-pack updates separately from core standard |
| teams pin old versions forever | update check emits advisory with migration notes |

## Success Metrics

Track:

- number of repos running audit in CI
- number of repos pinning a jankurai standard version
- average score trend over time
- time from finding to repair PR
- accepted repair PR rate
- wrong-owner edit count in benchmark tasks
- generated contract drift caught before merge
- security findings caught before release
- token use per successful repair
- public templates cloned or used
- external contributions to rules and benchmarks

## Strong Adoption Position

jankurai should be marketed plainly:

> Stop optimizing repositories for humans staring at files. Optimize them for agents proving changes.

The world does not need another vibe coding manifesto. It needs a versioned standard, a CI gate, a repair queue, and templates that make disciplined agent work cheaper than chaos.

That is the release goal.
