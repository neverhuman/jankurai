## The jankurai Audit Rubric And CI Contract

The paper becomes operational through the audit. A standard without a scanner is a philosophy. A scanner without an agent repair queue is a complaint generator. jankurai needs both: strict rules and actionable output.

### Audit Dimensions

| Dimension | Weight | Good evidence |
| --- | ---: | --- |
| Ownership and navigation surface | 14 | root `AGENTS.md`, owner map, test map, local docs |
| Contract and boundary integrity | 14 | generated clients, schema source, API drift checks |
| Proof lanes and test routing | 14 | fast lane, CI lane, path-to-test routing |
| Security and supply-chain posture | 14 | secret scan, dependency scan, SBOM/provenance, lockfiles |
| Code shape and semantic surface | 12 | small files/functions, low duplication, no placeholders |
| Data truth and workflow safety | 8 | migrations, constraints, DB isolated to adapters/db |
| Observability and repair evidence | 8 | OTel, tracing, request IDs, structured errors |
| Context economy and agent instructions | 8 | concise docs, generated zones, repair links |
| Python containment and polyglot hygiene | 4 | Python appears only in rare advanced-ML/data exceptions or explicit detector fixtures |
| Build speed signals | 4 | incremental-friendly fast commands |

### Hard Caps

| Missing capability or hard violation | Max score |
| --- | ---: |
| No root agent/developer instructions | 75 |
| No one-command setup or validation | 70 |
| No deterministic fast lane | 65 |
| High-risk repo with no security lane | 60 |
| Generated contracts or public API drift untested | 80 |
| Python has direct product truth or production DB ownership | 72 |
| No secret or dependency scanning in CI | 78 |
| No jankurai audit lane in CI | 82 |
| Non-optimal product language owns runtime surface | 74 |
| Too much Python in product surface | 72 |
| Vibe placeholders in product code | 68 |
| Fallback soup in product code | 70 |
| Severe duplication in product code | 70 |
| Generated-zone mutation risk | 76 |
| Direct DB access from wrong layer | 66 |
| Missing web E2E lane where web exists | 82 |
| Missing Rust property or integration tests where Rust core exists | 82 |
| No agent-friendly exception pattern | 76 |
| Missing agent-readable docs | 80 |

Caps matter because missing structure cannot be offset by polish. A repo with no fast lane is not an 89 with a footnote. It is capped because agents cannot prove work cheaply.

### Known Vibe-Coding Insults To Reject

These are not style nits. They are known patterns that make agent repair slower, broader, and less trustworthy.

| Insult | Why it fails | Required repair |
| --- | --- | --- |
| duplicated business logic | agents patch one copy and miss another | extract one owner and test it |
| fallback soup | bad states get hidden as "best effort" | model explicit states and retry policy |
| TODO/FIXME/HACK/XXX without owner | ambiguity ships | implement, delete, or create typed exception |
| stubs/placeholders/not implemented | fake completeness blocks proof | real behavior or unsupported-state error |
| `unreachable!`/`unimplemented!`/panic TODOs | runtime trap hidden from tests | typed error and proof |
| handwritten DTOs | API drift becomes invisible | generate from contract source |
| handwritten fetch wrappers | every endpoint forks the contract | generated client plus one transport wrapper |
| direct DB from UI/API/domain/exception-only Python | product truth leaks | isolate DB in adapters and PostgreSQL |
| Python product truth | dynamic runtime owns durable behavior | move to Rust/PostgreSQL |
| unnecessary runtime languages | more tooling and failure modes | converge to target stack |
| mega files | agents lose locality | split by owner before adding behavior |
| mega functions | behavior cannot be named or tested | extract named decisions |
| weak names | search and ownership fail | use domain vocabulary |
| junk drawers | global dumping ground | replace with owned modules |
| missing docs | agents infer policy from accidents | add scoped repair docs |
| missing audit CI | standard is decorative | run audit every PR |
| mutated generated zones | output becomes source | edit source and regenerate |
| no web E2E proof | humans become browser tests | add Playwright critical paths |
| no Rust property tests | invariants are example-only | add property/table tests |
| no security scan | generated churn leaks secrets/deps | add secret/dependency/provenance gates |
| opaque exceptions | failures do not teach repair | standardize error schema |
| console debugging | production evidence is unstructured | use tracing and request IDs |
| broad catch/swallow/log-and-continue | bad states disappear | typed error or bounded retry |
| dead flags and compatibility layers | agents preserve obsolete branches | delete or document exit criteria |
| brittle selectors and hard sleeps | tests become timing guesses | user-visible locators and web-first assertions |
| contradictory tool prompt files | agents choose randomly | generate adapters from canonical standard |
| prose-only architecture | no enforcement | maps, contracts, CI, audit |

### Output Contract

The audit must emit JSON and Markdown. JSON is for agents and CI. Markdown is for review.

```json
{
  "standard": "jankurai",
  "standard_version": "0.3.0",
  "target_stack": "Rust core + TypeScript/React/Vite + PostgreSQL + generated contracts + exception-only Python AI/data service",
  "score": 86,
  "raw_score": 91,
  "caps_applied": ["no-security-lane"],
  "dimensions": [],
  "findings": [
    {
      "severity": "high",
      "category": "boundary",
      "path": "apps/web/src/api.ts",
      "problem": "frontend appears to hand-maintain API types",
      "agent_fix": "generate client from contracts/openapi or protobuf source",
      "evidence": ["matched handwritten fetch wrapper", "no generated marker"]
    }
  ],
  "agent_fix_queue": []
}
```

Every finding needs path, evidence, and repair text. A score without repair guidance is not agent-native.

### CI Integration

Every adopting repo should run:

```bash
cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md
```

CI should upload both outputs and fail when policy thresholds are crossed. Adoption can be phased:

| Phase | CI behavior |
| --- | --- |
| observe | run audit, never fail |
| advisory | fail only malformed output or missing pin |
| guardrail | fail critical findings and hard caps |
| standard | fail high/critical and require score floor |
| repair loop | agent opens repair PRs from `agent_fix_queue` |
| release contract | audit, tests, security, contracts, and provenance all gate release |

### Release And Adoption Plan

jankurai should ship as four synchronized artifacts:

- paper edition: argument and evidence
- standard version: repo layout and rules
- audit version: scanner and output schema
- rule packs: Codex, Cursor, Claude, Copilot, Gemini, and others

The public adoption path should be pragmatic:

1. publish the paper PDF and Markdown
2. publish the standard and audit tool as a small dependency-free package
3. ship a GitHub Action and score badge
4. ship greenfield templates for the winning stack
5. ship migration templates for existing repos
6. ship agent-specific rule packs generated from the canonical standard
7. build benchmark tasks that compare baseline repos against jankurai-compliant repos
8. publish repair metrics: solve rate, wrong-owner edits, token use, validation radius, regression rate

The standard should be ambitious, but adoption should be incremental. Teams can start with observe-mode CI and move toward hard gates as findings become repairable.

### Future Research And Known Gaps

Known gaps:

- stack scores are synthesis, not controlled benchmarks
- X and Reddit sentiment are useful but unstable as evidence
- agent-tool behavior changes quickly
- optimal LOC caps need empirical refinement by language and domain
- exception schemas need real production trials
- audit heuristics can false-positive until backed by AST and dependency graph checks
- test explosion requires better quality metrics than line coverage
- Python containment policy needs careful exceptions for research-heavy companies

Future research should measure whether jankurai-compliant repos reduce wrong-owner edits, token use, time to first correct patch, flaky validation, and security regressions. The standard should change when evidence beats doctrine.
