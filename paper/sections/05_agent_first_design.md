## Agent-First Code Design: What the Public Evidence Converges On

The public record around coding agents is noisy, but it is not empty. Official tool docs, open-source agent projects, benchmark papers, and community failure reports converge on the same practical lesson: agents do better when the repository behaves like an interface, not a warehouse.

The research sidecar for this paper, `docs/research/agent_first_sources.md`, reviews official docs for Codex, Cursor, Claude Code, Gemini CLI, GitHub Copilot, Jules, Playwright, OpenTelemetry, OpenSSF tooling, generated contracts, and benchmark projects such as SWE-agent, SWE-bench, SetupBench, ContextBench, SWE-Effi, OpenHands, Aider, Cline, Roo Code, and Goose. Reddit and X are treated only as low-weight sentiment because posts are hard to stabilize, but they are still useful as failure smoke: people repeatedly report instruction drift, memory confusion, setup failures, context bloat, brittle tests, and agents ignoring rules that CI does not enforce.

The usable pattern is strong enough to standardize.

### The Repo Is The Agent Interface

Human developers can survive a repo that relies on hallway memory. Agents cannot. A repo that wants agent work must expose a small set of machine-readable surfaces:

| Surface | Job | jankurai rule |
| --- | --- | --- |
| `AGENTS.md` | root routing and non-negotiable rules | short, canonical, loaded by agent tools |
| local agent files | path-specific deviations | scoped, never contradicting root hard rules |
| `agent/owner-map.json` | path to owner and allowed dependencies | agents must route edits before touching code |
| `agent/test-map.json` | path to proof lane | agents must run or report the mapped lane |
| `agent/generated-zones.toml` | generated output manifest | generated files are read-only unless source changes |
| `agent/proof-lanes.toml` | canonical validation commands | no hidden local-only rituals |
| `docs/exceptions/` | approved standard violations | owner, reason, expiration, repair plan |
| `agent/repo-score.json` | audit contract | CI and agents consume it |

Official Codex docs describe hierarchical `AGENTS.md` discovery, root-to-local merge order, and a default project-doc size cap. Claude Code docs describe project `CLAUDE.md`, local memory, and the risk of vague or conflicting instructions. GitHub Copilot supports repository, path-specific, and agent instruction files. Cursor rules support scoped project rules. These are tool-specific surfaces, but they all reward the same repo design: small root instructions, local detail, no contradiction, and executable commands.

### The Setup Problem Is A Correctness Problem

SetupBench and the SWE-bench family matter because they show that environment bootstrap is not clerical. If an agent cannot install, build, seed, and test the repo deterministically, it will start patching symptoms. One-command setup and one-command validation are not niceties. They are correctness gates.

An agent-native repo needs:

- clean-environment setup that does not depend on personal shell state
- deterministic `fast` validation under a clear target time
- contract and generated-drift checks
- database migration checks
- browser smoke tests for user-critical flows
- security checks for secrets, dependencies, workflows, and provenance
- audit output with an ordered repair queue

Any lane that requires a person to remember flags, credentials, service order, or tribal test subsets is not agent-ready.

### Context Economy Is Architecture

ContextBench and SWE-Effi both point at the cost side of agent work: pass/fail is not enough if a system wastes tokens, wall time, and validation radius. Public tool docs say the same thing indirectly. Copilot code review uses bounded instruction context. Codex has project-doc size limits. Claude recommends shorter files for better adherence and on-demand topic files for detail. Cursor and Copilot both support path-scoped rules.

The standard therefore treats token economy as architecture:

| Bad pattern | Agent-native replacement |
| --- | --- |
| enormous root instructions | root router plus scoped local docs |
| prose-only architecture | owner/test/generated maps |
| repeated command output in docs | stable command plus artifact path |
| agents reading whole repo by default | path ownership and symbol maps |
| generated artifacts in default context | generated-zone manifest and source pointer |
| "be careful" rules | executable checks and hard caps |

RTK-style output filtering belongs here. It is not magic; it is a practical implementation of a general rule: trim repeated terminal noise while preserving the raw-output escape hatch for security, debugging, and audit work.

### Open-Source Agent Lessons

Open-source agent projects are useful because they expose what agents actually need to operate. SWE-agent emphasizes the agent-computer interface. OpenHands models a coding agent as a developer that uses shell, browser, and file tools. Aider uses repository maps. Cline, Roo Code, and Goose emphasize tool use, modes, file search, and user review.

The lesson for repository design is not "pick one agent." It is "make any competent agent safer":

- deterministic commands instead of prose wishes
- repo maps instead of global search
- local rules instead of giant prompts
- generated contracts instead of handwritten type mirrors
- structured failures instead of ambiguous logs
- CI gates instead of tool-specific trust
- repair packets instead of vague red builds

The agent is replaceable. The repo interface should be stable.

### Optimal Layout For The Winner Stack

The ideal layout is the one already used in Section 4, with one addition: every major cell should have local agent guidance.

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
  apps/
    web/AGENTS.md
    api/AGENTS.md
  crates/
    domain/AGENTS.md
    application/AGENTS.md
    adapters/AGENTS.md
    workers/AGENTS.md
  contracts/AGENTS.md
  db/AGENTS.md
  python/ai-service/AGENTS.md
  ops/AGENTS.md
  docs/
    decisions/
    exceptions/
    repair/
```

The local files should not repeat the whole standard. They should answer only three questions: what does this cell own, what is forbidden here, and which lane proves changes here?

### Generally Accepted Rules, Not Prompt Folklore

The paper intentionally excludes tricks that depend on a single user's preference. Compressed writing styles can help in some sessions, and highly terse "caveman" notes may be useful for local memory compression, but that is not a universal engineering standard. jankurai only standardizes practices that have broad support across tool docs, benchmarks, or mature engineering practice:

- short and scoped agent instructions
- one-command setup and validation
- generated contracts
- typed boundaries
- small files and functions
- deterministic test routing
- structured observability
- secret and dependency scanning
- versioned exceptions
- machine-readable audit output

The goal is not to make prompts clever. The goal is to make the repository difficult to misunderstand.
