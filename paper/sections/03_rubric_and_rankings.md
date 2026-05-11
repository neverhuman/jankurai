## The Ideal Agent-Native Stack Rubric

This rubric scores stacks for one purpose: building durable product systems in the agentic coding era. It is not a popularity contest, a benchmark table, or a hiring survey. It asks which stack makes wrong AI-generated code easiest to reject, localize, prove, audit, and repair.

The evidence pushes the weights toward verification and safety. DORA 2025 frames AI as an amplifier of organizational capability. CISA and NSA recommend memory-safe language direction as part of reducing vulnerability classes. Veracode's 2025 GenAI security work shows generated code still carries material security risk. GitGuardian's 2026 secrets-sprawl work shows AI-assisted development increases pressure on secret hygiene. GitHub Octoverse 2025 shows TypeScript rising in an AI-heavy ecosystem, and TypeScript 7/Vite 8 both push the typed frontend feedback loop faster. OpenTelemetry provides the vendor-neutral observability vocabulary a repairable system needs in production.

### The 100-Point Rubric

| Criterion | Weight | What earns full credit | What loses credit | Key references |
| --- | ---: | --- | --- | --- |
| Agent-verifiable correctness loop | 18 | Fast compile/type checks, deterministic changed-file test routing, generated contracts, property tests for invariants, reproducible local and CI lanes | Slow proof, vague test commands, runtime-only validation, flaky tests, handwritten API mirrors | DORA 2025, GitHub Octoverse 2025, TypeScript 7 beta |
| Security, memory safety, supply chain | 16 | Memory-safe core, secret scanning, SCA, SBOM, lockfiles, unsafe ledger, dependency rationale, CI security gate | Memory-unsafe core without isolation, secret leakage risk, unpinned deps, no dependency scanning | CISA/NSA memory-safe guidance, Veracode 2025, GitGuardian 2026 |
| Runtime performance, memory, cloud cost | 13 | Efficient core runtime, low memory overhead, predictable latency, cheap concurrency, bounded cold starts | Heavy runtime for hot paths, wasteful workers, slow startup, unbounded background work | Rust project docs, Go survey, .NET performance docs |
| Concurrency and resilience | 12 | Structured async, cancellation, backpressure, retries, idempotency, dead-letter handling, replayable workflows | Ad hoc goroutines/tasks, hidden retries, duplicate side effects, untested recovery | Go survey, Rust async ecosystem, OpenTelemetry |
| Boundary and contract integrity | 11 | OpenAPI/Protobuf/JSON Schema sources, generated clients, schema drift checks, DB migrations as truth | Handwritten DTO duplication, direct DB from UI/Python, public API changes without contract tests | OpenAPI, Protobuf, PostgreSQL docs |
| Simplicity and review surface | 10 | Small files, narrow modules, explicit ownership, boring dependency direction, no hidden magic | Mega-files, clever frameworks, mixed concerns, duplicated fallbacks, broad "utils" zones | Agent-tool docs, code review practice |
| Ecosystem, hiring, AI corpus strength | 8 | Large example corpus, mature packages, maintained tooling, common deployment patterns, strong model familiarity | Exotic stack, sparse examples, weak package maintenance, local-only conventions | GitHub Octoverse 2025, Go survey, Stack Overflow 2025 |
| Observability, auditability, provenance | 6 | OpenTelemetry traces/metrics/logs, structured events, request IDs, release provenance, repair receipts | Print logs, uncorrelated failures, no trace context, no change provenance | OpenTelemetry docs, DORA 2025 |
| Data integrity and durable workflow fit | 4 | PostgreSQL constraints, migrations, indexes, RLS where useful, transactional outbox/workflow safety | App-only invariants, schema drift, no migration checks, unbounded queues | PostgreSQL docs |
| Product velocity and interop | 2 | Fast UI loop, generated clients, easy local dev, standard browser ecosystem | Slow builds, bespoke UI tooling, awkward integration | TypeScript 7 beta, Vite 8 |

The weights are intentionally severe. Product velocity gets only two points because AI already makes first drafts cheap. Correctness, security, contracts, and repairability dominate because they decide whether the cheap draft is safe to merge.

### Scoring Logic

A stack earns points only when the property is structural. A team saying "we review carefully" does not earn security credit. A team saying "we document the API" does not earn contract credit. A team saying "senior engineers know where things go" does not earn ownership credit. The stack must make the right path easier than the wrong path:

- Compiler/type system catches invalid construction before runtime.
- Contract generator produces clients and stubs from one source.
- Database enforces truth that application code cannot silently forget.
- Test maps route the smallest credible proof lane for each change.
- Observability emits enough context for repair without a meeting.
- Agent instructions are short at root and precise near the files.
- Exceptions are named, documented, bounded, and visible to the audit.

This is why the rubric punishes "vibe coding" even when the product seems to work. Vibe coding optimizes for local momentum and explains architecture after the fact. Agent-native engineering optimizes for constrained generation and proof before trust.

### Top Five Stack Rankings

| Rank | Stack | ANSS | Best role | Main reason |
| ---: | --- | ---: | --- | --- |
| 1 | Rust core + TypeScript/React/Vite + PostgreSQL + generated contracts + exception-only Python | 94 | Best technical future stack | Strongest correctness/security loop with best product surface and durable truth |
| 2 | Go services + TypeScript/React/Vite + PostgreSQL | 90 | Best practical default for many companies | Simple, fast, easy to standardize, excellent concurrency, weaker invariant encoding than Rust |
| 3 | C#/.NET 10 + TypeScript/React/Vite + PostgreSQL | 89 | Best enterprise/regulatory stack | Mature platform, identity and enterprise tooling, strong operational story, more ceremony |
| 4 | TypeScript product plane + Rust/Go compute cells + PostgreSQL | 88 | Best product-velocity hybrid | Fastest product iteration, good escape hatches, high boundary-drift risk |
| 5 | Kotlin/Java 25 JVM + TypeScript/React/Vite + PostgreSQL | 87 | Necessary JVM/mobile modernization path | Strong ecosystem and concurrency options, but JVM runtime surface and legacy gravity keep it out of the standard |

```text
Agent-Native Stack Score (ANSS)

Rust + TS/Vite/React + PostgreSQL        94 | ##################################################
Go + TS/Vite/React + PostgreSQL          90 | ###############################################
C#/.NET + TS/Vite/React + PostgreSQL     89 | ###############################################
TS product plane + Rust/Go cells         88 | ##############################################
Kotlin/Java JVM + TS/Vite/React          87 | ##############################################
```

The scores are synthesis scores, not laboratory measurements. The small gap between ranks two through five matters less than the large strategic gap between "good local choice" and "standard worth specifying globally."

Kafka deserves a separate note because it is both strong and not the standard. It remains a serious event-streaming contender in brownfield systems: its semantics, ecosystem, and operational familiarity are real. But JVM-bound streaming infrastructure carries exactly the runtime and legacy surface this paper is trying to shrink. The jankurai position is therefore explicit: use Kafka when the system already needs it, treat it as necessary-evil infrastructure rather than stack identity, and expect the agent-native direction to move toward a Kafka-class, Rust-native replacement as AI compresses the cost of systems implementation.

### Why Rust Wins

Rust wins because agent-native engineering rewards compile-time distrust. Rust's ownership model, borrowing rules, enums, module privacy, trait boundaries, and exhaustive matching let teams encode decisions that other stacks leave to review discipline. That does not make Rust easy. It makes Rust honest. The compiler becomes a hard reviewer that does not care how confident the agent sounds.

Rust is not the whole stack because Rust should not own every concern. It is the core because core product truth, authorization-sensitive decisions, state machines, parsers, workflow rules, crypto-adjacent logic, concurrency-heavy work, and expensive compute should be in the place where invalid states are hardest to express. The UI should still be TypeScript. The database should still be PostgreSQL. Model and data work can use Python only when a rare advanced-ML/data exception is documented. Rust wins by owning the center, not by pretending to be the entire world.

### Why Go Comes Second

Go is the best practical default for many organizations because it is boring in the best way. It has fast builds, simple deployment, strong concurrency, a huge cloud ecosystem, and readable code that agents and humans can both navigate. The Go Developer Survey continues to show API services and CLIs as central Go use cases, which matches its strength as a services language.

Go loses to Rust because it encodes fewer invariants. It depends more on tests, review, and conventions for domain correctness. That can be an excellent trade in many companies. It is not the strongest universal answer when the goal is to minimize agent-authored wrongness at the core.

### Why .NET Comes Third

.NET is the enterprise answer. It has strong tooling, mature web frameworks, excellent identity integration, long-term platform stewardship, good performance work, and a familiar path for regulated organizations. In environments already committed to Microsoft infrastructure, .NET may beat Rust locally because organizational proof speed includes platform fit.

.NET ranks third globally because it usually carries more framework surface and more institutional ceremony than Rust or Go. It is powerful, but the universal standard should favor the narrowest core that maximizes correctness and auditability.

### Why the TypeScript Product Plane Is Not Enough

TypeScript is indispensable at the product surface. GitHub Octoverse 2025, TypeScript 7 beta, and Vite 8 all support the same direction: the typed frontend loop is fast, popular, and deeply represented in modern AI coding. The problem is that this success tempts teams to let TypeScript own too much.

TypeScript should own UI, forms, route state, client-side validation, generated API clients, and product interaction. It should not own durable truth, core authorization, workflow state, billing truth, or database writes. A TypeScript-heavy product plane can be excellent when backed by Rust or Go compute cells and PostgreSQL. It becomes fragile when the BFF turns into the real backend by accident.

### Why JVM Modernization Is Only a Necessary Evil

Kotlin/Java remains a serious answer because the JVM ecosystem is enormous, operationally mature, and deeply embedded in enterprise and mobile systems. Kotlin improves ergonomics, null-safety, and concurrency expression while preserving JVM reach. Java continues to modernize. The broader JVM data ecosystem remains strong where streaming and integration dominate.

The stack ranks fifth as a migration concession, not as a future standard. Agent-native work wants narrow, explicit ownership, low runtime surface, and fast proof. JVM organizations can absolutely build that, but they often start with more inherited framework and operational gravity than a greenfield Rust or Go core. The standard should help teams escape that gravity rather than bless it.

### Specialist Override: Elixir/Phoenix

| Specialist stack | General score | Realtime score | When it wins |
| --- | ---: | ---: | --- |
| Elixir/Phoenix + Rust workers + TypeScript islands | 83 | 94 | Collaboration, presence, realtime dashboards, notifications, multiplayer workflows, fault-tolerant coordination |

Elixir/Phoenix is the specialist overrule. On general product systems it should not displace the top five. In realtime coordination, it can beat them because the BEAM's process model, supervision culture, and Phoenix's realtime strengths align directly with the workload. Stack Overflow's 2025 survey admiration for Phoenix and Elixir's BEAM foundation support the specialist treatment.

The important point is scope. Elixir is not demoted because it is weak. It is constrained because the standard in this paper is universal product architecture. Specialist excellence stays specialist.

### Why We Only Care About the Winner Now

The ranking is useful only until it identifies the standard. After that, continuing to discuss every near-winner becomes a distraction. The paper is not trying to give every team emotional permission to keep its current stack. It is defining the target architecture for agent-native engineering.

That target is:

> Rust core + TypeScript/React/Vite product surface + PostgreSQL truth + generated contracts + exception-only Python AI/data service.

Everything after this point is winner-only because standards need sharp edges. The runner-up stacks remain valuable. Go is often the best pragmatic migration step. .NET is often the best enterprise path. Kotlin/Java is often the right modernization path. Elixir can win realtime. But the standard must specify one shape deeply enough that agents, audits, CI pipelines, and repair tooling can enforce it.
