## Automatic Scorer: Scoring Repos Against The Standard

`crates/jankurai/` contains the reference implementation of the audit in Section 8. The scorer is not a benchmark harness and not style police. It is a fast local filter for the question that matters in the AI era: does this repo make wrong code easy to reject, localize, prove, audit, and repair, or does it hide drift until it becomes expensive?

The implementation is intentionally fast and dependency-light. It is a Rust binary, so it can scan an arbitrary checkout immediately and emit the same contract in a clean environment or a messy one without a Python bootstrap step.

The current auditor line is `0.3.0`. It is intentionally strict for one target stack only:

```text
Rust core + TypeScript/React/Vite + PostgreSQL truth
+ generated contracts + exception-only Python AI/data service
```

### Static score dimensions

The rubric is fixed at 100 points. Each dimension is scored from 0 to its weight, and the baseline score is the sum of the dimension scores before caps. The weights are not a preference list. They are the paper's judgment about where AI-era repos succeed or fail.

| Dimension | Weight | What the scorer looks for |
| --- | ---: | --- |
| Ownership and navigation surface | 14 | Root instructions, owner maps, CODEOWNERS, local docs, obvious package boundaries, and no orphaned zones |
| Contract and boundary integrity | 14 | Generated clients and stubs, schema sources, explicit seams, and no handwritten drift across layers |
| Proof lanes and test routing | 14 | One-command setup, fast validation lanes, test maps, CI scripts, and targeted proofs that match ownership |
| Security and supply-chain posture | 14 | Lockfiles, secret scanning, SBOM/SCA, dependency rationale, and a visible security lane |
| Code shape and semantic surface | 12 | Reasonable file and function size, no junk drawers, and no hidden I/O in pure core code |
| Data truth and workflow safety | 8 | Migrations, constraints, RLS where useful, and durable writes owned by the right layer |
| Observability and repair evidence | 8 | OpenTelemetry, tracing, structured logs, request IDs, and failure evidence that helps repair |
| Context economy and agent instructions | 8 | Concise root instructions, local docs, generated-zone manifests, and readable routing for agents |
| Python containment and polyglot hygiene | 4 | Python appears only in rare advanced-ML/data exceptions, with no product truth or direct production DB ownership |
| Build speed signals | 4 | Fast compile and test loops, incremental-friendly tooling, and no unnecessary rebuild drag |

### Hard caps

Caps are ceilings, not extra penalties. If a repo misses one of these conditions, the final score cannot exceed the listed maximum. If several caps apply, the lowest ceiling wins.

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
| Non-optimal product language found | 74 |
| Too much Python in product surface | 72 |
| Vibe placeholders in product code | 68 |
| Fallback soup in product code | 70 |
| Severe duplication in product code | 70 |
| Generated-zone mutation risk | 76 |
| Direct DB access from wrong layer | 66 |
| Missing web E2E lane | 82 |
| Missing Rust property or integration tests | 82 |
| No agent-friendly exception pattern | 76 |
| Missing agent-readable docs | 80 |

The cap logic is meant to be monotone. Once a repo fails a structural requirement, no amount of polish elsewhere should let it score as if the requirement existed.

### Findings schema

The scorer should return one machine-readable object and one human-readable summary. The JSON is the contract. The Markdown is the review surface.

Required top-level fields:

- `score`: final integer score from 0 to 100 after caps
- `raw_score`: weighted score before caps
- `standard_version`: jankurai standard version used by the auditor
- `target_stack`: the exact stack the audit is judging
- `caps_applied`: list of cap identifiers or names that lowered the ceiling
- `dimensions`: list of per-dimension results
- `findings`: list of concrete issues
- `agent_fix_queue`: ordered list of next actions for an agent

Each `dimensions` entry should include at least:

- `name`
- `weight`
- `score`
- `evidence` or a similar short note field when the scorer needs to explain the result

Each `findings` entry should include at least:

- `severity`: `low`, `medium`, `high`, or `critical`
- `category`: `ownership`, `boundary`, `contracts`, `proof`, `security`, `shape`, `data`, `observability`, `instructions`, or `python`
- `path`: repo-relative file, directory, or manifest
- `problem`: short diagnosis
- `agent_fix`: the next change in imperative form
- `evidence`: concrete scan hits, not generic prose

Example shape:

```json
{
  "standard": "jankurai",
  "standard_version": "0.3.0",
  "target_stack": "Rust core + TypeScript/React/Vite + PostgreSQL + generated contracts + exception-only Python AI/data service",
  "score": 86,
  "raw_score": 91,
  "caps_applied": ["no-security-lane"],
  "dimensions": [
    {
      "name": "ownership_and_navigation_surface",
      "weight": 14,
      "score": 11,
      "evidence": ["AGENTS.md present", "owner map missing for apps/web"]
    }
  ],
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
  "agent_fix_queue": [
    "generate client from contracts/openapi",
    "add security lane to CI"
  ]
}
```

### Scanner checks

The scanner should prefer direct evidence over inference. If it flags a repo, it should be able to name the path or pattern that triggered the flag.

| Category | Signals |
| --- | --- |
| Ownership | `AGENTS.md`, `CODEOWNERS`, `agent/owner-map.json`, crate or package boundaries |
| Contracts | OpenAPI, protobuf, JSON Schema, generated clients, SQL migrations, TypeScript strict mode |
| Proof | `justfile`, `Makefile`, `package.json` scripts, CI workflows, `test-map`, fast and security lanes |
| Security | secret scanners, SBOM, SCA, lockfiles, unsafe ledger, dependency rationale |
| Code shape | mega files, mega functions by regex, junk-drawer dirs, hidden I/O in core or domain code |
| Data | migrations, constraints, RLS, direct DB access from the wrong layer |
| Python containment | Python outside a dated advanced-ML/data exception, direct DB drivers, product API ownership |
| Observability | OpenTelemetry, tracing, structured logs, request IDs, proof receipts |
| Agent readiness | concise root instructions, local docs, generated zones, raw evidence paths |
| Vibe-coding blocks | TODO stubs, fallback soup, duplicate blocks, weak names, hand-written DTOs, broad catches |
| Versioning | pinned standard version, audit version, output schema, update policy |

### Command contract

```bash
cargo run -p jankurai -- /path/to/repo --json agent/repo-score.json --md agent/repo-score.md
cargo run -p jankurai -- /path/to/repo --changed src/foo.rs contracts/api.yaml
```

The first command scores the whole repository and writes both output files. The second narrows the scan to changed paths when a diff-sensitive pass is enough.

The contract is simple on purpose. If a repo needs extra packages, extra setup, or extra explanation before it can be scored, it is already too heavy for the feedback loop this paper is arguing for.

### Changed-Path Mode

Changed-path mode does not excuse repo-level checks. It narrows file-scoped inspection while still checking root evidence such as instructions, CI, security lanes, and test maps.

```bash
cargo run -p jankurai -- . --changed apps/web/src/foo.ts contracts/openapi/public.yaml
```

Agents should use changed mode for local repair loops and full mode before merge.

### Repair Queue

The ordered `agent_fix_queue` is the most important output. It should be small enough for the next agent to act on immediately:

| Field | Meaning |
| --- | --- |
| `priority` | severity used for ordering |
| `path` | concrete path or manifest to edit |
| `task` | imperative repair instruction |
| `why` | short reason tied to the finding |

The audit is successful only when it creates useful work. A vague finding such as "improve architecture" is not acceptable. A jankurai finding should say where to go, what to change, which rule was broken, and why that repair makes future agent work safer.
